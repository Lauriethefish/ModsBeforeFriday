use serde::{Deserialize, Serialize};

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
        supported_versions: Option<Vec<String>>
    },
    Patched
}