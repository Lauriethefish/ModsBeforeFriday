use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest::ManifestInfo;
use crate::{axml, data_fix, downloads, APK_ID, DATAKEEPER_PATH, DOWNLOADS_PATH, PLAYER_DATA_BAK_PATH, PLAYER_DATA_PATH, SONGS_PATH, TEMP_PATH};
use crate::{axml::AxmlReader, patching};
use mbf_res_man::models::{CoreMod, VersionDiffs};
use mbf_zip::ZipFile;
use crate::mod_man::ModManager;
use crate::requests::{AppInfo, CoreModsInfo, ImportResultType, InstallStatus, ModModel, Request, Response};
use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
use mbf_res_man::res_cache::{JsonPullError, ResCache};
use xml::EmitterConfig;


pub fn handle_request(request: Request) -> Result<Response> {
    match request {
        Request::GetModStatus { override_core_mod_url } => handle_get_mod_status(override_core_mod_url),
        Request::Patch { downgrade_to ,
            remodding,
            manifest_mod,
            allow_no_core_mods,
            override_core_mod_url,
            vr_splash_path} => 
            handle_patch(downgrade_to, remodding, manifest_mod, allow_no_core_mods, override_core_mod_url, vr_splash_path),
        Request::SetModsEnabled {
            statuses
        } => run_mod_action(statuses),
        Request::QuickFix { override_core_mod_url, wipe_existing_mods } => handle_quick_fix(override_core_mod_url, wipe_existing_mods),
        Request::Import { from_path } => handle_import(from_path, None),
        Request::RemoveMod { id } => handle_remove_mod(id),
        Request::ImportUrl { from_url } => handle_import_mod_url(from_url),
        Request::FixPlayerData => handle_fix_player_data(),
        Request::GetDowngradedManifest { version } => handle_get_downgraded_manifest(version)
    }
}

fn run_mod_action(statuses: HashMap<String, bool>) -> Result<Response> {
    let mut mod_manager = ModManager::new(&get_app_version_only()?);
    mod_manager.load_mods().context("Failed to load installed mods")?;

    let mut error = String::new();

    for (id, new_status) in statuses {
        let mod_rc = match mod_manager.get_mod(&id) {
            Some(m) => m,
            None => {
                error.push_str(&format!("Mod with ID {id} did not exist\n"));
                continue;
            }
        };

        let already_installed = mod_rc.borrow().installed();
        if new_status && !already_installed {
            match mod_manager.install_mod(&id) {
                Ok(_) => info!("Installed {id}"),
                Err(err) => error.push_str(&format!("Failed to install {id}: {err}\n"))
            }
        }   else if !new_status && already_installed {
            match mod_manager.uninstall_mod(&id) {
                Ok(_) => info!("Uninstalled {id}"),
                Err(err) => error.push_str(&format!("Failed to install {id}: {err}\n"))
            }
        }
    }

    Ok(Response::ModSyncResult {
        installed_mods: get_mod_models(mod_manager)?,
        failures: if error.len() > 0 {
            if error.ends_with('\n') {
                error.pop();
            }

            Some(error)
        }   else    {
            None
        }
    })
}

fn handle_get_mod_status(override_core_mod_url: Option<String>) -> Result<Response> {
    crate::try_delete_legacy_dirs();

    info!("Searching for Beat Saber app");
    let app_info = get_app_info()?;
    let res_cache = crate::load_res_cache()?;

    let (core_mods, installed_mods) = match &app_info {
        Some(app_info) => {
            info!("Loading installed mods");
            let mut mod_manager = ModManager::new(&app_info.version);
            mod_manager.load_mods().context("Failed to load installed mods")?;
            
            (
                get_core_mods_info(&app_info.version, &mod_manager, override_core_mod_url, &res_cache, app_info.loader_installed.is_some())?,
                get_mod_models(mod_manager)?
            )
        },
        None => {
            warn!("Beat Saber is not installed!");
            (None, Vec::new())
        }
    };

    Ok(Response::ModStatus { 
        app_info,
        core_mods,
        modloader_install_status: patching::get_modloader_status()?,
        installed_mods
    })
}

