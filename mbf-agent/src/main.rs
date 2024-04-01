mod requests;
mod zip;
mod manifest;
mod axml;
mod patching;
mod bmbf_res;
mod mod_man;

use std::{io::{BufRead, BufReader, Cursor}, process::Command};

use axml::AxmlReader;
use bmbf_res::CoreModsError;
use lockfile::Lockfile;
use manifest::ManifestInfo;
use mod_man::ModManager;
use requests::{AppInfo, CoreModsInfo, ModAction, ModModel, Request, Response};
use anyhow::{anyhow, Context, Result};

const APK_ID: &str = "com.beatgames.beatsaber";
const LOCKFILE_PATH: &str = "/data/local/tmp/mbf.lock";

fn handle_request(request: Request) -> Result<Response> {
    match request {
        Request::GetModStatus => handle_get_mod_status(),
        Request::Patch => handle_patch(),
        Request::SetModsEnabled(action) => run_mod_action(action),
        _ => todo!()
    }
}

fn run_mod_action(action: ModAction) -> Result<Response> {
    let mut mod_manager = mod_man::ModManager::new();
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
    let mut mod_manager = mod_man::ModManager::new();
    mod_manager.load_mods().context("Failed to load installed mods")?;

    let app_info = get_app_info()?;
    let core_mods = match &app_info {
        Some(app_info) => get_core_mods_info(&app_info.version, &mod_manager)?,
        None => None
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
    let core_mods = match bmbf_res::fetch_core_mods() {
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
    let apk_path = match get_apk_path().context("Failed to find APK path")? {
        Some(path) => path,
        None => return Ok(None)
    };

    let apk_reader = std::fs::File::open(apk_path)?;
    let mut apk = zip::ZipFile::open(apk_reader).context("Failed to read APK as ZIP")?;

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

    Ok(Response::Patched)
}

pub fn get_apk_path() -> Result<Option<String>> {
    let pm_output = Command::new("pm")
        .args(["path", APK_ID])
        .output()
        .context("Failed to get APK path")?;
    if 8 > pm_output.stdout.len() {
        // App not installed
        Ok(None)
    }   else {
        Ok(Some(std::str::from_utf8(pm_output.stdout.split_at(8).1)?
            .trim_end()
            .to_owned()))
    }
}

fn main() -> Result<()> {
    let _lockfile = Lockfile::create(LOCKFILE_PATH).context("Failed to obtain lockfile - is MBF already open?")?;

    let mut reader = BufReader::new(std::io::stdin());
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let req: Request = serde_json::from_str(&line)?;

    let resp = handle_request(req)?;
    serde_json::to_writer(std::io::stdout(), &resp)?;
    Ok(())
}