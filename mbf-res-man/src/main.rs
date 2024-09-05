// Allow dead code since some functions are only used when this crate is imported by the MBF agent.
#![allow(unused)]

use std::{cell::LazyCell, collections::HashMap, ffi::OsStr, fs::{FileType, OpenOptions}, io::Write, path::{Path, PathBuf}};
use adb::uninstall_package;
use anyhow::{anyhow, Context, Result};
use clap::{arg, command, Parser, Subcommand};
use const_format::formatcp;
use log::{info, warn};
use mbf_zip::ZipFile;
use models::{DiffIndex, VersionDiffs};
use oculus_db::{get_obb_binary, AndroidBinary};
use release_editor::Repo;
use semver::{Op, Version};
use version_grabber::SemiSemVer;

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
const META_TOKEN_PATH: &str = "META_TOKEN.txt";

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

// Attempts to find the path to the folder containing the APK/OBB for the Beat Saber version that begins with `bs_version`
// Will give an Err if no BS version is saved that begins with bs_version, OR if multiple exist.
fn find_bs_ver_beginning_with(bs_version: &str) -> Result<PathBuf> {
    let mut matching = Vec::new();
    for entry_res in std::fs::read_dir(Path::new(BS_VERSIONS_PATH))
        .context("Failed to read apk_data folder")? {

        // Skip entries where reading the metadata failed.
        if let Ok(entry) = entry_res {
            // Find directories that begin with the required BS version
            if entry.file_type().is_ok_and(|file_type| file_type.is_dir()) 
                && entry.file_name().to_string_lossy().starts_with(bs_version) {
                    matching.push(entry.path());
                }
        }
    }

    // Only succeed if there is exactly one match
    if matching.len() == 0 {
        Err(anyhow!("No BS versions found beginning with {bs_version}"))
    }   else if matching.len() > 1 {
        Err(anyhow!("Multiple BS versions exist beginning with {bs_version}"))
    }   else {
        Ok(matching.into_iter().next().unwrap())
    }
}

// Finds the path to the Beat Saber version `bs_version`
// Setting `fuzzy_lookup` to true will instead find the (single) Beat Saber version beginning with `bs_version`
// Gives an Err if the bs_version does not exist, or multiple match if using fuzzy lookup
fn get_bs_ver_path(bs_version: &str, fuzzy_lookup: bool) -> Result<PathBuf> {
    let version_path = Path::new(BS_VERSIONS_PATH).join(&bs_version);
    if !version_path.exists() && !fuzzy_lookup {
        Err(anyhow!("Beat Saber version {bs_version} not in cache!"))
    }   else {
        find_bs_ver_beginning_with(bs_version)
    }
}

// Installs the APK and pushes OBBs for the given Beat Saber version to the quest.
fn install_bs_version(bs_version: &str, fuzzy_lookup: bool) -> Result<()> {
    info!("Removing existing installation (if there is one)");
    let _ = uninstall_package(APK_ID); // Allow failure, i.e. if app not already installed.

    info!("Installing BS {bs_version}");
    let version_path = get_bs_ver_path(bs_version, fuzzy_lookup)?;

    let (apk_path, maybe_obb_path) = get_obb_and_apk_path(bs_version, fuzzy_lookup)
        .context("Failed to get APK and OBB path")?;

    info!("Installing APK");
    adb::install_apk(&apk_path.to_string_lossy())?;

    if let Some(obb_path) = maybe_obb_path {
        let obb_file_name = obb_path.file_name().ok_or(anyhow!("Obb had no filename"))?
            .to_string_lossy()
            .to_string();
        info!("Copying {obb_file_name}");

        let obb_dest = format!("/sdcard/Android/obb/{APK_ID}/{obb_file_name}");
        adb::push_file(&obb_path.to_string_lossy(), &obb_dest)?;
    }   else {
        info!("No OBB found to copy");
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

    let mut handle = std::io::BufWriter::new(std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(DIFF_INDEX_PATH)?);

    serde_json::to_writer_pretty(&mut handle, &index)?;
    Ok(())
}

// Gets the path to the OBB and APK for the given Beat Saber version.
// Will error if multiple OBBs exist, or if the version is not stored locally.
fn get_obb_and_apk_path(version: &str, fuzzy_lookup: bool) -> Result<(PathBuf, Option<PathBuf>)> {
    let version_path = get_bs_ver_path(version, fuzzy_lookup)?;

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

    Ok((apk_path, obb_path))
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

    let (from_apk, from_obb) = get_obb_and_apk_path(&from_version, false).context("Getting APK/OBB path for original version")?;
    let (to_apk, to_obb) = get_obb_and_apk_path(&to_version, false).context("Getting APK/OBB path for downgraded version")?;
    
    if from_obb == None || to_obb == None {
        return Err(anyhow!("One of the Beat Saber versions had no OBB! Obb-less diffs aren't supported by mbf-res-man"));
    }

    let apk_diff_name = format!("bs-apk-{from_version}-to-{to_version}.apk.diff");
    let obb_diff_name = format!("bs-obb-{from_version}-to-{to_version}.obb.diff");

    info!("Generating diff for APK");
    let apk_diff = diff_builder::generate_diff(from_apk, to_apk, Path::new(DIFFS_PATH)
        .join(apk_diff_name)).context("Failed to generate diff for APK")?;
    
    info!("Generating diff for OBB");
    let obb_diff = diff_builder::generate_diff(from_obb.unwrap(), to_obb.unwrap(), Path::new(DIFFS_PATH)
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

const GITHUB_TOKEN_PATH: &str = "GITHUB_TOKEN.txt";
const GITHUB_TOKEN: LazyCell<Option<&'static str>> = LazyCell::new(|| {
    if Path::new(GITHUB_TOKEN_PATH).exists() {
        Some(Box::leak(Box::new(std::fs::read_to_string(GITHUB_TOKEN_PATH)
            .expect("Failed to read GH token"))))
    }   else    {
        None
    }
});

// Attempts to get or load the github auth token from its containing file if present.
// Gives an Err if the auth token file does not exist.
fn get_github_auth_token() -> Result<&'static str> {
    GITHUB_TOKEN.ok_or(
        anyhow!("No github token found: {GITHUB_TOKEN_PATH} did not exist"))
}

