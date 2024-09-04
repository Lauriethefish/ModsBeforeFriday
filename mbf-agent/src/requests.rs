use std::collections::HashMap;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::mod_man::Mod;

#[derive(Serialize)]
pub struct AppInfo {
    pub loader_installed: Option<ModLoader>,
    pub obb_present: bool,
    #[serde(skip_serializing)]
    pub path: String,
    pub version: String,
    pub manifest_xml: String
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
    GetModStatus {
        // If not null, this specifies a core mod JSON to use instead of the default core mods source.
        // This is useful for developers testing a core mod update.
        override_core_mod_url: Option<String>
    },
    /// Installs or uninstalls any number of mods.
    /// This will also attempt to download and install dependencies, upgrade dependencies and will uninstall any
    /// depending mods of mods that have been disabled.
    /// 
    /// Returns a `ModSyncResult` response.
    SetModsEnabled {
        statuses: HashMap<String, bool>
    },
    
    // TODO: Make these lists to allow importing multiple mods at once?

    /// Removes the mod with the given ID, which will uninstall dependant mods.
    /// Returns a Mods message containing the mods now installed.
    RemoveMod {
        id: String
    },
    /// Imports a mod or file copy from the given path on the quest.
    /// Returns an ImportedMod message containing the mods now installed, and the ID of the one that was imported, if importing a mod.
    /// Returns an ImportedFileCopy message if the file type was copied by a mod copy extension.
    /// Returns an ImportedSong message if the file type was copied to the songs folder.
    Import {
        from_path: String
    },
    /// Downloads the file from the given URL and then attempts to import it as a mod (only).
    /// Returns an ImportedMod message.
    ImportUrl {
        from_url: String,
    },

    /// - Patches Beat Saber to add support for modloaders.
    /// - Optionally, downgrades the game to the given version if downgrade_to is Some
    /// - Saves the modloader to the appropriate locatioon on the Quest.
    /// - Wipes any existing mods.
    /// - Installs the core mods for the current version.
    /// Returns a `Mods` response to update the frontend with the newly installed core mods.
    Patch {
        downgrade_to: Option<String>,
        // The contents of the manifest of the patched app, as XML
        // The frontend is reponsible for adding the necessary permissions and features here.
        manifest_mod: String,
        // The complete path to a PNG file to be used as the vr_splash.png file within the APK
        // This is the splash screen that appears when starting the game in headset.
        // This file will always be automatically deleted after patching, whether it succeeded or failed.
        vr_splash_path: Option<String>,
        // If this is true, patching will skip adding the modloader and libunity.so and will ONLY change permissions.
        // Patching will also not attempt to reinstall core mods.
        //
        // TODO: in the future, it might make sense for remodding to detect a change in the libunity.so (harder) 
        // or libmainloader (easier) so that these can be easily updated.
        remodding: bool,
        // If this is true, patching will not be failed if core mods cannot be found for the version.
        allow_no_core_mods: bool,
        // If not null, this specifies a core mod JSON to use instead of the default core mods source.
        // This is useful for developers testing a core mod update.
        override_core_mod_url: Option<String>
    },

    // Attempts to fix a blackscreen issue by removing PlayerData.dat from `/sdcard/...../files/`.
    // (and copying it to /sdcard/ModsBeforeFriday so it isn't lost. It will also be copied to the datakeeper directory iff there isn't already one there)
    // (This occurs when the permissions set by MBF copying the file lead to the game not being able to open it, typically on Quest 3,
    // unfortunately chmod 777 doesn't seem to fix the issue.)
    // Gives a `FixedPlayerData` response.
    FixPlayerData,
    /// Gets a copy of the AndroidManifest.xml for the given Beat Saber version, converted from AXML into an XML string.
    GetDowngradedManifest {
        version: String
    },
    /// Reinstalls any core mods that are misssing/out of date and overwrites the modloader in case it is corrupt.
    /// Should fix most issues with any installation.
    /// Returns a `Mods` response containing the newly installed mods.
    QuickFix {
        // If not null, this specifies a core mod JSON to use instead of the default core mods source.
        // This is useful for developers testing a core mod update.
        override_core_mod_url: Option<String>,
        // If true, this request will delete ALL mods before reinstalling only the core mods.
        wipe_existing_mods: bool
    },
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
    pub core_mod_install_status: InstallStatus
}

/// An enum that represents whether a particular piece of the modded game is:
#[derive(Copy, Clone, Serialize)]
pub enum InstallStatus {
    /// Installed and up to date
    Ready,
    /// Installed but not up to date
    NeedUpdate,
    /// Not installed
    Missing
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

        modloader_install_status: InstallStatus
    },
    Mods {
        installed_mods: Vec<ModModel>
    },
    ModSyncResult {
        // The new state of the installed mods after the operation
        installed_mods: Vec<ModModel>,
        // If any of the mods failed to install/uninstall, this will be Some with a string
        // containing a list of the errors generated.
        failures: Option<String>
    },
    Patched {
        installed_mods: Vec<ModModel>,
        did_remove_dlc: bool
    },
    ImportResult {
        result: ImportResultType, // The result of importing the file.
        used_filename: String // The filename that was actually used to determine how to import the mod.
    },
    // Sent to relay progress information during the modding process.
    // This will NOT be the final message sent.
    LogMsg {
        message: String,
        level: LogLevel
    },
    FixedPlayerData {
        // True if a PlayerData.dat existed to fix, false if the request did nothing.
        existed: bool
    },
    DowngradedManifest {
        manifest_xml: String
    }
}

/// What type of file a file was imported as, and details about the resulting file.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImportResultType {
    ImportedMod {
        installed_mods: Vec<ModModel>,
        imported_id: String  
    },
    ImportedFileCopy {
        // The full path where the file was copied to.
        copied_to: String,
        // The mod ID that the file copy belonged to
        mod_id: String
    },
    ImportedSong,
    // A non-quest mod was detected (i.e. `.DLL`) and so the import failed.
    NonQuestModDetected
}

/// The trimmed version of the ModInfo type that is sent to the web client.
#[derive(Serialize, Deserialize)]
pub struct ModModel {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub game_version: Option<String>,
    pub description: Option<String>,
    pub is_enabled: bool,
    // True if the mod is core or if it is a required dependency of another core mod (potentially indirectly.)
    pub is_core: bool
}

impl From<&Mod> for ModModel {
    fn from(value: &Mod) -> Self {
        Self {
            id: value.manifest().id.clone(),
            name: value.manifest().name.clone(),
            version: value.manifest().version.clone(),
            game_version: value.manifest().package_version.clone(),
            description: value.manifest().description.clone(),
            is_enabled: value.installed(),
            is_core: value.is_core()
        }
    }
}