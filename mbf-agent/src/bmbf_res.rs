//! Collection of types used to read the BMBF resources repository to fetch core mod information.


use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Context, Result};

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct CoreMod {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "downloadLink")]
    pub download_url: String
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct VersionedCoreMods {
    // lastUpdated omitted
    pub mods: Vec<CoreMod>
}

pub type CoreModIndex = HashMap<String, VersionedCoreMods>;

const CORE_MODS_URL: &str = "https://git.bmbf.dev/unicorns/resources/-/raw/master/com.beatgames.beatsaber/core-mods.json";
pub fn fetch_core_mods() -> Result<CoreModIndex> {
    let core_mods_str = ureq::get(CORE_MODS_URL)
        .call()
        .context("Failed to GET from resources repository")?
        .into_string()?;

    Ok(serde_json::from_str(&core_mods_str).context("Core mods JSON was invalid")?)
}