// Updates the current diff index on github with the diffs in the diffs folder
fn upload_diff_index() -> Result<()> {
    info!("Updating diff index");

    let repo = Repo {
        repo: "mbf-diffs".to_string(),
        owner: "Lauriethefish".to_string()
    };

    let auth_token = get_github_auth_token()?;
    let latest_release = release_editor::get_latest_release(repo, auth_token)?;
    release_editor::update_release_from_directory(DIFFS_PATH, &latest_release, auth_token, false)?;

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

        let manifest_output_path = Path::new(MANIFESTS_PATH).join(format!("{version_name}.xml"));
        if manifest_output_path.exists() {
            continue;
        }
        info!("Extracting manifest for {version_name}");

        // Extract the manifest to an external file.
        let apk_path = folder.path().join(format!("{APK_ID}.apk"));
        let apk_handle = std::fs::File::open(apk_path).context("No APK found for BS version")?;
        let mut apk_zip = ZipFile::open(apk_handle).context("APK wasn't a valid ZIP file")?;
        let manifest_contents = apk_zip.read_file("AndroidManifest.xml").context("Failed to read manifest")?;

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

    let auth_token = get_github_auth_token()?;
    let latest_release = release_editor::get_latest_release(repo, auth_token)?;
    release_editor::update_release_from_directory(MANIFESTS_PATH, &latest_release, auth_token, false)?;
    Ok(())
}

// Based on the current Beat Saber versions in the index, does a few key actions:
// - Ensures all manifests are available on the manifests repo.
// - Ensures a diff exists from the latest version to the latest moddable version.
fn update_all_repositories(latest_bs_version: String) -> Result<()> {
    // First check all manifests are available
    update_manifests()?;
    upload_manifests()?;

    // Then generate a diff if necessary.
    let latest_moddable = get_latest_moddable_bs()?;
    if latest_bs_version == latest_moddable {
        info!("The installed Beat Saber version is already the latest moddable version, no need to generate diff");
    }   else {
        info!("Ensuring diff exists from latest to latest moddable");
        add_diff_to_index(latest_bs_version, latest_moddable, false)?;
        upload_diff_index()?;
    }

    Ok(())
}

fn merge_obb(version: String, out_path: impl AsRef<Path>) -> Result<()> {
    info!("Merging APK and OBB for version {version}");
    let (apk_path, maybe_obb_path) = get_obb_and_apk_path(&version, true)?;

    info!("Copying APK to destination");
    std::fs::copy(&apk_path, out_path.as_ref())
        .context("Failed to copy APK to destination path")?;
    
    let apk_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(out_path)?;
    let mut apk_zip = ZipFile::open(apk_file)
        .context("APK was not valid ZIP archive")?;

    let obb_path = maybe_obb_path.ok_or(anyhow!("No OBB found for v{version} to merge"))?;

    let mut obb_zip = ZipFile::open(std::fs::File::open(obb_path)?)
        .context("OBB was not valid ZIP archive")?;

    info!("Copying entries from OBB into APK");
    obb_zip.copy_all_entries_to(&mut apk_zip).context("Failed to copy over OBB entries")?;

    const CERT_PEM: &[u8] = include_bytes!("../../mbf-agent/src/debug_cert.pem");
    let (cert, priv_key) = mbf_zip::signing::load_cert_and_priv_key(CERT_PEM);
    apk_zip.save_and_sign_v2(&priv_key, &cert).context("Failed to sign/save APK")?;
    Ok(())
}

