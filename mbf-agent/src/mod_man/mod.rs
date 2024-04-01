mod manifest;
use std::path::Path;

pub use manifest::*;

use anyhow::{Context, Result, anyhow};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::zip::ZipFile;

const QMODS_DIR: &str = "/sdcard/ModsBeforeFriday/Mods";
const LATE_MODS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/early_mods";
const EARLY_MODS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/mods";
const LIBS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/libs";

#[derive(Serialize, Deserialize)]
pub struct Mod {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub description: Option<String>,
    pub is_enabled: bool
}

impl From<ModJson> for Mod {
    fn from(value: ModJson) -> Self {
        Self {
            id: value.id,
            name: value.name,
            version: value.version,
            description: value.description,
            is_enabled: false
        }
    }
}

fn create_mods_dir() -> Result<()> {
    std::fs::create_dir_all(QMODS_DIR)?;
    Ok(())
}

pub fn load_mod_info() -> Result<Vec<ModJson>> {
    create_mods_dir()?;

    let mut result = Vec::new();
    for stat in std::fs::read_dir(QMODS_DIR)? {
        let entry = match stat {
            Ok(entry) => entry,
            Err(_) => continue // Ignore innacessible mods
        };

        if !entry.file_type()?.is_file() {
            continue;
        }

        result.push(read_mod_json(entry.path())?);
    }

    Ok(result)
}

fn read_mod_json(from: impl AsRef<Path>) -> Result<ModJson> {
    let mod_file = std::fs::File::open(from).context("Failed to open mod archive")?;
    let mut zip = ZipFile::open(mod_file).context("Mod was invalid ZIP archive")?;

    let json_data = match zip.read_file("mod.json")? {
        Some(data) => data,
        None => return Err(anyhow!("Mod contained no mod.json manifest"))
    };

    Ok(serde_json::from_slice(&json_data)?)
}