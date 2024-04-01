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
        app_info: Option<AppInfo>,
        supported_versions: Vec<String>
    },
    Patched
}