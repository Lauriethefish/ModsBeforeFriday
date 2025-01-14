//! Models used for communication *from the frontend to the backend*.

use std::collections::HashMap;

use serde::Deserialize;

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
        override_core_mod_url: Option<String>,
    },
    /// Installs or uninstalls any number of mods.
    /// This will also attempt to download and install dependencies, upgrade dependencies and will uninstall any
    /// depending mods of mods that have been disabled.
    ///
    /// Returns a `ModSyncResult` response.
    SetModsEnabled {
        statuses: HashMap<String, bool>,
    },

    // TODO: Make these lists to allow importing multiple mods at once?
    /// Removes the mod with the given ID, which will uninstall dependant mods.
    /// Returns a Mods message containing the mods now installed.
    RemoveMod {
        id: String,
    },
    /// Imports a mod or file copy from the given path on the quest.
    /// Returns an ImportedMod message containing the mods now installed, and the ID of the one that was imported, if importing a mod.
    /// Returns an ImportedFileCopy message if the file type was copied by a mod copy extension.
    /// Returns an ImportedSong message if the file type was copied to the songs folder.
    Import {
        from_path: String,
    },
    /// Downloads the file from the given URL and then attempts to import it.
    /// Returns an ImportResult message.
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
        // If this is true, libovrplatformloader.so will be replaced with the old version to make the game work on quest 1
        replace_ovr: bool,
        // If not null, this specifies a core mod JSON to use instead of the default core mods source.
        // This is useful for developers testing a core mod update.
        override_core_mod_url: Option<String>,
    },

    // Attempts to fix a blackscreen issue by removing PlayerData.dat from `/sdcard/...../files/`.
    // (and copying it to /sdcard/ModsBeforeFriday so it isn't lost. It will also be copied to the datakeeper directory iff there isn't already one there)
    // (This occurs when the permissions set by MBF copying the file lead to the game not being able to open it, typically on Quest 3,
    // unfortunately chmod 777 doesn't seem to fix the issue.)
    // Gives a `FixedPlayerData` response.
    FixPlayerData,
    /// Gets a copy of the AndroidManifest.xml for the given Beat Saber version, converted from AXML into an XML string.
    GetDowngradedManifest {
        version: String,
    },
    /// Reinstalls any core mods that are misssing/out of date and overwrites the modloader in case it is corrupt.
    /// Should fix most issues with any installation.
    /// Returns a `Mods` response containing the newly installed mods.
    QuickFix {
        // If not null, this specifies a core mod JSON to use instead of the default core mods source.
        // This is useful for developers testing a core mod update.
        override_core_mod_url: Option<String>,
        // If true, this request will delete ALL mods before reinstalling only the core mods.
        wipe_existing_mods: bool,
    },
}
