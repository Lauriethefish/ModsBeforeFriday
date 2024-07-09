// Allow dead code since some functions are only used when this crate is imported by the MBF agent.
#![allow(unused)]

use std::{collections::HashMap, ffi::OsStr, io::Write, path::{Path, PathBuf}};
use adb::uninstall_package;
use anyhow::{anyhow, Context, Result};
use clap::{arg, command, Parser, Subcommand};
use const_format::formatcp;
use log::{info, warn};
use mbf_zip::ZipFile;
use models::{DiffIndex, VersionDiffs};
use oculus_db::{get_obb_binary, AndroidBinary};
use release_editor::Repo;

mod models;
mod adb;
mod diff_builder;
mod external_res;
mod release_editor;
mod oculus_db;
mod version_grabber;

const APK_ID: &str = "com.beatgames.beatsaber";
const APK_DATA_DIR: &str = "apk_data";
const BS_VERSIONS_PATH: &str = formatcp!("{APK_DATA_DIR}/versions");
const DIFFS_PATH: &str = formatcp!("{APK_DATA_DIR}/diffs");
const MANIFESTS_PATH: &str = formatcp!("{APK_DATA_DIR}/manifests");
const DIFF_INDEX_PATH: &str = formatcp!("{DIFFS_PATH}/index.json");

// Downloads the installed version of Beat Saber to BS_VERSIONS_PATH
fn download_installed_bs() -> Result<String> {
    info!("Downloading the currently installed copy of Beat Saber");
    warn!("Check that your APK is not modded before running this command");

    let bs_version = match adb::get_package_version(APK_ID)? {
        Some(ver) => ver,
        None => return Err(anyhow!("Package {APK_ID} is not installed"))
    };

    let version_path = Path::new(BS_VERSIONS_PATH).join(&bs_version);
    std::fs::create_dir_all(&version_path).context("Failed to create output path")?;

    info!("Downloading APK");
    let apk_output_path = version_path.join(format!("{APK_ID}.apk"));

    adb::download_apk(APK_ID, &apk_output_path.to_string_lossy())
        .context("Failed to download APK")?;

    info!("Downloading OBB file(s)");

    adb::download_obbs(APK_ID, &version_path).context("Failed to download OBBs")?;
    Ok(bs_version)
}

// Installs the APK and pushes OBBs for the given Beat Saber version to the quest.
fn install_bs_version(bs_version: &str) -> Result<()> {
    info!("Removing existing installation (if there is one)");
    let _ = uninstall_package(APK_ID); // Allow failure, i.e. if app not already installed.

    info!("Installing BS {bs_version}");
    let version_path = Path::new(BS_VERSIONS_PATH).join(&bs_version);
    if !version_path.exists() {
        return Err(anyhow!("Beat Saber version {bs_version} not in cache!"));
    }

    let apk_path = version_path.join(format!("{APK_ID}.apk"));
    info!("Installing APK");
    adb::install_apk(&apk_path.to_string_lossy())?;

    for file_res in std::fs::read_dir(version_path)? {
        let file_path = file_res?.path();

        // Upload any files with the obb file extension to the obbs folder on the quest.
        if file_path.extension() == Some(OsStr::new("obb")) {
            let obb_file_name = file_path.file_name().ok_or(anyhow!("Obb had no filename"))?
                .to_string_lossy()
                .to_string();
            info!("Copying {obb_file_name}");

            let obb_dest = format!("/sdcard/Android/obb/{APK_ID}/{obb_file_name}");
            adb::push_file(&file_path.to_string_lossy(), &obb_dest)?;
        }
    }

    Ok(())
}


// Loads the current diff index from disk.
fn load_current_diffs() -> Result<DiffIndex> {
    info!("Loading current diff index");
    if !Path::new(DIFF_INDEX_PATH).exists() {
        return Ok(Vec::new())
    }

    let mut handle = std::fs::File::open(DIFF_INDEX_PATH)?;
    Ok(serde_json::from_reader(&mut handle).context("Existing diff index was invalid JSON")?)
}

