//! Handler for the GetModStatus request.

use std::{fs::File, io::Cursor};

use log::{error, info, warn};
use mbf_res_man::{
    external_res, models::{CoreMod, VersionDiffs}, res_cache::{self, ResCache}
};
use mbf_zip::ZipFile;

use crate::{
    downgrading, manifest::ManifestInfo, mod_man::ModManager, models::response::{self, CoreModsInfo, Response}, patching
};
use mbf_axml::{self, AxmlReader};
use anyhow::{Context, Result};

/// Handles `GetModStatus` [Requests](response::Request).
///
/// # Returns
/// The [Response](response::Response) to the request (variant `ModStatus`)
pub(super) fn handle_get_mod_status(override_core_mod_url: Option<String>) -> Result<Response> {
    crate::try_delete_legacy_dirs();

    info!("Searching for Beat Saber app");
    let app_info = get_app_info()?;
    let res_cache = crate::load_res_cache()?;

    let (core_mods, installed_mods) = match &app_info {
        Some(app_info) => {
            info!("Loading installed mods");
            let mut mod_manager = ModManager::new(app_info.version.clone(), &res_cache);
            mod_manager.load_mods().context("Loading installed mods")?;

            (
                get_core_mods_info(
                    &app_info.version,
                    &mod_manager,
                    override_core_mod_url,
                    &res_cache,
                    app_info.loader_installed.is_some(),
                )?,
                super::mod_management::get_mod_models(mod_manager)?,
            )
        }
        None => {
            warn!("Beat Saber is not installed!");
            (None, Vec::new())
        }
    };

    Ok(Response::ModStatus {
        app_info,
        core_mods,
        modloader_install_status: patching::get_modloader_status()?,
        installed_mods,
    })
}

pub(super) fn get_app_info() -> Result<Option<response::AppInfo>> {
    let apk_path = match crate::get_apk_path().context("Finding APK path")? {
        Some(path) => path,
        None => return Ok(None),
    };

    let apk_reader = std::fs::File::open(&apk_path)?;
    let mut apk = ZipFile::open(apk_reader).context("Reading APK as ZIP")?;

    let modloader = patching::get_modloader_installed(&mut apk)?;
    let obb_present = patching::check_obb_present()?;

    let (manifest_info, manifest_xml) = get_manifest_info_and_xml(&mut apk)?;
    Ok(Some(response::AppInfo {
        loader_installed: modloader,
        version: manifest_info.package_version,
        obb_present,
        path: apk_path,
        manifest_xml,
    }))
}

fn get_manifest_info_and_xml(apk: &mut ZipFile<File>) -> Result<(ManifestInfo, String)> {
    let manifest = apk
        .read_file("AndroidManifest.xml")
        .context("Reading manifest file from APK")?;

    // Decode various important information from the manifest
    let mut manifest_reader = Cursor::new(&manifest);
    let mut axml_reader = AxmlReader::new(&mut manifest_reader)?;
    let manifest_info =
        ManifestInfo::read(&mut axml_reader).context("Parsing manifest from AXML")?;

    // Re-read the manifest as a full XML document.
    let xml_str =
        axml_bytes_to_xml_string(&manifest).context("Converting manifest to readable XML")?;

    Ok((manifest_info, xml_str))
}

pub(super) fn axml_bytes_to_xml_string(bytes: &[u8]) -> Result<String> {
    let mut cursor = Cursor::new(bytes);
    let mut axml_reader = AxmlReader::new(&mut cursor)
        .context("File on manifests URI was invalid AXML. Report this!")?;

    let mut xml_output = Vec::new();
    let mut xml_writer = xml::EmitterConfig::new()
        .perform_indent(true)
        .create_writer(Cursor::new(&mut xml_output));

    mbf_axml::axml_to_xml(&mut xml_writer, &mut axml_reader).context("Converting AXML to XML")?;

    Ok(String::from_utf8(xml_output).expect("XML output should be valid UTF-8"))
}

