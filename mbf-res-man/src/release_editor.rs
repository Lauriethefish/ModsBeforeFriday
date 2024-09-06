
use std::{collections::{HashMap, HashSet}, ffi::OsStr, fs::DirEntry, io::{Cursor, Read, Seek, SeekFrom}, path::Path};

use anyhow::{anyhow, Context, Result};
use byteorder::{ReadBytesExt, BE};
use log::{error, info, warn};

use crate::hash_cache::HashCache;

const API_ROOT: &str = "https://api.github.com";
const UPLOAD_API_ROOT: &str = "https://uploads.github.com";
const ASSET_CRC_FILENAME: &str = "assets.crc32.json";

#[derive(Clone, Debug)]
pub struct Repo {
    pub owner: String,
    pub repo: String,
}

#[derive(Clone, Debug)]
pub struct Release {
    pub repo: Repo,
    pub id: i64
}

#[derive(Clone, Debug)]
pub struct ReleaseAsset {
    pub id: i64,
    pub file_name: String,
    pub url: String,
    pub crc32: Option<u32>
}

fn set_headers(req: ureq::Request, auth_token: &str) -> ureq::Request {
    req
    .set("Accept", "application/vnd.github+json")
    .set("Authorization", &format!("Bearer {auth_token}"))
    .set("X-GitHub-Api-Version", "2022-11-28")
}

// Reads the details about an asset, other than its CRC32, from the github JSON response
fn read_asset_details(asset: &serde_json::Value) -> ReleaseAsset {
    ReleaseAsset {
        id: asset.get("id").unwrap().as_i64().unwrap(),
        file_name: asset.get("name").unwrap().as_str().unwrap().to_string(),
        url: asset.get("url").unwrap().as_str().unwrap().to_string(),
        crc32: None
    }
}

pub fn get_assets(release: &Release, auth_token: &str) -> Result<Vec<ReleaseAsset>> {
    let req_path = format!("{API_ROOT}/repos/{}/{}/releases/{}", release.repo.owner, release.repo.repo, release.id);

    let resp = set_headers(crate::default_agent::get_agent()
        .get(&req_path), auth_token).call()?;

    let document: serde_json::Value = serde_json::from_reader(resp.into_reader())?;
    let mut assets = Vec::new();
    for asset in document.get("assets").unwrap()
        .as_array().unwrap() {
        
        assets.push(read_asset_details(asset))
    }

    Ok(assets)
}

pub fn get_latest_release(repo: Repo, auth_token: &str) -> Result<Release> {
    let req_path = format!("{API_ROOT}/repos/{}/{}/releases/latest", repo.owner, repo.repo);

    let resp = set_headers(crate::default_agent::get_agent()
        .get(&req_path), auth_token).call()?;

    let document: serde_json::Value = serde_json::from_reader(resp.into_reader())?;
    let release_id = document.get("id").unwrap().as_i64().unwrap();

    Ok(Release {
        id: release_id,
        repo: repo
    })
}

pub fn delete_asset(asset: &ReleaseAsset, release: &Release, auth_token: &str) -> Result<()> {
    let req_path = format!("{API_ROOT}/repos/{}/{}/releases/assets/{}", release.repo.owner, release.repo.repo, asset.id);

    set_headers(crate::default_agent::get_agent()
        .delete(&req_path), auth_token).call()?;

    Ok(())
}

pub fn upload_asset_from_reader(release: &Release, file_name: &str, mut content: impl Read + Seek, auth_token: &str) -> Result<ReleaseAsset> {
    let req_path = format!("{UPLOAD_API_ROOT}/repos/{}/{}/releases/{}/assets", release.repo.owner, release.repo.repo, release.id);

    content.seek(SeekFrom::End(0))?;
    let length = content.stream_position()?;
    content.seek(SeekFrom::Start(0))?;

    let resp = set_headers(crate::default_agent::get_agent()
        .post(&req_path)
        .query("name", file_name), auth_token)
        .set("Content-Type", "application/octet-stream")
        .set("Content-Length", &length.to_string())
        .send(content)?;

    let document: serde_json::Value = serde_json::from_reader(resp.into_reader())?;
    Ok(read_asset_details(&document))
}

