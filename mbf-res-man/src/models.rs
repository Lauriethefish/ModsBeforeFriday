use semver::Version;
use serde::{Deserialize, Serialize};

pub type DiffIndex = Vec<VersionDiffs>;

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