fn get_core_mods_info(
    apk_version: &str,
    mod_manager: &ModManager,
    override_core_mod_url: Option<String>,
    res_cache: &ResCache,
    is_patched: bool,
) -> Result<Option<CoreModsInfo>> {
    // Fetch the core mods from the resources repo
    info!("Fetching core mod index");
    let core_mods =
        match mbf_res_man::external_res::fetch_core_mods(res_cache, override_core_mod_url) {
            Ok(mods) => mods,
            Err(res_cache::JsonPullError::FetchError(fetch_err)) => {
                error!(
                "Failed to fetch core mod index: assuming no internet connection: {fetch_err:?}"
            );
                return Ok(None);
            }
            Err(res_cache::JsonPullError::ParseError(err)) => return Err(err.into()),
        };

    // Check that all core mods are installed with an appropriate version
    let all_core_mods_installed = match core_mods.get(apk_version) {
        Some(core_mods) => get_core_mods_install_status(&core_mods.mods, mod_manager),
        None => response::InstallStatus::Missing,
    };

    let supported_versions: Vec<String> = core_mods
        .into_keys()
        .filter(|version| {
            let mut iter = version.split('.');
            let _major = iter.next().unwrap();
            let minor = iter.next().unwrap();

            minor
                .parse::<i64>()
                .expect("Invalid version in core mod index")
                >= 35
        })
        .collect();
    let is_version_supported = supported_versions.iter().any(|ver| ver == apk_version);

    // If the app is patched and not vanilla, then it's not possible to downgrade it even if a diff is available for the corresponding vanilla APK
    // Therefore, we can skip fetching the diff index, which will help startup times.

    let (downgrade_versions, newer_than_latest_diff) = if is_patched {
        // While technically newer_than_latest_diff is true, our app is patched already so we couldn't downgrade it
        // even if there was a diff available.
        (Vec::new(), false)
    } else {
        let diff_index = downgrading::get_all_accessible_versions(res_cache, apk_version)
            .context("Formatting downgrading information")?;

        let newer_than_latest = is_version_newer_than_latest_diff(apk_version, &external_res::get_diff_index(res_cache)?);
        (
            diff_index.into_keys().collect(),
            newer_than_latest,
        )
    };

    Ok(Some(CoreModsInfo {
        supported_versions,
        core_mod_install_status: all_core_mods_installed,
        downgrade_versions,
        is_awaiting_diff: newer_than_latest_diff && !is_version_supported,
    }))
}

// Attempts to work out if the provided apk_version is newer than the latest version that has diffs to the latest moddable version
// Essentially, this returns true iff the diff index is outdated, since the APK version given doesn't have the necessary diff to mod the game.
// This is used by the frontend to explain that they need to wait for a diff to be generated.
fn is_version_newer_than_latest_diff(apk_version: &str, diffs: &[VersionDiffs]) -> bool {
    let sem_apk_ver = match try_parse_bs_ver_as_semver(apk_version) {
        Some(ver) => ver,
        None => {
            warn!("APK version {apk_version} did not have a valid semver section as Beat Saber versions normally do");
            warn!("Will be unable to check if version is newer than latest diff");
            return false;
        }
    };

    // Iterate through the diffs to check if the APK version is at or below their version
    for diff in diffs {
        match try_parse_bs_ver_as_semver(&diff.from_version) {
            Some(diff_version) => {
                if sem_apk_ver <= diff_version {
                    return false; // If it is, then this is not an "awaiting diff" situation
                }
            }
            None => {}
        }
    }

    // APK version is not equal to or older than ANY of the versions with diffs
    // Hence, it is newer than the latest diff
    true
}

fn try_parse_bs_ver_as_semver(version: &str) -> Option<semver::Version> {
    let version_segment = version
        .split('_')
        .next()
        .expect("Split iterator always returns at least one string");
    semver::Version::parse(version_segment).ok()
}

// Checks whether all the core mods in the provided slice are present within the mod manager given.
// Will give InstallStatus::Ready if all core mods are installed and up to date,
// InstallStatus::NeedUpdate if any core mods are out of date but all are installed, and InstallStatus::Missing if any
// of the core mods are not installed or not even present.
fn get_core_mods_install_status(
    core_mods: &[CoreMod],
    mod_man: &ModManager,
) -> response::InstallStatus {
    info!("Checking if core mods installed and up to date");
    mark_all_core_mods(mod_man, core_mods);

    let mut missing_core_mods = false;
    let mut outdated_core_mods = false;
    for core_mod in core_mods {
        match mod_man.get_mod(&core_mod.id) {
            Some(existing_mod) => {
                let mod_ref = existing_mod.borrow();

                // NB: We consider a core mod "missing" if it is present but not installed.
                // Technically it's not "missing" in this case but for the user the difference isn't relevant since MBF auto-downloads mods either way.
                if !mod_ref.installed() {
                    warn!("Core mod {} was present (ver {}) but is not installed: needs to be installed", core_mod.id, mod_ref.manifest().version);
                    missing_core_mods = true;
                } else if mod_ref.manifest().version < core_mod.version {
                    warn!(
                        "Core mod {} is outdated, latest version: {}, installed version: {}",
                        core_mod.id,
                        core_mod.version,
                        mod_ref.manifest().version
                    );
                    outdated_core_mods = true;
                }
            }
            None => missing_core_mods = true,
        }
    }

    if missing_core_mods {
        response::InstallStatus::Missing
    } else if outdated_core_mods {
        response::InstallStatus::NeedUpdate
    } else {
        response::InstallStatus::Ready
    }
}

/// Marks all of the mods with IDs matching mods in `core_mods` and all of their dependencies as core within the provided ModManager
pub(super) fn mark_all_core_mods(mod_manager: &ModManager, core_mods: &[CoreMod]) {
    for core_mod in core_mods {
        mod_manager.set_mod_core(&core_mod.id);
    }
}