fn get_mod_models(mut mod_manager: ModManager) -> Result<Vec<ModModel>> {
    // A dependency of one mod may have been installed in the operation
    // That mod is now therefore considered installed, even though it wasn't when the mods were loaded
    // Therefore, check dependencies of mods again to double-check which are really installed.
    mod_manager.check_mods_installed()?;

    Ok(mod_manager.get_mods()
        .map(|mod_info| ModModel::from(&*(**mod_info).borrow()))
        .collect())
}

fn get_core_mods_info(apk_version: &str, mod_manager: &ModManager, override_core_mod_url: Option<String>,
    res_cache: &ResCache, is_patched: bool) -> Result<Option<CoreModsInfo>> {
    // Fetch the core mods from the resources repo
    info!("Fetching core mod index");
    let core_mods = match mbf_res_man::external_res::fetch_core_mods(
        res_cache, override_core_mod_url) {
        Ok(mods) => mods,
        Err(JsonPullError::FetchError(fetch_err)) => {
            error!("Failed to fetch core mod index: assuming no internet connection: {fetch_err:?}");
            return Ok(None);
        },
        Err(JsonPullError::ParseError(err)) => return Err(err.into())
    };

    // Check that all core mods are installed with an appropriate version
    let all_core_mods_installed = match core_mods.get(apk_version) {
        Some(core_mods) => get_core_mods_install_status(&core_mods.mods, mod_manager),
        None => InstallStatus::Missing
    };

    let supported_versions: Vec<String> = core_mods.into_keys().filter(|version| {
        let mut iter = version.split('.');
        let _major = iter.next().unwrap();
        let minor = iter.next().unwrap();

        minor.parse::<i64>().expect("Invalid version in core mod index") >= 35
    }).collect();
    let is_version_supported = supported_versions.iter().any(|ver| ver == apk_version);

    // If the app is patched and not vanilla, then it's not possible to downgrade it even if a diff is available for the corresponding vanilla APK
    // Therefore, we can skip fetching the diff index, which will help startup times.

    let (downgrade_versions, newer_than_latest_diff) = if is_patched {
        // While technically newer_than_latest_diff is true, our app is patched already so we couldn't downgrade it
        // even if there was a diff available.
        (Vec::new(), false)
    }   else    {
        let diff_index = mbf_res_man::external_res::get_diff_index(res_cache)
        .context("Failed to get downgrading information")?;

        let newer_than_latest = is_version_newer_than_latest_diff(apk_version, &diff_index);
        (diff_index.into_iter()
            .filter(|diff| diff.from_version == apk_version)
            .map(|diff| diff.to_version)
            .collect(), newer_than_latest)
    };

    Ok(Some(CoreModsInfo {
        supported_versions,
        core_mod_install_status: all_core_mods_installed,
        downgrade_versions,
        is_awaiting_diff: newer_than_latest_diff && !is_version_supported
    }))
}

// Checks whether all the core mods in the provided slice are present within the mod manager given.
// Will give InstallStatus::Ready if all core mods are installed and up to date,
// InstallStatus::NeedUpdate if any core mods are out of date but all are installed, and InstallStatus::Missing if any 
// of the core mods are not installed or not even present.
fn get_core_mods_install_status(core_mods: &[CoreMod], mod_man: &ModManager) -> InstallStatus {
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
                }   else if mod_ref.manifest().version < core_mod.version {
                    warn!("Core mod {} is outdated, latest version: {}, installed version: {}", core_mod.id, core_mod.version, mod_ref.manifest().version);
                    outdated_core_mods = true;
                }
            },
            None => missing_core_mods = true
        }
    }

    if missing_core_mods {
        InstallStatus::Missing
    }   else if outdated_core_mods {
        InstallStatus::NeedUpdate
    }   else {
        InstallStatus::Ready
    }
}

