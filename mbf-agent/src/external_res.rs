//! Collection of types used to read the BMBF resources repository to fetch core mod information.
use log::info;
use semver::Version;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, sync, time::Duration};
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

/// We separate this out into an enum as if the core mod index can't be fetched,
/// then the frontend warns of a lack of internet access and prevents the user from trying to patch.
#[derive(Debug)]
pub enum JsonPullError {
    FetchError(anyhow::Error),
    ParseError(anyhow::Error)
}

impl Display for JsonPullError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(e) => write!(f, "Failed to parse core mod index: {e}"),
            Self::FetchError(e) => write!(f, "Failed to download core mod index: {e}")
        }
    }
}

impl std::error::Error for JsonPullError { }

const CORE_MODS_URL: &str = "https://raw.githubusercontent.com/QuestPackageManager/bs-coremods/main/core_mods.json";

// If no data is read for this period of time during a file download, the download will be failed.
pub const REQUEST_TIMEOUT_READ_SECS: u64 = 20;

static AGENT: sync::OnceLock<ureq::Agent> = sync::OnceLock::new();

pub fn get_agent() -> &'static ureq::Agent {
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(REQUEST_TIMEOUT_READ_SECS))
            .https_only(true)
            .try_proxy_from_env(true)
            .user_agent(format!("mbf-agent/{}", env!("CARGO_PKG_VERSION")).as_str())
            .build()
    })
}

pub fn fetch_json<T: DeserializeOwned>(from: &str) -> Result<T, JsonPullError> {
    let response = match get_agent().get(from)
        .call()
        .context("Failed to GET resource") {
            Ok(resp) => resp,
            Err(err) => return Err(JsonPullError::FetchError(err))
        };

    let resp_string = match response.into_string() {
        Ok(str) => str,
        Err(err) => return Err(JsonPullError::ParseError(err.into()))
    };

    match serde_json::from_str(&resp_string).context("JSON was invalid") {
        Ok(parsed) => Ok(parsed),
        Err(err) => Err(JsonPullError::ParseError(err.into()))
    }
}

pub fn fetch_core_mods(override_core_mod_url: Option<String>) -> Result<CoreModIndex, JsonPullError> {
    match override_core_mod_url {
        Some(url) => {
            info!("Using overridden core mod URL");
            fetch_json(&url)
        },
        None => fetch_json(CORE_MODS_URL)
    }
}

const UNITY_INDEX_URL: &str = "https://raw.githubusercontent.com/Lauriethefish/QuestUnstrippedUnity/main/index.json";
const UNITY_VER_FORMAT: &str = "https://raw.githubusercontent.com/Lauriethefish/QuestUnstrippedUnity/main/versions/{0}.so";

pub fn get_libunity_url(apk_id: &str, version: &str) -> Result<Option<String>> {
    let resp = get_agent().get(UNITY_INDEX_URL)
        .call()
        .context("Failed to GET libunity index")?;

    // Contains an entry for each app supported by the index, which contains an entry for each version of that app.
    let unity_index: HashMap<String, HashMap<String, String>> = serde_json::from_str(&resp.into_string()?)?;

    let app_index = match unity_index.get(apk_id) {
        Some(app_index) => app_index,
        None => return Ok(None)
    };
    match app_index.get(version) {
        Some(unity_version) => Ok(Some(UNITY_VER_FORMAT.replace("{0}", &unity_version))),
        None => Ok(None)
    }
}

/// The next section contains the methods used to access the diffs needed to downgrade.
/// MBF only supports downgrading from the latest version to latest moddable, but this implementation does support having a diff from any version to any other version.

/// We just use one github release with a JSON file attached to it that explains the content of the other files attached,
/// since there is no quota on the total size of a release.

const DIFF_INDEX_STEM: &str = "https://github.com/Lauriethefish/mbf-diffs/releases/download/1.0.0";

pub type DiffIndex = Vec<VersionDiffs>;

/// The diffs needed to downgrade between two particular Beat Saber versions.
#[derive(Clone, Deserialize, Serialize)]
pub struct VersionDiffs {
    pub from_version: String,
    pub to_version: String,

    pub apk_diff: Diff,
    pub obb_diffs: Vec<Diff>
}

/// A diff for a particular file.
#[derive(Clone, Deserialize, Serialize)]
pub struct Diff {
    pub diff_name: String,

    pub file_name: String,
    pub file_crc: u32,
    pub output_file_name: String,
    pub output_crc: u32,
    pub output_size: usize
}

pub fn get_diff_index() -> Result<DiffIndex, JsonPullError> {
    fetch_json(&format!("{DIFF_INDEX_STEM}/index.json"))
}

pub fn get_diff_url(diff: &Diff) -> String {
    format!("{DIFF_INDEX_STEM}/{}", diff.diff_name)
}