#[derive(Parser)]
#[command(version, long_about = None)]
#[command(arg_required_else_help = true)]
#[command(about = "Automation for the management of resources needed for MBF to function")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[clap(short, long)]
    access_token: Option<String>
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
    /// Downloads the specified Beat Saber version from the Oculus API
    FetchVersion {
        version: String,
        #[arg(short, long)]
        older_binaries: bool,
    },
    /// Lists the currently LIVE Beat Saber versions from the Oculus API.
    ListVersions,
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
        password: String,
        /// Saves the access token to disk to be used in later commands without entering it each time.
        #[arg(short, long)]
        save: bool
    },
    MergeObb {
        #[arg(short, long)]
        version: String,
        #[arg(short, long)]
        out_path: String
    },
    /// Fetches Beat Saber versions from the oculus database, then:
    /// - Ensures all manifests are available on the manifests repo.
    /// - Ensures the latest version has a diff to the latest moddable version (if not the same)
    /// - Uploads the diff to the mbf-diffs repo.
    UpdateReposFromOculusApi {
        #[arg(short, long)]
        min_version: Option<String>
    }
}

/// If `argument` is Some, this unwraps `argument` and returns the contained access token.
/// Otherwise, this function tries to load the meta access token from META_TOKEN_PATH
/// Gives an error if reading the file fails. (or the file doesn't exist)
fn get_or_load_access_token(argument: Option<String>) 
    -> Result<String> {
    match argument {
        Some(arg_token) => Ok(arg_token),
        None => {
            if Path::new(META_TOKEN_PATH).exists() {
                Ok(std::fs::read_to_string(META_TOKEN_PATH)?)
            }   else {
                Err(anyhow!("No meta access token passed (with argument -a <token>) and
{META_TOKEN_PATH} did not exist, so could not get access token"))
            }
        }
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
        Commands::FetchVersion { version, older_binaries } => {
            let access_token = get_or_load_access_token(cli.access_token)?;
            let versions = version_grabber::get_live_bs_versions(&access_token, Version::new(0, 0, 0))?;

            version_grabber::download_version(&access_token,
                &versions,
                &version,
                older_binaries,
                BS_VERSIONS_PATH,
                false)?;
        },
        Commands::ListVersions => {
            let access_token = get_or_load_access_token(cli.access_token)?;
            let mut versions: Vec<_> = version_grabber::get_live_bs_versions(&access_token, Version::new(0, 0, 0))?
                .into_keys()
                .collect();
            versions.sort_by_cached_key(|ver| ver.semver.clone());

            for version in versions {
                info!("{}", version.non_semver);
            }
        }
        Commands::GenerateDiff { from_version, to_version, overwrite } => add_diff_to_index(from_version, to_version, overwrite)?,
        Commands::GenerateDiffToLatest { from_version, overwrite } => {
            let latest_moddable = get_latest_moddable_bs()?;
            if from_version == latest_moddable {
                return Err(anyhow!("{from_version} is already the latest moddable version"));
            }

            add_diff_to_index(from_version, latest_moddable, overwrite)?;
        }
        Commands::UpdateDiffIndex => upload_diff_index()?,
        // Enable fuzzy lookup so that if e.g. 1.28.0 is selected, the command will find the full version string with the build suffix and install that
        Commands::InstallVersion { version } => install_bs_version(&version, true)?,
        Commands::InstallLatestModdable => install_bs_version(&get_latest_moddable_bs()?, false)?,
        Commands::UpdateManifestsRepo => {
            update_manifests()?;
            upload_manifests()?;
        },
        Commands::AcceptNewVersion => {
            let installed_bs_version = download_installed_bs()?;
            
            update_all_repositories(installed_bs_version)?;
        },
        Commands::GetAccessToken { email, password, save } => {
            let token = oculus_db::get_quest_access_token(&email, &password)?;
            // Save the access token to a file if specified.
            if save {
                info!("Access token saved!");
                let token_bytes = token.as_bytes();
                let mut token_writer = OpenOptions::new()
                    .truncate(true)
                    .write(true)
                    .create(true)
                    .open(META_TOKEN_PATH)?;
                token_writer.write_all(token_bytes)?;
            }   else {
                info!("Access token: {token}");
            }
        },
        Commands::UpdateReposFromOculusApi { min_version } => {
            let access_token = get_or_load_access_token(cli.access_token)?;

            let min_version_semver = match min_version {
                Some(version_string) => semver::Version::parse(&version_string).context("Failed to parse provided version string")?,
                None => semver::Version::new(0, 0, 0)
            };

            let latest_bs_version = version_grabber::download_bs_versions(&access_token, BS_VERSIONS_PATH, min_version_semver, false)?;
            info!("Latest Beat Saber version is {latest_bs_version}");
            update_all_repositories(latest_bs_version)?;

        },
        Commands::MergeObb { version, out_path } => { merge_obb(version, out_path)? }
    }

    Ok(())
}