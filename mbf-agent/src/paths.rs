//! Module containing all of the fixed file paths used by MBF, for easy changing throughout the project.

use const_format::formatcp;
use crate::APK_ID;

/// Directory that QMOD files are stored in.
/// `$` is replaced with the game version
pub const QMODS: &str = formatcp!("/sdcard/ModData/{APK_ID}/Packages/$");
/// The legacy directory used to contain QMOD files in older builds of MBF.
pub const OLD_QMODS: &str = "/sdcard/ModsBeforeFriday/Mods";
/// The path of the `.nomedia` file added to ModData.
pub const MODDATA_NOMEDIA: &str = formatcp!("/sdcard/ModData/{APK_ID}/.nomedia");
/// Directory containing the modloader.
pub const MODLOADER_DIR: &str = formatcp!("/sdcard/ModData/{APK_ID}/Modloader");
/// Directory containing installed late mod files.
pub const LATE_MODS: &str = formatcp!("{MODLOADER_DIR}/mods");
/// Directory containing installed early mod files.
pub const EARLY_MODS: &str = formatcp!("{MODLOADER_DIR}/early_mods");
/// Directory containing installed library files.
pub const LIBS: &str = formatcp!("{MODLOADER_DIR}/libs");
/// The Android `files` directory for the app being modded.
pub const ANDROID_APP_FILES: &str = formatcp!("/sdcard/Android/data/{APK_ID}/files");
/// Path of the `PlayerData.dat` in the vanilla game.
pub const PLAYER_DATA: &str = formatcp!("{ANDROID_APP_FILES}/PlayerData.dat");
/// Path of the backup `PlayerData.dat` in the vanilla game.
pub const PLAYER_DATA_BAK: &str = formatcp!("{ANDROID_APP_FILES}/PlayerData.dat.bak");
/// Directory containing OBBs for the app.
pub const OBB_DIR: &str = formatcp!("/sdcard/Android/obb/{APK_ID}/");

/// Path to the `PlayerData.dat` of the `datakeeper` mod.
pub const DATAKEEPER_PLAYER_DATA: &str = "/sdcard/ModData/com.beatgames.beatsaber/Mods/datakeeper/PlayerData.dat";
/// An auxillary path that `PlayerData.dat` is copied to when modding in case it is corrupted/lost for any other reason.
pub const AUX_DATA_BACKUP: &str = "/sdcard/ModsBeforeFriday/PlayerData.backup.dat";

/// The folder that SongCore loads custom levels from.
pub const CUSTOM_LEVELS: &str = formatcp!("/sdcard/ModData/{APK_ID}/Mods/SongCore/CustomLevels");
/// A folder that MBF uses to download temporary files.
pub const MBF_DOWNLOADS: &str = "/data/local/tmp/mbf/downloads";
/// Temporary folder used by MBF during patching.
pub const TEMP: &str = "/data/local/tmp/mbf/tmp";
/// Path to the MBF resource cache.
pub const RES_CACHE: &str = "/data/local/tmp/mbf/res-cache";
/// Directories no longer used by MBF that should be deleted on startup if detected.
pub const LEGACY_DIRS: &[&str] = &[
    "/data/local/tmp/mbf-downloads",
    "/data/local/tmp/mbf-res-cache",
    "/data/local/tmp/mbf-tmp",
    "/data/local/tmp/mbf-uploads"
];