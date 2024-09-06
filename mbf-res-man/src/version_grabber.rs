use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, Context, Result};
use log::{error, info, warn};
use semver::Version;

use crate::{oculus_db::{self, AndroidBinary, ObbBinary}, APK_ID};

const BEATSABER_GRAPH_APP_ID: &str = "2448060205267927";

// Used to hold a Beat Saber version, which follows semver but has a build suffix that is not valid semver.
// The `non_semver` here is the complete version string with build suffix.
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct SemiSemVer {
    pub semver: Version,
    pub non_semver: String
}

// The set of files needed to install a particular Beat Saber build.
pub struct BeatSaberBinaries {
    apk_id: String, // APK binary ID
    version_code: u32,
    obb: Option<ObbInfo>
}

// Information about an OBB file needed by an APK.
pub struct ObbInfo {
    obb_id: String,
    obb_filename: String
}

// The different Beat Saber builds available for a particular version.
pub struct VersionBinaries {
    // The build with the newest version code.
    main: AndroidBinary,
    // Any builds with older version codes.
    older_versions: Vec<AndroidBinary>
}

// For the given Beat Saber version, attempts to find an OBB file needed for its installation.
// Returns the APK binary and its obb, if there is one.
fn get_obb_info(android_bin: &AndroidBinary, access_token: &str) -> Result<Option<ObbInfo>> {
    // Detect Beat Saber versions older than 1.34.6 and do not bother getting obb details: these versions do not use OBBs
    info!("Fetching OBB data for version {}", android_bin.version);
    let maybe_obb = oculus_db::get_obb_binary(&access_token, &android_bin.id)?;
    Ok(maybe_obb.map(|obb| ObbInfo {
        obb_id: obb.id,
        obb_filename: obb.file_name
    }))
}

/// Fetches all of the live (i.e. publicly accessible) Beat Saber versions newer than (or equal to) the specified minimum version.
/// If a version is invalid semver, it will be skipped.
pub fn get_live_bs_versions(access_token: &str, min_version: Version) -> Result<HashMap<SemiSemVer, VersionBinaries>> {
    // Each version may potentially have multiple binaries as there may be a quest 1 and quest 2+ binary.
    let mut versions_map: HashMap<String, Vec<AndroidBinary>> = HashMap::new();

    info!("Listing all app versions");
    let resp = oculus_db::list_app_versions(&access_token, BEATSABER_GRAPH_APP_ID)?;
    for mut binary in resp {
        // Skip non-live releases: these are private.
        if !binary.binary_release_channels.nodes.iter()
            .any(|channel| channel.channel_name == "LIVE") {
            continue;
        }

        // Add each version binary to the list for this version.
        // NB: There may be multiple APKs for each version as quest 1 uses a different APK to quest 2.
        match versions_map.get_mut(&binary.version) {
            Some(ver_list) => ver_list.push(binary),
            None => { versions_map.insert(binary.version.clone(), vec![binary]); }
        }
    }

    let mut ver_binaries_map: HashMap<SemiSemVer, VersionBinaries> = HashMap::new();
    for (ver, mut binary_vec) in versions_map {
        binary_vec.sort_by_key(|binary| -(binary.version_code as i32));

        // Remove the _BUILDID suffix from the version and attempt to parse it as semver.
        let semver = match semver::Version::parse(&ver.split('_').next()
            .expect("Version should not be empty")) {
                Ok(semver) => semver,
                Err(err) => {
                    // TODO: Some older (but valid) versions are not valid semver.
                    warn!("Beat Saber version {ver} was invalid semver, skipping!");
                    continue;
                }
            };


        if semver < min_version {
            continue;
        }

        let mut binary_iter = binary_vec.into_iter();
        let mut ver_binaries = VersionBinaries {
            // First binary
            main: binary_iter.next().ok_or(anyhow!("Beat Saber version had no binaries"))?,
            // Subsequent binaries, used on Quest 1
            older_versions: binary_iter.collect()
        };

        ver_binaries_map.insert(SemiSemVer {
            semver,
            non_semver: ver
        }, ver_binaries);
    }

    Ok(ver_binaries_map)
}