fn try_parse_bs_ver_as_semver(version: &str) -> Option<semver::Version> {
    let version_segment = version.split('_').next().expect("Split iterator always returns at least one string");
    semver::Version::parse(version_segment).ok()
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
            Some(diff_version) => if sem_apk_ver <= diff_version {
                return false; // If it is, then this is not an "awaiting diff" situation
            },
            None => {}
        }
    }

    // APK version is not equal to or older than ANY of the versions with diffs
    // Hence, it is newer than the latest diff
    true
}

fn get_app_info() -> Result<Option<AppInfo>> {
    let apk_path = match crate::get_apk_path().context("Failed to find APK path")? {
        Some(path) => path,
        None => return Ok(None)
    };

    let apk_reader = std::fs::File::open(&apk_path)?;
    let mut apk = ZipFile::open(apk_reader).context("Failed to read APK as ZIP")?;

    let modloader = patching::get_modloader_installed(&mut apk)?;
    let obb_present = patching::check_obb_present()?;

    let (manifest_info, manifest_xml) = get_manifest_info_and_xml(&mut apk)?;
    Ok(Some(AppInfo {
        loader_installed: modloader,
        version: manifest_info.package_version,
        obb_present,
        path: apk_path,
        manifest_xml
    }))    
}

// Gets the version of the currently installed Beat Saber app.
// Gives an Err if the app is not currently installed.
// Intended as a more lightweight form of get_app_info that doesn't need to read the manifest
fn get_app_version_only() -> Result<String> {
    let dumpsys_output = Command::new("dumpsys")
        .args(["package", APK_ID])
        .output().context("Failed to invoke dumpsys")?;
    let dumpsys_stdout = String::from_utf8(dumpsys_output.stdout)
        .context("Failed to convert dumpsys output to UTF-8")?;

    let version_offset = match dumpsys_stdout.find("versionName=") {
        Some(offset) => offset,
        None => return Err(anyhow!("Beat Saber was not installed"))
    } + 12;

    let newline_offset = version_offset + dumpsys_stdout[version_offset..].find('\n')
        .ok_or(anyhow!("No newline after version name"))?;

    Ok(dumpsys_stdout[version_offset..newline_offset].trim().to_string())
}

fn axml_bytes_to_xml_string(bytes: &[u8]) -> Result<String> {
    let mut cursor = Cursor::new(bytes);
    let mut axml_reader = AxmlReader::new(&mut cursor)
        .context("File on manifests URI was invalid AXML. Report this!")?;

    let mut xml_output = Vec::new();
    let mut xml_writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(Cursor::new(&mut xml_output));

    axml::axml_to_xml(&mut xml_writer, &mut axml_reader).context("Failed to convert AXML to XML")?;

    Ok(String::from_utf8(xml_output)
        .expect("XML output should be valid UTF-8"))
}

fn get_manifest_info_and_xml(apk: &mut ZipFile<File>) -> Result<(ManifestInfo, String)> {
    let manifest = apk.read_file("AndroidManifest.xml").context("Failed to read manifest")?;

    // Decode various important information from the manifest
    let mut manifest_reader = Cursor::new(&manifest);
    let mut axml_reader = AxmlReader::new(&mut manifest_reader)?;
    let manifest_info = ManifestInfo::read(&mut axml_reader).context("Failed to read manifest")?;

    // Re-read the manifest as a full XML document.
    let xml_str = axml_bytes_to_xml_string(&manifest).context("Failed to convert manifest to XML")?;

    Ok((manifest_info, xml_str))
}

fn handle_import_mod_url(from_url: String) -> Result<Response> {
    std::fs::create_dir_all(DOWNLOADS_PATH)?;
    let download_path = Path::new(DOWNLOADS_PATH).join("import_from_url");

    info!("Downloading {}", from_url);
    let filename: Option<String> = downloads::download_file_with_attempts(&crate::get_dl_cfg(), &download_path, &from_url)?;
    
    // Attempt to import the downloaded file as a qmod, removing the temporary file if this fails.
    handle_import(&download_path, filename)
}

