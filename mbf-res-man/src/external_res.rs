//! Collection of types used to read the BMBF resources repository to fetch core mod information.
use log::info;
use crate::{models::{Diff, DiffIndex, VersionedCoreMods}, res_cache::{JsonPullError, ResCache}};
use std::collections::HashMap;
use anyhow::{Context, Result, anyhow};

pub type CoreModIndex = HashMap<String, VersionedCoreMods>;

const CORE_MODS_URL: &str = "https://raw.githubusercontent.com/QuestPackageManager/bs-coremods/main/core_mods.json";

pub fn fetch_core_mods(res_cache: &ResCache, override_core_mod_url: Option<String>) -> Result<CoreModIndex, JsonPullError> {
    match override_core_mod_url {
        Some(url) => {
            info!("Using overridden core mod URL");
            // TODO: The override core mod URL should NOT cache
            res_cache.get_json_cached(&url, "core_mods_override.json")
        },
        None => res_cache.get_json_cached(CORE_MODS_URL, "core_mods.json")
    }
}

const UNITY_INDEX_URL: &str = "https://raw.githubusercontent.com/Lauriethefish/QuestUnstrippedUnity/main/index.json";
const UNITY_VER_FORMAT: &str = "https://raw.githubusercontent.com/Lauriethefish/QuestUnstrippedUnity/main/versions/{0}.so";

pub fn get_libunity_url(res_cache: &ResCache, apk_id: &str, version: &str) -> Result<Option<String>> {
    // Contains an entry for each app supported by the index, which contains an entry for each version of that app.
    let unity_index: HashMap<String, HashMap<String, String>> = 
        res_cache.get_json_cached(UNITY_INDEX_URL, "libunity_index.json")?;

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

pub fn get_diff_index(res_cache: &ResCache) -> Result<DiffIndex, JsonPullError> {
    res_cache.get_json_cached(&format!("{DIFF_INDEX_STEM}/index.json"), "diff_index.json")
}

pub fn get_diff_url(diff: &Diff) -> String {
    format!("{DIFF_INDEX_STEM}/{}", diff.diff_name)
}

const MANIFEST_FORMAT: &str = "https://github.com/Lauriethefish/mbf-manifests/releases/download/1.0.0/{0}.xml";

// Downloads the AndroidManifest.xml file for the given Beat Saber version (in AXML format) and returns it to the frontend.
pub fn get_manifest_axml(agent: &ureq::Agent, version: String) -> Result<Vec<u8>> {
    let manifest_url = MANIFEST_FORMAT.replace("{0}", &version);

    let resp = agent.get(&manifest_url)
        .call()
        .context("Fetching manifest for BS ver")?;

    if resp.status() == 404 {
        return Err(anyhow!("Could not find an AXML manifest for version {version} (404). Report this so that one can be added"));
    }

    let mut buffer = Vec::new();
    resp.into_reader().read_to_end(&mut buffer).context("Reading response")?;

    Ok(buffer)
}