// Downloads the file with the given binary ID to the given path.
fn download_to_path(binary_id: &str, path: impl AsRef<Path>, access_token: &str) -> Result<()> {
    let mut reader = oculus_db::download_binary(access_token, binary_id)?;
    let mut writer = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)?;

    std::io::copy(&mut reader, &mut writer)?;
    Ok(())
}

// Attempts to download the file with the given binary ID to the given path and produces a warning if this fails.
fn download_and_warn_on_err(binary_id: &str, path: impl AsRef<Path>, access_token: &str) {
    match download_to_path(binary_id, path, access_token) {
        Ok(_) => {},
        Err(err) => error!("Failed to download BS APK/OBB: {err}")
    }
}

// Downloads all of the binaries for the given Beat Saber build to `to`. Appends `suffix` to the filenames if not empty.
fn download_binaries(access_token: &str, apk_binary: &AndroidBinary, to: impl AsRef<Path>, suffix: &str) -> Result<()> {
    let apk_path = to.as_ref().join(format!("{APK_ID}{suffix}.apk"));
    info!("Downloading APK");

    download_and_warn_on_err(&apk_binary.id, apk_path, access_token);

    let obb_info = get_obb_info(apk_binary, access_token).context("Getting OBB information")?;
    if let Some(obb) = obb_info {
        let obb_path = to.as_ref().join(format!("{}{}", obb.obb_filename, suffix));
        info!("Downloading OBB");
        download_and_warn_on_err(&obb.obb_id, obb_path, access_token);
    }

    Ok(())
}

/// Downloads the Beat Saber version with version name `version`
/// This will be stored in a directory with name `version` within `output_dir`
/// Iff `include_older_binaries` is `true`, this will also download Quest 1 only binaries.
pub fn download_version(access_token: &str,
    versions: &HashMap<SemiSemVer, VersionBinaries>,
    version: &str,
    include_older_binaries: bool,
    to_dir: impl AsRef<Path>,
    skip_existing: bool) -> Result<()> {
    let ver_path = to_dir.as_ref().join(version);
    if ver_path.exists() && skip_existing {
        return Ok(());
    }
    
    // Get a map of all available BS versions
    let matching_version: Option<&VersionBinaries> = versions.iter()
        .filter(|(ver, _)| ver.non_semver == version)
        .map(|(_, binaries)| binaries)
        .next();

    let binaries = match matching_version {
        Some(binaries) => binaries,
        None => return Err(anyhow!("Beat Saber version {version} not found"))
    };

    std::fs::create_dir_all(&ver_path)?;

    info!("Downloading binaries for {}", version);
    download_binaries(access_token, &binaries.main, &ver_path, "")?;
    if include_older_binaries {
        for other_bin in &binaries.older_versions {
            info!("Also downloading quest 1 only binaries");
            let ver_code_str = other_bin.version_code.to_string();
            download_binaries(access_token, &other_bin, &ver_path, &ver_code_str)?;
        }
    }

    Ok(())
}

// Downloads the currently available Beat Saber versions (skipping any that already have been downloaded.)
// Will skip any versions older than `min_version`
// Returns the latest Beat Saber version, as a string with its build suffix (_bignumber)
pub fn download_bs_versions(access_token: &str, output_dir: impl AsRef<Path>, min_version: Version,
    include_older_binaries: bool) -> Result<String> {
    info!("Using graph API to get version data");
    let mut latest_ver = SemiSemVer {
        non_semver: String::new(),
        semver: Version::new(0, 0, 0)
    };
    let output_dir = output_dir.as_ref();

    let versions = get_live_bs_versions(access_token, min_version)?;
    for (ver, binaries) in &versions {
        if ver.semver > latest_ver.semver {
            latest_ver = ver.clone();
        }

        download_version(access_token,
            &versions,
            &ver.non_semver,
            include_older_binaries,
            output_dir,
            true)?;
    }

    Ok(latest_ver.non_semver)
}