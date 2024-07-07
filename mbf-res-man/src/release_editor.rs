
use std::{collections::HashSet, ffi::OsStr, fs::DirEntry, io::{Cursor, Read, Seek, SeekFrom}, path::Path};

use anyhow::{anyhow, Context, Result};
use byteorder::{ReadBytesExt, BE};
use log::{info, warn};

const API_ROOT: &str = "https://api.github.com";
const UPLOAD_API_ROOT: &str = "https://uploads.github.com";

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

    let resp = set_headers(crate::external_res::get_agent()
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

    let resp = set_headers(crate::external_res::get_agent()
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

    set_headers(crate::external_res::get_agent()
        .delete(&req_path), auth_token).call()?;

    Ok(())
}

pub fn upload_asset_from_reader(release: &Release, file_name: &str, mut content: impl Read + Seek, auth_token: &str) -> Result<ReleaseAsset> {
    let req_path = format!("{UPLOAD_API_ROOT}/repos/{}/{}/releases/{}/assets", release.repo.owner, release.repo.repo, release.id);

    content.seek(SeekFrom::End(0))?;
    let length = content.stream_position()?;
    content.seek(SeekFrom::Start(0))?;

    let resp = set_headers(crate::external_res::get_agent()
        .post(&req_path)
        .query("name", file_name), auth_token)
        .set("Content-Type", "application/octet-stream")
        .set("Content-Length", &length.to_string())
        .send(content)?;

    let document: serde_json::Value = serde_json::from_reader(resp.into_reader())?;
    Ok(read_asset_details(&document))
}

pub fn crc_of_stream(mut stream: impl Read) -> Result<u32> {
    let mut crc = crate::diff_builder::ZIP_CRC.digest();
    let mut buffer = vec![0u8; 4096];

    loop {
        let read_bytes = stream.read(&mut buffer)?;
        if read_bytes == 0 {
            break Ok(crc.finalize())
        }
        
        crc.update(&buffer[0..read_bytes])
    }
}

pub fn get_asset_crc32(assets: &[ReleaseAsset], file_name: &str, auth_token: &str) -> Result<Option<u32>> {
    let crc32_filename = format!("{file_name}.crc32");

    match assets.iter().filter(|asset| asset.file_name == crc32_filename).next() {
        Some(crc32_asset) => {
            let resp = set_headers(ureq::get(&crc32_asset.url), auth_token)
                .set("Accept", "application/octet-stream")
                .call()?;

            Ok(Some(resp.into_reader().read_u32::<BE>()?))
        },
        None => Ok(None)
    }
}

pub fn upload_or_overwrite(file_path: impl AsRef<Path>, assets: &[ReleaseAsset], release: &Release, auth_token: &str) -> Result<()> {
    let file_name = file_path.as_ref()
        .file_name().ok_or(anyhow!("Asset had no filename"))?
        .to_string_lossy()
        .to_string();

    let mut file_handle = std::fs::File::open(file_path)?;
    let file_crc = crc_of_stream(&mut file_handle)?;
    info!("CRC32: {file_crc}. Checking if up-to-date in release");

    // Only one asset may exist with each file-name
    match assets.iter().filter(|asset| asset.file_name == file_name).next() {
        Some(existing_asset) => {
            let existing_crc = get_asset_crc32(assets, &existing_asset.file_name, auth_token)?;

            // Asset already up to date
            if existing_crc == Some(file_crc) {
                info!("File up to date.");
                return Ok(());
            }   else    {
                info!("File not up to date {existing_crc:?}, deleting existing asset");
                // Need to update asset, so delete existing asset
                delete_asset(existing_asset, release, auth_token)?;
                let asset_crc_name = format!("{}.crc32", existing_asset.file_name);
                if let Some(crc_asset) = assets.iter().filter(|asset| asset.file_name == asset_crc_name).next() {
                    info!("Also deleting crc32");
                    delete_asset(crc_asset, release, auth_token)?;
                }
            }
        },
        None => info!("File does not already exist.")
    };

    // Upload the asset to the release
    info!("Uploading file and its CRC to the release");
    file_handle.seek(SeekFrom::Start(0))?;
    upload_asset_from_reader(release, &file_name, &mut file_handle, auth_token)?;

    // Upload the CRC-32 of the asset
    let crc_asset_name = format!("{file_name}.crc32");
    let be_bytes = file_crc.to_be_bytes();

    upload_asset_from_reader(release, &crc_asset_name, Cursor::new(be_bytes), auth_token)?;

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

pub fn update_release_from_directory(dir_path: impl AsRef<Path>, release: &Release, auth_token: &str) -> Result<()> {
    let release_files = get_assets(release, auth_token)?;
    let mut assets_no_file_match: HashSet<String> = release_files.iter()
        .filter(|asset| !asset.file_name.ends_with(".crc32"))
        .map(|asset| asset.file_name.clone())
        .collect();

    for file in list_files_json_last(&dir_path).context("Failed to list files")? {
        info!("Processing file {:?}", file.path());
        if file.path().extension() == Some(OsStr::new("crc32")) {
            warn!("The crc32 of each file is automatically generated and should not be included in this directory, skipping!");
            continue;
        }

        let path = file.path();
        let file_name = path.file_name().ok_or(anyhow!("File had no valid file name"))?;

        assets_no_file_match.remove(file_name.to_string_lossy().as_ref());
        upload_or_overwrite(path, &release_files, release, auth_token)?;
    }

    for file in release_files {
        // If the given file is a checksum, only delete it if the file it is a checksum for does not exist.
        let effective_name = if file.file_name.ends_with(".crc32") {
            &file.file_name[0..file.file_name.len() - 6]
        }   else    {
            &file.file_name
        };

        if !dir_path.as_ref().join(&effective_name).exists() {
            info!("Deleting {} from release", file.file_name);
            delete_asset(&file, release, auth_token)?;
        }
    }

    Ok(())
}