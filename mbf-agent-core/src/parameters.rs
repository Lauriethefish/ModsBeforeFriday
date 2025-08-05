//! Module containing all of the fixed file paths used by MBF, for easy changing throughout the project.

use std::sync::{OnceLock, LazyLock};

/// A OnceLock that contains the default parameters for the agent.
static _PARAMETERS: OnceLock<AgentParameters> = OnceLock::new();

/// Initializes the parameters for the agent with the given game ID and ignore_package_id flag.
/// This function is to be called once at the start of the program to set up the parameters.
///
/// # Arguments
///
/// * `game_id` - The game ID to be used for the agent.
/// * `ignore_package_id` - A boolean flag indicating whether to skip the game ID check.
///
/// # Returns
///
/// Returns true if the parameters were successfully initialized, false if they were already initialized.
pub fn init_parameters(game_id: &str, ignore_package_id: bool) -> bool {
    _PARAMETERS.set(AgentParameters::new(game_id, ignore_package_id)).is_ok()
}

/// A static reference to the parameters for the agent.
pub static PARAMETERS: LazyLock<AgentParameters> = LazyLock::new(|| _PARAMETERS.get_or_init(|| AgentParameters::new("com.beatgames.beatsaber", false)).clone());

/// A struct that contains the parameters for the agent, including various file paths and settings.
#[derive(Clone)]
pub struct AgentParameters {
    /// The APK ID of the app being modded.
    pub apk_id: String,

    /// If true, the game ID check is skipped during qmod install.
    pub _ignore_package_id: bool,

    /// Directory that QMOD files are stored in.
    /// `$` is replaced with the game version
    pub qmods: String,

    /// The legacy directory used to contain QMOD files in older builds of MBF.
    pub old_qmods: String,

    /// The path of the `.nomedia` file added to ModData.
    pub moddata_nomedia: String,

    /// Directory containing the modloader.
    pub modloader_dir: String,

    /// Directory containing installed late mod files.
    pub late_mods: String,

    /// Directory containing installed early mod files.
    pub early_mods: String,

    /// Directory containing installed library files.
    pub libs: String,

    /// The Android `files` directory for the app being modded.
    pub _android_app_files: String,

    /// Path of the `PlayerData.dat` in the vanilla game.
    pub player_data: String,

    /// Path of the backup `PlayerData.dat` in the vanilla game.
    pub player_data_bak: String,

    /// Directory containing OBBs for the app.
    pub obb_dir: String,

    /// Path to the `PlayerData.dat` of the `datakeeper` mod.
    pub datakeeper_player_data: String,

    /// An auxillary path that `PlayerData.dat` is copied to when modding in case it is corrupted/lost for any other reason.
    pub aux_data_backup: String,

    /// The folder that SongCore loads custom levels from.
    pub custom_levels: String,

    /// A folder that MBF uses to download temporary files.
    pub mbf_downloads: String,

    /// Temporary folder used by MBF during patching.
    pub temp: String,

    /// Path to the MBF resource cache.
    pub res_cache: String,

    /// Directories no longer used by MBF that should be deleted on startup if detected.
    pub legacy_dirs:  [String; 4],
}

/// Implements the `AgentParameters` struct, which contains various paths and settings used by the agent.
impl AgentParameters {
    /// Creates a new instance of `AgentParameters` with paths initialized based on the provided APK ID.
    ///
    /// # Arguments
    ///
    /// * `apk_id` - A string slice that holds the APK ID, which is used to construct various paths.
    ///
    /// # Returns
    ///
    /// Returns an instance of `AgentParameters` with all fields populated based on the provided APK ID.
    ///
    /// # Example
    ///
    /// ```rust
    /// let apk_id = "com.example.app";
    /// let paths = AgentParameters::new(apk_id);
    /// println!("{}", paths.qmods); // Outputs: /sdcard/ModData/com.example.app/Packages/$
    /// ```
    pub fn new<'a>(apk_id: &str, ignore_package_id: bool) -> AgentParameters {
        let local_tmp = "/data/local/tmp";

        let apk_id = format!("{apk_id}").to_string();
        let qmods = format!("/sdcard/ModData/{apk_id}/Packages/$").to_string();
        let old_qmods = "/sdcard/ModsBeforeFriday/Mods".to_string();
        let moddata_nomedia = format!("/sdcard/ModData/{apk_id}/.nomedia").to_string();
        let modloader_dir = format!("/sdcard/ModData/{apk_id}/Modloader").to_string();
        let late_mods = format!("{modloader_dir}/mods").to_string();
        let early_mods = format!("{modloader_dir}/early_mods").to_string();
        let libs = format!("{modloader_dir}/libs").to_string();
        let android_app_files = format!("/sdcard/Android/data/{apk_id}/files").to_string();
        let player_data = format!("{android_app_files}/PlayerData.dat").to_string();
        let player_data_bak = format!("{android_app_files}/PlayerData.dat.bak").to_string();
        let obb_dir = format!("/sdcard/Android/obb/{apk_id}/").to_string();
        let datakeeper_player_data = format!("/sdcard/ModData/{apk_id}/Mods/datakeeper/PlayerData.dat").to_string();
        let aux_data_backup = "/sdcard/ModsBeforeFriday/PlayerData.backup.dat".to_string();
        let custom_levels = format!("/sdcard/ModData/{apk_id}/Mods/SongCore/CustomLevels").to_string();
        let mbf_downloads = format!("{local_tmp}/mbf/downloads").to_string();
        let temp = format!("{local_tmp}/mbf/tmp").to_string();
        let res_cache = format!("{local_tmp}/mbf/res-cache").to_string();
        let legacy_dirs = [
            format!("{local_tmp}/mbf-downloads").to_string(),
            format!("{local_tmp}/mbf-res-cache").to_string(),
            format!("{local_tmp}/mbf-tmp").to_string(),
            format!("{local_tmp}/mbf-uploads").to_string(),
        ];

        Self {
            apk_id,
            _ignore_package_id: ignore_package_id,
            qmods,
            old_qmods,
            moddata_nomedia,
            modloader_dir,
            late_mods,
            early_mods,
            libs,
            _android_app_files: android_app_files,
            player_data,
            player_data_bak,
            obb_dir,
            datakeeper_player_data,
            aux_data_backup,
            custom_levels,
            mbf_downloads,
            temp,
            res_cache,
            legacy_dirs
        }
    }
}