// Saves the given diff index to its location on disk.
fn save_diff_index(index: DiffIndex) -> Result<()> {
    info!("Saving diff index");
    std::fs::create_dir_all(DIFFS_PATH)?;

    let mut handle = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(DIFF_INDEX_PATH)?;

    serde_json::to_writer_pretty(&mut handle, &index)?;
    Ok(())
}

// Gets the path to the OBB and APK for the given Beat Saber version.
// Will error if multiple OBBs exist, or if the version is not stored locally.
fn get_obb_and_apk_path(version: &str) -> Result<(PathBuf, PathBuf)> {
    let version_path = Path::new(BS_VERSIONS_PATH).join(&version);
    if !version_path.exists() {
        return Err(anyhow!("Beat Saber version {version} is not pulled"));
    }

    // APK is always in the same place
    let apk_path = version_path.join(format!("{APK_ID}.apk"));
    if !apk_path.exists() {
        return Err(anyhow!("APK did not exist"));
    }

    // Iterate the folder and find the .OBB file.
    // NB: Multiple OBBs are not yet supported. If Beat Saber begins using multiple OBBs,
    // then we will have to make a decision about how to do the downgrading: e.g. concatenating the OBBs and downgrading this to the previous version.

    // We will also have to figure out how to associate the OBBs containing the same content with each other
    // (we cannot JUST do this by filename as the filename of each OBB varies with version, so it'll take some figuring out)
    let mut obb_path: Option<PathBuf> = None;
    for file_result in std::fs::read_dir(version_path)? {
        let file_path = file_result?.path();

        // Find the file with OBB extension
        if file_path.extension() == Some(OsStr::new("obb")) {
            match obb_path {
                Some(_) => return Err(anyhow!("Multiple obb files existed for {version}. A decision will need to be made about how diffs are handled")),
                None => obb_path = Some(file_path)
            }
        }
    }

    if let Some(path_of_obb) = obb_path {
        Ok((apk_path, path_of_obb))
    }   else {
        Err(anyhow!("No obb file found"))
    }
}

// Removes a file if it already exists.
fn remove_if_exists(path: impl AsRef<Path>) -> Result<()> {
    if path.as_ref().exists() {
        std::fs::remove_file(path)?;
    }

    Ok(())
}

// Checks if a diff exists to downgrade from `from_version` -> `to_version`
// If no diff exists, this returns Ok()
// If a diff exists, and `delete_if_exists` is `true` then the existing diff is
// removed from the index.
// If `delete_if_exists` is false, then this function gives an error.
fn verify_no_existing_diff(index: &mut DiffIndex,
    from_version: &str,
    to_version: &str,
    delete_if_exists: bool) -> Result<()> {
    match index.iter()
        .filter(|diff| diff.from_version == from_version && diff.to_version == to_version)
        .next() {
        Some(diff) => if !delete_if_exists {
            Err(anyhow!("A diff already existed to go from {from_version} to {to_version}"))
        }   else    {
            info!("Removing existing diff");
            // Remove all files for the existing diff
            remove_if_exists(Path::new(DIFFS_PATH).join(&diff.apk_diff.diff_name))?;
            for obb_diff in &diff.obb_diffs {
                remove_if_exists(Path::new(DIFFS_PATH).join(&obb_diff.diff_name))?;
            }

            // Remove the diff from the index.
            index.retain(|diff| diff.from_version != from_version || diff.to_version != to_version);
            Ok(())

        },
        None => Ok(())
    }
}

