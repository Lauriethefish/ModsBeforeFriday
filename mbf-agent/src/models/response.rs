//! Models used for communication *from the backend back to the frontend*

use serde::{Deserialize, Serialize};

use crate::mod_man;

#[derive(Serialize)]
pub struct AppInfo {
    pub loader_installed: Option<ModLoader>,
    pub obb_present: bool,
    #[serde(skip_serializing)]
    pub path: String,
    pub version: String,
    pub manifest_xml: String,
}

#[derive(Serialize)]
pub struct CoreModsInfo {
    /// All of the Beat Saber versions with core mods using Scotland2 are keys in this HashMap
    pub supported_versions: Vec<String>,
    /// The versions of Beat Saber that can be reached by downgrading the game.
    pub downgrade_versions: Vec<String>,
    /// True only if the Beat Saber version does not support mods, and the latest diff available in the diff index
    /// is intended to start with a Beat Saber version older than the current version.
    /// In these circumstances, the user needs to wait for a diff to be generated.
    pub is_awaiting_diff: bool,
    pub core_mod_install_status: InstallStatus,
}

/// An enum that represents whether a particular piece of the modded game is:
#[derive(Copy, Clone, Serialize)]
pub enum InstallStatus {
    /// Installed and up to date
    Ready,
    /// Installed but not up to date
    NeedUpdate,
    /// Not installed
    Missing,
}

#[derive(Serialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Serialize)]
pub enum ModLoader {
    Scotland2,
    QuestLoader,
    Unknown,
}

/// What type of file a file was imported as, and details about the resulting file.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImportResultType {
    ImportedMod {
        installed_mods: Vec<ModModel>,
        imported_id: String,
    },
    ImportedFileCopy {
        // The full path where the file was copied to.
        copied_to: String,
        // The mod ID that the file copy belonged to
        mod_id: String,
    },
    ImportedSong,
    // A non-quest mod was detected (i.e. `.DLL`) and so the import failed.
    NonQuestModDetected,
}

/// The trimmed version of the ModInfo type that is sent to the web client.
#[derive(Serialize, Deserialize)]
pub struct ModModel {
    pub id: String,
    pub name: String,
    pub version: semver::Version,
    pub game_version: Option<String>,
    pub description: Option<String>,
    pub is_enabled: bool,
    // True if the mod is core or if it is a required dependency of another core mod (potentially indirectly.)
    pub is_core: bool,
}

impl From<&mod_man::Mod> for ModModel {
    fn from(value: &mod_man::Mod) -> Self {
        Self {
            id: value.manifest().id.clone(),
            name: value.manifest().name.clone(),
            version: value.manifest().version.clone(),
            game_version: value.manifest().package_version.clone(),
            description: value.manifest().description.clone(),
            is_enabled: value.installed(),
            is_core: value.is_core(),
        }
    }
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

        modloader_install_status: InstallStatus,
    },
    Mods {
        installed_mods: Vec<ModModel>,
    },
    ModSyncResult {
        // The new state of the installed mods after the operation
        installed_mods: Vec<ModModel>,
        // If any of the mods failed to install/uninstall, this will be Some with a string
        // containing a list of the errors generated.
        failures: Option<String>,
    },
    Patched {
        installed_mods: Vec<ModModel>,
        did_remove_dlc: bool,
    },
    ImportResult {
        result: ImportResultType, // The result of importing the file.
        used_filename: String, // The filename that was actually used to determine how to import the mod.
    },
    // Sent to relay progress information during the modding process.
    // This will NOT be the final message sent.
    LogMsg {
        message: String,
        level: LogLevel,
    },
    FixedPlayerData {
        // True if a PlayerData.dat existed to fix, false if the request did nothing.
        existed: bool,
    },
    DowngradedManifest {
        manifest_xml: String,
    },
}
