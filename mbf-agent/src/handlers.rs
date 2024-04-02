use std::{fs::OpenOptions, io::Cursor};

use crate::{axml::AxmlReader, patching, zip::ZipFile};
use crate::bmbf_res::CoreModsError;
use crate::manifest::ManifestInfo;
use crate::mod_man::ModManager;
use crate::requests::{AppInfo, CoreModsInfo, ModAction, ModModel, Request, Response};
use anyhow::{anyhow, Context, Result};
use log::{info, warn};

pub fn handle_request(request: Request) -> Result<Response> {
    match request {
        Request::GetModStatus => handle_get_mod_status(),
        Request::Patch => handle_patch(),
        Request::SetModsEnabled(action) => run_mod_action(action),
        _ => todo!()
    }
}

fn run_mod_action(action: ModAction) -> Result<Response> {
    let mut mod_manager = ModManager::new();
    mod_manager.load_mods().context("Failed to load installed mods")?;

    let mut log = String::new();

    for to_install in action.to_install {
       match mod_manager.install_mod(&to_install) {
            Ok(_) => log.push_str(&format!("Installed {to_install}\n")),
            Err(err) => log.push_str(&format!("Failed to install {to_install}: {err}\n"))
       }
    }

    for to_uninstall in action.to_uninstall {
        match mod_manager.install_mod(&to_uninstall) {
             Ok(_) => log.push_str(&format!("Uninstalled {to_uninstall}\n")),
             Err(err) => log.push_str(&format!("Failed to install {to_uninstall}: {err}\n"))
        }
     }

    Ok(Response::ModInstallResult {
        installed_mods: get_mod_models(mod_manager),
        install_log: log,
        full_success: true // TODO
    })
}

fn handle_get_mod_status() -> Result<Response> {
    info!("Loading installed mods");
    let mut mod_manager = ModManager::new();
    mod_manager.load_mods().context("Failed to load installed mods")?;

    info!("Searching for Beat Saber app");
    let app_info = get_app_info()?;
    let core_mods = match &app_info {
        Some(app_info) => get_core_mods_info(&app_info.version, &mod_manager)?,
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
    mod_manager.into_mods()
        .map(|mod_info| ModModel::from(mod_info))
        .collect()
}

fn get_core_mods_info(apk_version: &str, mod_manager: &ModManager) -> Result<Option<CoreModsInfo>> {
    // Fetch the core mods from the resources repo
    info!("Fetching core mod index");
    let core_mods = match crate::bmbf_res::fetch_core_mods() {
        Ok(mods) => mods,
        Err(CoreModsError::FetchError(_)) => return Ok(None),
        Err(CoreModsError::ParseError(err)) => return Err(err)
    };

    // Check that all core mods are installed with an appropriate version
    let all_core_mods_installed = match core_mods.get(apk_version) {
        Some(core_mods) => core_mods.mods
            .iter()
            .all(|core_mod| match mod_manager.get_mod(&core_mod.id) {
                None => false,
                Some(installed_version) => installed_version.manifest().version >= core_mod.version
            }),
        None => false
    };

    let supported_versions: Vec<String> = core_mods.into_keys().filter(|version| {
        let mut iter = version.split('.');
        let _major = iter.next().unwrap();
        let _minor = iter.next().unwrap();

        _minor.parse::<i64>().expect("Invalid version in core mod index") >= 35
    }).collect();

    Ok(Some(CoreModsInfo {
        supported_versions,
        all_core_mods_installed
    }))
}

fn get_app_info() -> Result<Option<AppInfo>> {
    let apk_path = match crate::get_apk_path().context("Failed to find APK path")? {
        Some(path) => path,
        None => return Ok(None)
    };

    let apk_reader = std::fs::File::open(apk_path)?;
    let mut apk = ZipFile::open(apk_reader).context("Failed to read APK as ZIP")?;

    let is_modded = apk
        .iter_entry_names()
        .any(|entry| entry.contains("modded"));

    let manifest = match apk.read_file("AndroidManifest.xml")? {
        Some(contents) => contents,
        None => return Err(anyhow!("Manifest not found in APK"))
    };
    let mut manifest_reader = Cursor::new(manifest);

    let mut axml_reader = AxmlReader::new(&mut manifest_reader)?;
    let info = ManifestInfo::read(&mut axml_reader)?;

    Ok(Some(AppInfo {
        is_modded,
        version: info.package_version
    }))    
}

fn handle_patch() -> Result<Response> {
    patching::mod_current_apk().context("Failed to patch APK")?;
    patching::install_modloader().context("Failed to save modloader")?;

    let mut mod_manager = ModManager::new();
    info!("Wiping all existing mods");
    mod_manager.wipe_all_mods().context("Failed to wipe existing mods")?;

    install_core_mods(&mut mod_manager, get_app_info()?
        .expect("Beat Saber should be installed after patching"))?;

    mod_manager.load_mods().context("Failed to load core mods - is one invalid? If so, this is a BIG problem")?;
    

    Ok(Response::Patched)
}

fn install_core_mods(mod_manager: &mut ModManager, app_info: AppInfo) -> Result<()> {
    info!("Installing core mods");
    let core_mod_index = match crate::bmbf_res::fetch_core_mods() {
        Ok(core_mods) => core_mods,
        // We do not care if it was fetching or parsing: this should succeed because otherwise the frontend would not permit us to get to this point.
        Err(err) => return Err(match err {
            CoreModsError::FetchError(e) => e,
            CoreModsError::ParseError(e) => e
        })
    };

    let core_mods = core_mod_index.get(&app_info.version)
        .ok_or(anyhow!("No core mods existed for {}", app_info.version))?;

    for core_mod in &core_mods.mods {
        info!("Downloading {} v{}", core_mod.id, core_mod.version);
        let save_path = mod_manager.mods_path().as_ref()
            .join(format!("{}-v{}-CORE.qmod", core_mod.id, core_mod.version));

        let mut resp_body = ureq::get(&core_mod.download_url)
            .call()
            .context("Failed to download core mod: is the connection working")?
            .into_reader();

        let mut writer = OpenOptions::new()
            .write(true)
            .create(true)
            .open(save_path).context("Failed to create core mod QMOD file")?;

        std::io::copy(&mut resp_body, &mut writer).context("Failed to download core mod: was the WiFi connection lost?")?;
    }

    info!("Loading and installing core mods");
    mod_manager.load_mods().context("Failed to load core mods - is one invalid? If so, this is a BIG problem")?;
    for core_mod in &core_mods.mods {
        mod_manager.install_mod(&core_mod.id)?;
    }
    
    Ok(())
}