// Generates a diff to go between the two provided Beat Saber versions
fn add_diff_to_index(from_version: String, to_version: String, delete_existing: bool) -> Result<()> {
    info!("Preparing to generate diff from {from_version} to {to_version}");

    let mut current_diff_idx = load_current_diffs().context("Failed to load diff index")?;

    verify_no_existing_diff(&mut current_diff_idx, &from_version, &to_version, delete_existing).context("Failed to verify removal of existing diff")?;

    let (from_apk, from_obb) = get_obb_and_apk_path(&from_version).context("Getting APK/OBB path for original version")?;
    let (to_apk, to_obb) = get_obb_and_apk_path(&to_version).context("Getting APK/OBB path for downgraded version")?;
    
    let apk_diff_name = format!("bs-apk-{from_version}-to-{to_version}.apk.diff");
    let obb_diff_name = format!("bs-obb-{from_version}-to-{to_version}.obb.diff");

    info!("Generating diff for APK");
    let apk_diff = diff_builder::generate_diff(from_apk, to_apk, Path::new(DIFFS_PATH)
        .join(apk_diff_name)).context("Failed to generate diff for APK")?;
    info!("Generating diff for OBB");
    let obb_diff = diff_builder::generate_diff(from_obb, to_obb, Path::new(DIFFS_PATH)
        .join(obb_diff_name)).context("Failed to generate diff for OBB")?;

    info!("Adding to diff index");
    current_diff_idx.push(VersionDiffs {
        apk_diff,
        obb_diffs: vec![obb_diff],
        from_version,
        to_version,
    });
    save_diff_index(current_diff_idx).context("Failed to save diff index")?;

    Ok(())
}

// Converts a Beat Saber version string to semver.
fn bs_ver_to_semver(bs_ver: &str) -> semver::Version {
    let semver_portion = bs_ver.split('_')
        .next()
        .expect("Beat Saber version should not be blank");

    semver::Version::parse(semver_portion).expect("BS version was invalid semver")
}

// Gets the latest moddable version of Beat Saber
fn get_latest_moddable_bs() -> Result<String> {
    info!("Working out latest moddable version");

    let core_mods = crate::external_res::fetch_core_mods(None)
        .context("Failed to GET core mod index")?;

    let latest_ver = core_mods.into_keys().max_by(|version_a, version_b|
        bs_ver_to_semver(version_a).cmp(&bs_ver_to_semver(version_b)));

    latest_ver.ok_or(anyhow!("No Beat Saber versions were moddable"))
}

const GITHUB_AUTH_TOKEN: &str = include_str!("../GITHUB_TOKEN.txt");

// Updates the current diff index on github with the diffs in the diffs folder
fn upload_diff_index() -> Result<()> {
    info!("Updating diff index");

    let repo = Repo {
        repo: "mbf-diffs".to_string(),
        owner: "Lauriethefish".to_string()
    };
    let latest_release = release_editor::get_latest_release(repo, GITHUB_AUTH_TOKEN)?;
    release_editor::update_release_from_directory(DIFFS_PATH, &latest_release, GITHUB_AUTH_TOKEN)?;

    Ok(())
}

// Updates the AndroidManifest.xml files to reflect the current Beat Saber versions the script knows about.
fn update_manifests() -> Result<()> {
    info!("Extracting manifests from APKs");
    std::fs::create_dir_all(MANIFESTS_PATH)?;

    for folder_result in std::fs::read_dir(BS_VERSIONS_PATH)? {
        let folder = folder_result?;

        let version_name = folder
            .path()
            .file_name()
            .expect("No filename in BS version")
            .to_string_lossy()
            .to_string();
        info!("Extracting manifest for {version_name}");

        // Extract the manifest to an external file.
        let apk_path = folder.path().join(format!("{APK_ID}.apk"));
        let apk_handle = std::fs::File::open(apk_path).context("No APK found for BS version")?;
        let mut apk_zip = ZipFile::open(apk_handle).context("APK wasn't a valid ZIP file")?;
        let manifest_contents = apk_zip.read_file("AndroidManifest.xml").context("Failed to read manifest")?;

        let manifest_output_path = Path::new(MANIFESTS_PATH).join(format!("{version_name}.xml"));
        let mut out_handle = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(manifest_output_path)?;

        out_handle.write_all(&manifest_contents)?;
    }

    Ok(())
}

fn upload_manifests() -> Result<()> {
    info!("Updating manifests GH release");
    let repo = Repo {
        repo: "mbf-manifests".to_string(),
        owner: "Lauriethefish".to_string()
    };
    let latest_release = release_editor::get_latest_release(repo, GITHUB_AUTH_TOKEN)?;
    release_editor::update_release_from_directory(MANIFESTS_PATH, &latest_release, GITHUB_AUTH_TOKEN)?;
    Ok(())
}