fn handle_import(from_path: impl AsRef<Path> + Debug, override_filename: Option<String>) -> Result<Response> {
    // Load the installed mods.
    let mut mod_manager = ModManager::new(&get_app_version_only()?);
    mod_manager.load_mods()?;

    let filename = match override_filename {
        Some(filename) => filename,
        None => from_path.as_ref().file_name().ok_or(anyhow!("No filename in {from_path:?}"))?
            .to_string_lossy()
            .to_string()
    };


    let path = from_path.as_ref().to_owned();
    info!("Attempting to import from {filename}");

    let file_ext = filename
        .split('.')
        .rev()
        .next()
        .ok_or(anyhow!("No file extension in filename {filename}"))?
        .to_string()
        .to_lowercase();

    let import_result = if file_ext == "qmod" {
        handle_import_qmod(mod_manager, path.clone())
    }   else if file_ext == "zip" {
        attempt_song_import(path.clone())
    }   else if file_ext == "dll" {
        // This is a PC mod file, so delete it and return this fact to the importer.
        std::fs::remove_file(path.clone()).context("Removing temporary upload file")?;
        Ok(ImportResultType::NonQuestModDetected)
    }  else  {
        attempt_file_copy(path.clone(), file_ext, mod_manager)
    };
    
    // Make sure to remove the temporary file in the case that importing the file failed.
    match import_result {
        Ok(result) => Ok(Response::ImportResult {
            result,
            used_filename: filename
        }),
        Err(err) => {
            match std::fs::remove_file(path) {
                Ok(_) => {},
                Err(err) => warn!("Failed to remove temporary file: {err}")
            }

            Err(err)
        }
    }
}

// Attempts to import the given path as a QMOD
// The file will be deleted if this results in a success.
fn handle_import_qmod(mut mod_manager: ModManager, from_path: PathBuf) -> Result<ImportResultType> {
    debug!("Loading {from_path:?} as a QMOD");
    let id = mod_manager.try_load_new_mod(std::fs::File::open(&from_path)?)?;
    std::fs::remove_file(from_path)?; // Delete temporary file.

    // If the mod loads successfully, we then need to *unload it* so that the file is not in use, then copy it to the mods directory.
    let installed_mods = get_mod_models(mod_manager)?; // Drops the mod_manager/the mod file handles

    Ok(ImportResultType::ImportedMod {
        imported_id: id,
        installed_mods
    })
}

// Attempts to copy the given file as a mod file copy.
// If returning Ok, the file will have been deleted.
fn attempt_file_copy(from_path: PathBuf, file_ext: String, mod_manager: ModManager) -> Result<ImportResultType> {
    for m in mod_manager.get_mods() {
        let mod_ref = (**m).borrow();
        match mod_ref.manifest()
            .copy_extensions.iter()
            .filter(|ext| ext.extension.eq_ignore_ascii_case(&file_ext))
            .next() 
        {
            Some(copy_ext) => {
                info!("Copying to {}", copy_ext.destination);
                let dest_folder = Path::new(&copy_ext.destination);
                std::fs::create_dir_all(dest_folder).context("Failed to create destination folder")?;
                let dest_path = dest_folder.join(from_path.file_name().unwrap());

                // Rename is not used as these may be in separate volumes.
                std::fs::copy(&from_path, &dest_path).context("Failed to copy file")?;
                std::fs::remove_file(&from_path)?;

                return Ok(ImportResultType::ImportedFileCopy {
                    copied_to: dest_path.to_string_lossy().to_string(),
                    mod_id: mod_ref.manifest().id.to_string()
                })
            },
            None => {}
        }
    }

    Err(anyhow!("File extension `.{}` was not recognised by any mod", file_ext))
}

