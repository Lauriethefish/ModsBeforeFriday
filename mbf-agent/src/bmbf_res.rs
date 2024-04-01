//! Collection of types used to read the BMBF resources repository to fetch core mod information.


use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Context, Result};

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct CoreMod {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "version")]
    pub version: Version,
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

pub enum CoreModsError {
    FetchError(anyhow::Error),
    ParseError(anyhow::Error)
}

const CORE_MODS_URL: &str = "https://git.bmbf.dev/unicorns/resources/-/raw/master/com.beatgames.beatsaber/core-mods.json";
pub fn fetch_core_mods() -> Result<CoreModIndex, CoreModsError> {
    let response = match ureq::get(CORE_MODS_URL)
        .call()
        .context("Failed to GET from resources repository") {
            Ok(resp) => resp,
            Err(err) => return Err(CoreModsError::FetchError(err))
        };

    let resp_string = match response.into_string() {
        Ok(str) => str,
        Err(err) => return Err(CoreModsError::ParseError(err.into()))
    };

    match serde_json::from_str(&resp_string).context("Core mods JSON was invalid") {
        Ok(core_mods) => Ok(core_mods),
        Err(err) => Err(CoreModsError::ParseError(err.into()))
    }
}