#[derive(Parser)]
#[command(version, long_about = None)]
#[command(arg_required_else_help = true)]
#[command(about = "Automation for the management of resources needed for MBF to function")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Downloads the currently installed Beat Saber APK and OBB(s) into the local version list
    PullVersion,
    /// Generates a diff file to downgrade between the two given versions, and adds it to the diff index.
    GenerateDiff {
        #[arg(short, long)]
        from_version: String,
        #[arg(short, long)]
        to_version: String,
        #[arg(short, long)]
        overwrite: bool
    },
    /// Generates a diff file to downgrade from the given game version to the latest moddable version, and adds it to the diff index.
    GenerateDiffToLatest {
        #[arg(short, long)]
        from_version: String,
        #[arg(short, long)]
        overwrite: bool
    },
    /// Installs the given Beat Saber version onto the Quest.
    InstallVersion {
        version: String
    },
    /// Installs the latest moddable Beat Saber version onto the Quest.
    InstallLatestModdable,
    /// Uploads any changes made to the mbf diffs index.
    UpdateDiffIndex,
    /// Extracts all AndroidManifest.xml files from APKs and uploads them to the MBF manifests repo.
    UpdateManifestsRepo,
    /// Convenience command for use when a Beat Saber update releases.
    /// - Pulls the new update from the quest.
    /// - Generates a diff from this version to the latest moddable version.
    /// - Uploads the diff to the mbf-diffs repo.
    /// - Extracts the manifest from this version's APK.
    /// - Uploads the manifest to the manifests repo.
    AcceptNewVersion,
    /// Obtains a meta access token that can be used to download Beat Saber versions.
    /// Requires the email and password to log in.
    GetAccessToken {
        #[arg(short, long)]
        email: String,
        #[arg(short, long)]
        password: String
    },
    /// Fetches Beat Saber versions from the oculus database
    DownloadVersions {
        #[arg(short, long)]
        access_token: String,
        #[arg(short, long)]
        min_version: Option<String>
    }
}


fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_target(false)
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::PullVersion => { download_installed_bs()?; },
        Commands::GenerateDiff { from_version, to_version, overwrite } => add_diff_to_index(from_version, to_version, overwrite)?,
        Commands::GenerateDiffToLatest { from_version, overwrite } => {
            let latest_moddable = get_latest_moddable_bs()?;
            if from_version == latest_moddable {
                return Err(anyhow!("{from_version} is already the latest moddable version"));
            }

            add_diff_to_index(from_version, latest_moddable, overwrite)?;
        }
        Commands::UpdateDiffIndex => upload_diff_index()?,
        Commands::InstallVersion { version } => install_bs_version(&version)?,
        Commands::InstallLatestModdable => install_bs_version(&get_latest_moddable_bs()?)?,
        Commands::UpdateManifestsRepo => {
            update_manifests()?;
            upload_manifests()?;
        },
        Commands::AcceptNewVersion => {
            let installed_bs_version = download_installed_bs()?;
            let latest_moddable = get_latest_moddable_bs()?;
            if installed_bs_version == latest_moddable {
                return Err(anyhow!("The installed Beat Saber version is already the latest moddable version"));
            }

            add_diff_to_index(installed_bs_version, latest_moddable, false)?;
            upload_diff_index()?;
            update_manifests()?;
            upload_manifests()?;
        },
        Commands::GetAccessToken { email, password } => {
            let token = oculus_db::get_quest_access_token(&email, &password)?;
            info!("Access token: {token}");
        },
        Commands::DownloadVersions { access_token, min_version } => {
            let min_version_semver = match min_version {
                Some(version_string) => semver::Version::parse(&version_string).context("Failed to parse provided version string")?,
                None => semver::Version::new(0, 0, 0)
            };

            version_grabber::download_bs_versions(&access_token, BS_VERSIONS_PATH, min_version_semver)?;
        }
    }

    Ok(())
}