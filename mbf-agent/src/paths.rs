//! Module containing all of the fixed file paths used by MBF, for easy changing throughout the project.

use std::sync::OnceLock;

use crate::APK_ID;



/// Macro to simplify initializing `OnceLock` with a static string.
macro_rules! init_lock {
    ($destination:expr, $value:expr) => {
        *($destination.get_or_init(|| $value))
    }
}

/// Macro to simplify initializing `OnceLock` with a formatted string.
macro_rules! init_lock_format {
    ($destination:expr, $($arg:tt)*) => {
        *($destination.get_or_init(|| { Box::leak(format!($($arg)*).into_boxed_str())}))
    };
}

/// Directory that QMOD files are stored in.
/// `$` is replaced with the game version
pub static QMODS: OnceLock<&str> = OnceLock::new();
/// The legacy directory used to contain QMOD files in older builds of MBF.
pub static OLD_QMODS: OnceLock<&str> = OnceLock::new();
/// The path of the `.nomedia` file added to ModData.
pub static MODDATA_NOMEDIA: OnceLock<&str> = OnceLock::new();
/// Directory containing the modloader.
pub static MODLOADER_DIR: OnceLock<&str> = OnceLock::new();
/// Directory containing installed late mod files.
pub static LATE_MODS: OnceLock<&str> = OnceLock::new();
/// Directory containing installed early mod files.
pub static EARLY_MODS: OnceLock<&str> = OnceLock::new();
/// Directory containing installed library files.
pub static LIBS: OnceLock<&str> = OnceLock::new();
/// The Android `files` directory for the app being modded.
pub static ANDROID_APP_FILES: OnceLock<&str> = OnceLock::new();
/// Path of the `PlayerData.dat` in the vanilla game.
pub static PLAYER_DATA: OnceLock<&str> = OnceLock::new();
/// Path of the backup `PlayerData.dat` in the vanilla game.
pub static PLAYER_DATA_BAK: OnceLock<&str> = OnceLock::new();
/// Directory containing OBBs for the app.
pub static OBB_DIR: OnceLock<&str> = OnceLock::new();
/// Path to the `PlayerData.dat` of the `datakeeper` mod.
pub static DATAKEEPER_PLAYER_DATA: OnceLock<&str> = OnceLock::new();
/// An auxillary path that `PlayerData.dat` is copied to when modding in case it is corrupted/lost for any other reason.
pub static AUX_DATA_BACKUP: OnceLock<&str> = OnceLock::new();
/// The folder that SongCore loads custom levels from.
pub static CUSTOM_LEVELS: OnceLock<&str> = OnceLock::new();
/// A folder that MBF uses to download temporary files.
pub static MBF_DOWNLOADS: OnceLock<&str> = OnceLock::new();
/// Temporary folder used by MBF during patching.
pub static TEMP: OnceLock<&str> = OnceLock::new();
/// Path to the MBF resource cache.
pub static RES_CACHE: OnceLock<&str> = OnceLock::new();
/// Directories no longer used by MBF that should be deleted on startup if detected.
pub static LEGACY_DIRS: &[&str] = &[
    "/data/local/tmp/mbf-downloads",
    "/data/local/tmp/mbf-res-cache",
    "/data/local/tmp/mbf-tmp",
    "/data/local/tmp/mbf-uploads",
];

static INITIALIZED: OnceLock<bool> = OnceLock::new();

pub fn init_paths(apk_id: &str) {
    if *(INITIALIZED.get().unwrap_or(&false)) {
        return;
    }

    let _apk_id = init_lock_format!(APK_ID, "{}", apk_id);
    let _qmods = init_lock_format!(QMODS, "/sdcard/ModData/{}/Packages/$", _apk_id);
    let _old_qmods = init_lock!(OLD_QMODS, "/sdcard/ModsBeforeFriday/Mods");
    let _moddata_nomedia = init_lock_format!(MODDATA_NOMEDIA, "/sdcard/ModData/{}/.nomedia", _apk_id);
    let _modloader_dir = init_lock_format!(MODLOADER_DIR, "/sdcard/ModData/{}/Modloader", _apk_id);
    let _late_mods = init_lock_format!(LATE_MODS, "{}/mods", _modloader_dir);
    let _early_mods = init_lock_format!(EARLY_MODS, "{}/early_mods", _modloader_dir);
    let _libs = init_lock_format!(LIBS, "{}/libs", _modloader_dir);
    let _android_app_files = init_lock_format!(ANDROID_APP_FILES, "/sdcard/Android/data/{}/files", _apk_id);
    let _player_data = init_lock_format!(PLAYER_DATA, "{}/PlayerData.dat", _android_app_files);
    let _player_data_bak = init_lock_format!(PLAYER_DATA_BAK, "{}/PlayerData.dat.bak", _android_app_files);
    let _obb_dir = init_lock_format!(OBB_DIR, "/sdcard/Android/obb/{}/", _apk_id);
    let _datakeeper_player_data = init_lock_format!(DATAKEEPER_PLAYER_DATA, "/sdcard/ModData/{}/Mods/datakeeper/PlayerData.dat", _apk_id);
    let _aux_data_backup = init_lock!(AUX_DATA_BACKUP, "/sdcard/ModsBeforeFriday/PlayerData.backup.dat");
    let _custom_levels = init_lock_format!(CUSTOM_LEVELS, "/sdcard/ModData/{}/Mods/SongCore/CustomLevels", _apk_id);
    let _mbf_downloads = init_lock!(MBF_DOWNLOADS, "/data/local/tmp/mbf/downloads");
    let _temp = init_lock!(TEMP, "/data/local/tmp/mbf/tmp");
    let _res_cache = init_lock!(RES_CACHE, "/data/local/tmp/mbf/res-cache");

    let _ = INITIALIZED.set(true);
}
