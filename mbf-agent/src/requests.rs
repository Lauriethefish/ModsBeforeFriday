use serde::{Deserialize, Serialize};

use crate::mod_man::Mod;

#[derive(Serialize)]
pub struct AppInfo {
    pub is_modded: bool,
    pub version: String
}


#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    GetModStatus,
    Patch
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum Response {
    ModStatus {
        // None if Beat Saber is not installed.
        app_info: Option<AppInfo>,
        // None if an internet connection could not be established
        supported_versions: Option<Vec<String>>,
        // All mods currently found in the mods folder
        installed_mods: Vec<Mod>
    },
    Patched
}