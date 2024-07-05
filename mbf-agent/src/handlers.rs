use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::manifest::ManifestInfo;
use crate::{axml, data_fix, download_file_with_attempts, external_res, DATAKEEPER_PATH, DOWNLOADS_PATH, PLAYER_DATA_BAK_PATH, PLAYER_DATA_PATH, SONGS_PATH, TEMP_PATH};
use crate::{axml::AxmlReader, patching, zip::ZipFile};
use crate::external_res::{get_diff_index, JsonPullError};
use crate::mod_man::ModManager;
use crate::requests::{AppInfo, CoreModsInfo, ImportResultType, ModModel, Request, Response};
use anyhow::{anyhow, Context, Result};
use log::{error, info, warn};
use xml::EmitterConfig;


pub fn handle_request(request: Request) -> Result<Response> {
    match request {
        Request::GetModStatus { override_core_mod_url } => handle_get_mod_status(override_core_mod_url),
        Request::Patch { downgrade_to , remodding, manifest_mod, allow_no_core_mods, override_core_mod_url} => 
            handle_patch(downgrade_to, remodding, manifest_mod, allow_no_core_mods, override_core_mod_url),
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
    let mut mod_manager = ModManager::new();
    mod_manager.load_mods().context("Failed to load installed mods")?;

    for (id, new_status) in statuses {
        let mod_rc = match mod_manager.get_mod(&id) {
            Some(m) => m,
            None => {
                error!("Mod with ID {id} did not exist");
                continue;
            }
        };

        let already_installed = mod_rc.borrow().installed();
        if new_status && !already_installed {
            match mod_manager.install_mod(&id) {
                Ok(_) => info!("Installed {id}"),
                Err(err) => error!("Failed to install {id}: {err}")
            }
        }   else if !new_status && already_installed {
            match mod_manager.uninstall_mod(&id) {
                Ok(_) => info!("Uninstalled {id}"),
                Err(err) => error!("Failed to install {id}: {err}")
            }
        }
        
    }

    Ok(Response::Mods {
        installed_mods: get_mod_models(mod_manager),
    })
}

fn handle_get_mod_status(override_core_mod_url: Option<String>) -> Result<Response> {
    info!("Loading installed mods");

    let mut mod_manager = ModManager::new();
    mod_manager.load_mods().context("Failed to load installed mods")?;

    info!("Searching for Beat Saber app");
    let app_info = get_app_info()?;
    let core_mods = match &app_info {
        Some(app_info) => get_core_mods_info(&app_info.version, &mod_manager, override_core_mod_url)?,
        None => {
            warn!("Beat Saber is not installed!");
            None
        }
    };

    Ok(Response::ModStatus { 
        app_info,
        core_mods,
        modloader_present: patching::get_modloader_path()?.exists(),
        installed_mods: get_mod_models(mod_manager)
    })
}

fn get_mod_models(mod_manager: ModManager) -> Vec<ModModel> {
    mod_manager.get_mods()
        .map(|mod_info| ModModel::from(&*(**mod_info).borrow()))
        .collect()
}

fn get_core_mods_info(apk_version: &str, mod_manager: &ModManager, override_core_mod_url: Option<String>) -> Result<Option<CoreModsInfo>> {
    // Fetch the core mods from the resources repo
    info!("Fetching core mod index");
    let core_mods = match crate::external_res::fetch_core_mods(override_core_mod_url) {
        Ok(mods) => mods,
        Err(JsonPullError::FetchError(_)) => return Ok(None),
        Err(JsonPullError::ParseError(err)) => return Err(err)
    };

    // Check that all core mods are installed with an appropriate version
    let all_core_mods_installed = match core_mods.get(apk_version) {
        Some(core_mods) => core_mods.mods
            .iter()
            .all(|core_mod| match mod_manager.get_mod(&core_mod.id) {
                None => false,
                Some(installed_version) => {
                    let installed_ref = installed_version.borrow();
                    installed_ref.manifest().version >= core_mod.version && installed_ref.installed()
                }
            }),
        None => false
    };
    info!("All core mods installed: {}", all_core_mods_installed);

    let supported_versions: Vec<String> = core_mods.into_keys().filter(|version| {
        let mut iter = version.split('.');
        let _major = iter.next().unwrap();
        let _minor = iter.next().unwrap();

        _minor.parse::<i64>().expect("Invalid version in core mod index") >= 35
    }).collect();

    let downgrade_versions: Vec<String> = get_diff_index()
        .context("Failed to get downgrading information")?
        .into_iter()
        .filter(|diff| diff.from_version == apk_version)
        .map(|diff| diff.to_version)
        .collect();

    Ok(Some(CoreModsInfo {
        supported_versions,
        all_core_mods_installed,
        downgrade_versions
    }))
}

fn get_app_info() -> Result<Option<AppInfo>> {
    let apk_path = match crate::get_apk_path().context("Failed to find APK path")? {
        Some(path) => path,
        None => return Ok(None)
    };

    let apk_reader = std::fs::File::open(&apk_path)?;
    let mut apk = ZipFile::open(apk_reader).context("Failed to read APK as ZIP")?;

    let modloader = patching::get_modloader_installed(&mut apk)?;

    let (manifest_info, manifest_xml) = get_manifest_info_and_xml(&mut apk)?;
    Ok(Some(AppInfo {
        loader_installed: modloader,
        version: manifest_info.package_version,
        path: apk_path,
        manifest_xml
    }))    
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
    let filename: Option<String> = download_file_with_attempts(&download_path, &from_url)?;
    
    // Attempt to import the downloaded file as a qmod, removing the temporary file if this fails.
    handle_import(&download_path, filename)
}

fn handle_import(from_path: impl AsRef<Path> + Debug, override_filename: Option<String>) -> Result<Response> {
    // Load the installed mods.
    let mut mod_manager = ModManager::new();
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
        .to_string();

    let import_result = if file_ext == "qmod" {
        handle_import_qmod(mod_manager, path.clone())
    }   else if file_ext == "zip" {
        attempt_song_import(path.clone())
    }   else    {
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
    info!("Loading {from_path:?} as a QMOD");
    let id = mod_manager.try_load_new_mod(from_path.clone())?;

    // A bit of a hack here: when installing mods, 
    // we don't want to copy the unvalidated mod to the QMODs directory,
    // so we load it from a temporary directory.

    // If the mod loads successfully, we then need to *unload it* so that the file is not in use, then copy it to the mods directory.
    let new_path = mod_manager.get_unique_mod_path(&id);
    let installed_mods = get_mod_models(mod_manager); // Drops the mod_manager/the mod file handles

    // Copy to a new patch in the mods directory
    std::fs::copy(&from_path, new_path)?;
    std::fs::remove_file(from_path)?;

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
    let mut mod_manager = ModManager::new();
    mod_manager.load_mods()?;
    mod_manager.remove_mod(&id)?;

    Ok(Response::Mods {
        installed_mods: get_mod_models(mod_manager)
    })
}

fn handle_quick_fix(override_core_mod_url: Option<String>, wipe_existing_mods: bool) -> Result<Response> {
    let app_info = get_app_info()?
        .ok_or(anyhow!("Cannot quick fix when app is not installed"))?;

    let mut mod_manager = ModManager::new();
    if wipe_existing_mods {
        info!("Wiping all existing mods");
        mod_manager.wipe_all_mods().context("Failed to wipe existing mods")?;
    }
    mod_manager.load_mods()?; // Should load no mods.

    // Reinstall missing core mods and overwrite the modloader with the one contained within the executable.
    install_core_mods(&mut mod_manager, app_info, override_core_mod_url)?;
    patching::install_modloader()?;
    Ok(Response::Mods {
        installed_mods: get_mod_models(mod_manager)
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

        info!("Removing (potentially faulty) PlayerData.dat at {}", PLAYER_DATA_PATH);
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
    override_core_mod_url: Option<String>) -> Result<Response> {
    let app_info = get_app_info()?
        .ok_or(anyhow!("Cannot patch when app not installed"))?;

    std::fs::create_dir_all(TEMP_PATH)?;

    // Either downgrade or just patch the current APK depending on the caller's choice.
    let patching_result = if let Some(to_version) = downgrade_to {
        let diff_index = get_diff_index()
            .context("Failed to get diff index to downgrade")?;
        let version_diffs = diff_index.into_iter()
            .filter(|diff| diff.from_version == app_info.version && diff.to_version == to_version)
            .next()
            .ok_or(anyhow!("No diff existed to go from {} to {}", app_info.version, to_version))?;

        patching::downgrade_and_mod_apk(Path::new(TEMP_PATH), &app_info, version_diffs, manifest_mod)
            .context("Failed to downgrade and patch APK")
    }   else {
        patching::mod_current_apk(Path::new(TEMP_PATH), &app_info, manifest_mod, repatch)
            .context("Failed to patch APK")
    };

    // No matter what, make sure that all temporary files are gone.
    std::fs::remove_dir_all(TEMP_PATH)?;

    if let Err(err) = patching_result {
        return Err(err).context("Failed to patch")
    }

    patching::install_modloader().context("Failed to save modloader")?;

    let mut mod_manager = ModManager::new();
    
    if !repatch {
        info!("Wiping all existing mods");
        mod_manager.wipe_all_mods().context("Failed to wipe existing mods")?;
        mod_manager.load_mods()?; // Should load no mods.
    
        match install_core_mods(&mut mod_manager, get_app_info()?
            .ok_or(anyhow!("Beat Saber should be installed after patching"))?, override_core_mod_url) {
                Ok(_) => info!("Successfully installed all core mods"),
                Err(err) => if allow_no_core_mods {
                    warn!("Failed to install core mods: {err}")
                }   else    {
                    return Err(err).context("Failed to install core mods")
                }
            }
    }
    
    Ok(Response::Mods { installed_mods: get_mod_models(mod_manager) })
}

fn install_core_mods(mod_manager: &mut ModManager, app_info: AppInfo, override_core_mod_url: Option<String>) -> Result<()> {
    info!("Preparing core mods");
    let core_mod_index = crate::external_res::fetch_core_mods(override_core_mod_url)?;

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
        let save_path = mod_manager.mods_path().as_ref()
            .join(format!("{}-v{}-CORE.qmod", core_mod.id, core_mod.version));

        download_file_with_attempts(&save_path, &core_mod.download_url).context("Failed to download core mod")?;
        mod_manager.try_load_new_mod(save_path)?;
        
    }

    info!("Installing core mods");
    mod_manager.load_mods().context("Failed to load core mods - is one invalid? If so, this is a BIG problem")?;
    for core_mod in &core_mods.mods {
        mod_manager.install_mod(&core_mod.id)?;
    }
    
    Ok(())
}

fn handle_get_downgraded_manifest(version: String) -> Result<Response> {
    info!("Downloading manifest AXML file");
    let manifest_bytes = external_res::get_manifest_axml(version)
        .context("Failed to GET AndroidManifest.xml")?;
    info!("Converting into readable XML");
    let manifest_xml = axml_bytes_to_xml_string(&manifest_bytes)?;

    Ok(Response::DowngradedManifest { manifest_xml })
}