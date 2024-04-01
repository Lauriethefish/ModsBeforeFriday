use semver::Version;
use serde::{Deserialize, Serialize};

use crate::mod_man::ModInfo;

#[derive(Serialize)]
pub struct AppInfo {
    pub is_modded: bool,
    pub version: String
}


#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    /// Gathers several pieces of data to check that the installation is modded appropriately, including:
    /// - Whether the APK is patched
    /// - The APK version
    /// - The known installed mods and whether all their necessary files exist.
    /// - The core mods that need to be installed.
    /// - Whether the modloader is in the correct place
    GetModStatus,
    /// Checks which mods are installed and whether the files needed for each mod are copied correctly.
    /// If a new mod is pushed via ADB, this will attempt to load the mod, and will relay back any problems with the mod.
    GetMods,
    /// Installs or uninstalls any number of mods.
    /// This will also attempt to download and install dependencies, upgrade dependencies and will uninstall any
    /// depending mods of mods that have been disabled.
    /// 
    /// If two changes conflict, one of the changes will be kept and an error log will be provided.
    SetModsEnabled(ModAction),

    /// - Patches Beat Saber to add support for modloaders.
    /// - Saves the modloader to the appropriate locaiton on the Quest.
    /// - Wipes any existing mods.
    /// - Installs the core mods for the current version.
    /// If the app is already patched, it will not be patched again.
    Patch
}

#[derive(Serialize, Deserialize)]
pub struct ModAction {
    /// The mods that will be disabled in the request.
    to_uninstall: Vec<String>,
    /// The mods that will be enabled in the request.
    to_install: Vec<String>,
}

#[derive(Serialize)]
pub struct CoreModsInfo {
    /// All of the Beat Saber versions with core mods using Scotland2
    pub supported_versions: Vec<String>,
    pub all_core_mods_installed: bool
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum Response {
    ModStatus {
        // None if Beat Saber is not installed.
        app_info: Option<AppInfo>,
        
        // All mods currently found in the mods folder
        installed_mods: Vec<ModModel>,

        // Information about the status of core mods.
        // None if an internet connection could not be established.
        core_mods: Option<CoreModsInfo>,

        modloader_present: bool
    },
    Mods {
        installed_mods: Vec<ModModel>
    },
    ModInstallResult {
        installed_mods: Vec<ModModel>,
        // A log of dependency resolution/which mods were installed and uninstalled.
        install_log: String,
        // True if all the mods in the to_install part of ModAction are now installed, and the ones in to_uninstall are now uninstalled.
        full_success: bool
    },
    Patched
}

/// The trimmed version of the ModInfo type that is sent to the web client.
#[derive(Serialize, Deserialize)]
pub struct ModModel {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub description: Option<String>,
    pub is_enabled: bool
}

impl From<ModInfo> for ModModel {
    fn from(value: ModInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
            version: value.version,
            description: value.description,
            is_enabled: value.is_enabled
        }
    }
}