fn attempt_song_import(from_path: PathBuf) -> Result<ImportResultType> {
    let song_handle = std::fs::File::open(&from_path)?;
    let mut zip = ZipFile::open(song_handle).context("Song was invalid ZIP file")?;

    if zip.contains_file("info.dat") || zip.contains_file("Info.dat") {
        let extract_path = Path::new(SONGS_PATH).join(from_path.file_stem().expect("Must have file stem"));

        if extract_path.exists() {
            std::fs::remove_dir_all(&extract_path).context("Failed to delete existing song")?;
        }

        std::fs::create_dir_all(&extract_path)?;
        let entry_names = zip.iter_entry_names()
            // TODO: This is not nice for performance
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        for entry_name in entry_names {
            zip.extract_file_to(&entry_name, extract_path.join(&entry_name))?;
        }

        drop(zip);
        std::fs::remove_file(from_path)?;
        Ok(ImportResultType::ImportedSong)
    }   else {
        Err(anyhow!("ZIP file was not a song; Unclear know how to import it"))
    }
}

fn handle_remove_mod(id: String) -> Result<Response> {
    let mut mod_manager = ModManager::new(&get_app_version_only()?);
    mod_manager.load_mods()?;
    mod_manager.remove_mod(&id)?;

    Ok(Response::Mods {
        installed_mods: get_mod_models(mod_manager)?
    })
}

fn handle_quick_fix(override_core_mod_url: Option<String>, wipe_existing_mods: bool) -> Result<Response> {
    let app_info = get_app_info()?
        .ok_or(anyhow!("Cannot quick fix when app is not installed"))?;
    let res_cache = crate::load_res_cache()?;

    let mut mod_manager = ModManager::new(&app_info.version);
    if wipe_existing_mods {
        info!("Wiping all existing mods");
        mod_manager.wipe_all_mods().context("Failed to wipe existing mods")?;
    }
    mod_manager.load_mods()?; // Should load no mods.

    // Reinstall missing core mods and overwrite the modloader with the one contained within the executable.
    install_core_mods(&res_cache, &mut mod_manager, app_info, override_core_mod_url)?;
    patching::install_modloader()?;
    Ok(Response::Mods {
        installed_mods: get_mod_models(mod_manager)?
    })
}

fn handle_fix_player_data() -> Result<Response> {
    patching::kill_app()?; // Kill app, in case it's still stuck in a hanging state

    let mut did_work = false;
    if Path::new(DATAKEEPER_PATH).exists() {
        info!("Fixing color scheme issues");
        data_fix::fix_colour_schemes(DATAKEEPER_PATH)?;
        did_work = true;
    }
    
    if Path::new(PLAYER_DATA_PATH).exists() {
        info!("Backing up player data");
        patching::backup_player_data()?;

        info!("Removing (potentially faulty) PlayerData.dat in game files");
        debug!("(removing {PLAYER_DATA_PATH})");
        std::fs::remove_file(PLAYER_DATA_PATH).context("Failed to delete faulty player data")?;
        if Path::new(PLAYER_DATA_BAK_PATH).exists() {
            std::fs::remove_file(PLAYER_DATA_BAK_PATH)?;
        }
        did_work = true;
    }   else {
        warn!("No player data found to \"fix\"");
    }

    Ok(Response::FixedPlayerData {
        existed: did_work
    })
}

