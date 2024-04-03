//! Collection of types used to read the BMBF resources repository to fetch core mod information.
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};
use anyhow::{Context, Result};
use std::io::Read;

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

/// We separate this out into an enum as if the core mod index can't be fetched,
/// then the frontend warns of a lack of internet access and prevents the user from trying to patch.
#[derive(Debug)]
pub enum CoreModsError {
    FetchError(anyhow::Error),
    ParseError(anyhow::Error)
}

impl Display for CoreModsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(e) => write!(f, "Failed to parse core mod index: {e}"),
            Self::FetchError(e) => write!(f, "Failed to download core mod index: {e}")
        }
    }
}

impl std::error::Error for CoreModsError { }

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

const UNITY_INDEX_URL: &str = "https://raw.githubusercontent.com/Lauriethefish/QuestUnstrippedUnity/main/index.json";
const UNITY_VER_FORMAT: &str = "https://raw.githubusercontent.com/Lauriethefish/QuestUnstrippedUnity/main/versions/{0}.so";

pub fn get_libunity_stream(apk_id: &str, version: &str) -> Result<Option<impl Read>> {
    let resp = ureq::get(UNITY_INDEX_URL)
        .call()
        .context("Failed to GET libunity index")?;

    // Contains an entry for each app supported by the index, which contains an entry for each version of that app.
    let unity_index: HashMap<String, HashMap<String, String>> = serde_json::from_str(&resp.into_string()?)?;

    let app_index = match unity_index.get(apk_id) {
        Some(app_index) => app_index,
        None => return Ok(None)
    };
    match app_index.get(version) {
        Some(unity_version) => {
            let version_uri = UNITY_VER_FORMAT.replace("{0}", &unity_version);

            Ok(Some(ureq::get(&version_uri)
                .call()
                .context("Failed to GET libunity version")?
                .into_reader()))
        },
        None => Ok(None)
    }
}

