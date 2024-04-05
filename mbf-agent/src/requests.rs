use std::collections::HashMap;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::mod_man::Mod;

#[derive(Serialize)]
pub struct AppInfo {
    pub loader_installed: Option<ModLoader>,
    pub version: String,
    #[serde(skip_serializing)]
    pub path: String
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
    /// Installs or uninstalls any number of mods.
    /// This will also attempt to download and install dependencies, upgrade dependencies and will uninstall any
    /// depending mods of mods that have been disabled.
    /// 
    /// Returns a `Mods` response.
    SetModsEnabled {
        statuses: HashMap<String, bool>
    },
    // TODO: Make these lists to allow importing multiple mods at once?

    /// Removes the mod with the given ID, which will uninstall dependant mods.
    /// Returns a Mods message containing the mods now installed.
    RemoveMod {
        id: String
    },
    /// Imports a mod from the given path on the quest.
    /// Returns an ImportedMod message containing the mods now installed, and the ID of the one that was imported.
    Import {
        from_path: String
    },
    /// - Patches Beat Saber to add support for modloaders. (will not patch again if the app is already modded)
    /// - Saves the modloader to the appropriate locatioon on the Quest.
    /// - Wipes any existing mods.
    /// - Installs the core mods for the current version.
    /// Returns a `Mods` response to update the frontend with the newly installed core mods.
    Patch,

    /// Reinstalls any core mods that are misssing/out of date and overwrites the modloader in case it is corrupt.
    /// Should fix most issues with any installation.
    /// Returns a `Mods` response containing the newly installed mods.
    QuickFix,
}

#[derive(Serialize)]
pub struct CoreModsInfo {
    /// All of the Beat Saber versions with core mods using Scotland2
    pub supported_versions: Vec<String>,
    pub all_core_mods_installed: bool
}

#[derive(Serialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace
}

#[derive(Serialize)]
pub enum ModLoader {
    Scotland2,
    QuestLoader,
    Unknown
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
    ImportedMod {
        installed_mods: Vec<ModModel>,
        imported_id: String  
    },
    // Sent to relay progress information during the modding process.
    // This will NOT be the final message sent.
    LogMsg {
        message: String,
        level: LogLevel
    }
}

/// The trimmed version of the ModInfo type that is sent to the web client.
#[derive(Serialize, Deserialize)]
pub struct ModModel {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub game_version: Option<String>,
    pub description: Option<String>,
    pub is_enabled: bool
}

impl From<&Mod> for ModModel {
    fn from(value: &Mod) -> Self {
        Self {
            id: value.manifest().id.clone(),
            name: value.manifest().name.clone(),
            version: value.manifest().version.clone(),
            game_version: value.manifest().package_version.clone(),
            description: value.manifest().description.clone(),
            is_enabled: value.installed()
        }
    }
}