pub fn upload_or_overwrite(file_path: impl AsRef<Path>,
    assets: &[ReleaseAsset],
    crc32_map: &mut HashMap<String, u32>,
    release: &Release,
    auth_token: &str,
    hash_cache: &mut HashCache<u32>) -> Result<()> {
    let file_name = file_path.as_ref()
        .file_name().ok_or(anyhow!("Asset had no filename"))?
        .to_string_lossy()
        .to_string();

    let file_crc = hash_cache.get_file_hash(file_path.as_ref())?;

    // Only one asset may exist with each file-name
    match assets.iter().filter(|asset| asset.file_name == file_name).next() {
        Some(existing_asset) => {
            if crc32_map.get(&file_name).copied() == Some(file_crc) {
                info!("File up to date. (CRC {file_crc})");
                return Ok(());
            }   else    {
                info!("File not up to date (New CRC {file_crc}), deleting existing asset");
                delete_asset(existing_asset, release, auth_token)?;
                // Remove existing CRC32 before uploading new file to ensure the file is reuploaded in the case of a partial upload and internet failure.
                crc32_map.remove(&file_name);
            }
        },
        None => info!("File does not already exist.")
    };

    info!("Uploading file to the release");
    let mut file_handle = std::fs::File::open(file_path)?;
    file_handle.seek(SeekFrom::Start(0))?;
    upload_asset_from_reader(release, &file_name, &mut file_handle, auth_token)?;
    crc32_map.insert(file_name, file_crc);

    Ok(())
}

fn list_files_json_last(dir_path: impl AsRef<Path>) -> Result<Vec<DirEntry>> {
    let mut default_files = Vec::new();
    let mut json_files = Vec::new();

    for file_result in std::fs::read_dir(&dir_path)? {
        let file = file_result.context("Failed to read file details")?;

        if !file.file_type()?.is_file() {
            continue;
        }

        if file.path().extension() == Some(OsStr::new("json")) {
            json_files.push(file);
        }   else {
            default_files.push(file);
        }
    }

    default_files.append(&mut json_files);
    Ok(default_files)
}

/// Reads the JSON file containing the CRC-32 hashes of each existing file within the release.
pub fn read_existing_asset_crc32s(release_files: &[ReleaseAsset], auth_token: &str) -> Result<(Option<ReleaseAsset>, HashMap<String, u32>)> {
    match release_files.iter().filter(|asset| asset.file_name == ASSET_CRC_FILENAME).next() {
        Some(crc32_asset) => {
            let resp = set_headers(ureq::get(&crc32_asset.url), auth_token)
                .set("Accept", "application/octet-stream")
                .call()?;

            let json_str = resp.into_string()?;
            Ok((Some(crc32_asset.clone()), serde_json::from_str(&json_str)?))
        },
        None => Ok((None, HashMap::new()))
    }
}

pub fn write_asset_crc32s(release: &Release, asset_crc32s: &HashMap<String, u32>, auth_token: &str) -> Result<()> {
    let json_buf = serde_json::to_vec(&asset_crc32s)?;
    let mut json_reader = Cursor::new(json_buf);

    upload_asset_from_reader(release, ASSET_CRC_FILENAME, &mut json_reader, auth_token)?;
    Ok(())
}

pub fn update_release_from_directory(dir_path: impl AsRef<Path>,
    release: &Release,
    auth_token: &str,
    delete_nonexisting: bool,
    hash_cache: &mut HashCache<u32>) -> Result<()> {
    let release_files = get_assets(release, auth_token)?;
    info!("Getting CRC32 of files in release");
    let (crc32_asset, mut crc32_map) = read_existing_asset_crc32s(&release_files, auth_token)?;

    for file in list_files_json_last(&dir_path).context("Failed to list files")? {
        info!("Processing file {:?}", file.path());
        if file.path().file_name() == Some(OsStr::new(ASSET_CRC_FILENAME)) {
            warn!("A file was found in the assets folder with name {ASSET_CRC_FILENAME}, conflicting with the asset CRC-32 file");
            warn!("This file will be skipped");
            continue;
        }

        let path = file.path();
        let file_name = match path.file_name() {
            Some(name) => name,
            None => { error!("File had no file name"); continue } 
        };

        if let Err(err) = upload_or_overwrite(&path, &release_files, &mut crc32_map, release, auth_token, hash_cache) {
            error!("Failed to upload/overwrite {file_name:?}: {err}");
        }
    }

    for file in release_files {
        if delete_nonexisting && !dir_path.as_ref().join(&file.file_name).exists() {
            info!("Deleting {} from release", file.file_name);
            if let Err(err) = delete_asset(&file, release, auth_token) {
                error!("Failed to remove from release: {err}");
            }
            crc32_map.remove(&file.file_name);
        }

        if file.file_name.ends_with(".crc32") {
            warn!("Removing legacy CRC32 file {}", file.file_name);
            delete_asset(&file, release, auth_token)?;
        }
    }

    if let Some(existing_crc_asset) = crc32_asset {
        info!("Deleting existing digests file");
        delete_asset(&existing_crc_asset, release, auth_token)?;
    }
    info!("Uploading digests file to release");
    write_asset_crc32s(release, &crc32_map, auth_token).context("Failed to upload CRC32's file");
    Ok(())
}