fn handle_patch(downgrade_to: Option<String>,
    repatch: bool,
    manifest_mod: String,
    allow_no_core_mods: bool,
    override_core_mod_url: Option<String>,
    vr_splash_path: Option<String>) -> Result<Response> {
    let app_info = get_app_info()?
        .ok_or(anyhow!("Cannot patch when app not installed"))?;
    let res_cache = crate::load_res_cache()?;

    std::fs::create_dir_all(TEMP_PATH)?;

    // Either downgrade or just patch the current APK depending on the caller's choice.
    let patching_result = if let Some(to_version) = &downgrade_to {
        let diff_index = mbf_res_man::external_res::get_diff_index(&res_cache)
            .context("Failed to get diff index to downgrade")?;
        let version_diffs = diff_index.into_iter()
            .filter(|diff| diff.from_version == app_info.version && &diff.to_version == to_version)
            .next()
            .ok_or(anyhow!("No diff existed to go from {} to {}", app_info.version, to_version))?;

        patching::downgrade_and_mod_apk(Path::new(TEMP_PATH), &app_info, version_diffs, manifest_mod, vr_splash_path.as_deref(), &res_cache)
            .context("Failed to downgrade and patch APK")
    }   else {
        patching::mod_current_apk(Path::new(TEMP_PATH), &app_info, manifest_mod, repatch, vr_splash_path.as_deref(), &res_cache)
            .context("Failed to patch APK").map(|_| false) // Modding the currently installed APK will never remove DLC as they are restored automatically.
    };

    // No matter what, make sure that all temporary files are gone.
    std::fs::remove_dir_all(TEMP_PATH)?;
    if let Some(splash_path) = vr_splash_path {
        std::fs::remove_file(splash_path)?;
    }

    let removed_dlc = patching_result.context("Failed to patch game")?;

    patching::install_modloader().context("Failed to save modloader")?;

    let new_app_version = downgrade_to.unwrap_or(app_info.version);
    let mut mod_manager = ModManager::new(&new_app_version);
    
    if !repatch {
        info!("Wiping all existing mods");
        mod_manager.wipe_all_mods().context("Failed to wipe existing mods")?;
        mod_manager.load_mods()?; // Should load no mods.
    
        match install_core_mods(&res_cache, &mut mod_manager, get_app_info()?
            .ok_or(anyhow!("Beat Saber should be installed after patching"))?, override_core_mod_url) {
                Ok(_) => info!("Successfully installed all core mods"),
                Err(err) => if allow_no_core_mods {
                    warn!("Failed to install core mods: {err}")
                }   else    {
                    return Err(err).context("Failed to install core mods")
                }
            }
    }
    
    Ok(Response::Patched { installed_mods: get_mod_models(mod_manager)?, did_remove_dlc: removed_dlc })
}

/// Marks all of the mods with IDs matching mods in `core_mods` and all of their dependencies as core within the provided ModManager
fn mark_all_core_mods(mod_manager: &ModManager, core_mods: &[CoreMod]) {
    for core_mod in core_mods {
        mod_manager.set_mod_core(&core_mod.id);
    }
}

fn install_core_mods(res_cache: &ResCache, mod_manager: &mut ModManager, app_info: AppInfo, override_core_mod_url: Option<String>) -> Result<()> {
    info!("Preparing core mods");
    let core_mod_index = mbf_res_man::external_res::fetch_core_mods(
        &res_cache, override_core_mod_url)?;

    let core_mods = core_mod_index.get(&app_info.version)
        .ok_or(anyhow!("No core mods existed for {}", app_info.version))?;


    for core_mod in &core_mods.mods {
        // Check if there is already an existing mod.
        match mod_manager.get_mod(&core_mod.id) {
            Some(existing) => {
                let existing_ref = existing.borrow();
                if existing_ref.manifest().version >= core_mod.version {
                    info!("Core mod {} was already installed with new enough version: {}", core_mod.id, existing_ref.manifest().version);
                    continue;
                }
            },
            None => {}
        }

        info!("Downloading {} v{}", core_mod.id, core_mod.version);

        let core_mod_vec = downloads::download_to_vec_with_attempts(&crate::get_dl_cfg(), &core_mod.download_url)
            .context("Failed to download core mod")?;
        let result = mod_manager.try_load_new_mod(Cursor::new(core_mod_vec));
        // Delete the temporary file either way
        result?;
        
    }

    info!("Installing core mods");
    mod_manager.load_mods().context("Failed to load core mods - is one invalid? If so, this is a BIG problem")?;
    for core_mod in &core_mods.mods {
        mod_manager.install_mod(&core_mod.id)?;
    }
    mark_all_core_mods(&mod_manager, &core_mods.mods);
    
    Ok(())
}

fn handle_get_downgraded_manifest(version: String) -> Result<Response> {
    info!("Downloading manifest AXML file");
    let manifest_bytes = mbf_res_man::external_res::get_manifest_axml(mbf_res_man::default_agent::get_agent(), version)
        .context("Failed to GET AndroidManifest.xml")?;
    info!("Converting into readable XML");
    let manifest_xml = axml_bytes_to_xml_string(&manifest_bytes)?;

    Ok(Response::DowngradedManifest { manifest_xml })
}