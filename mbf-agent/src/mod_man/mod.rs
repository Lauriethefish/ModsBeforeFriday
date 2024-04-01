mod manifest;
use std::{collections::{HashMap, HashSet}, path::Path};

pub use manifest::*;

use anyhow::{Context, Result, anyhow};

use crate::zip::ZipFile;

const QMODS_DIR: &str = "/sdcard/ModsBeforeFriday/Mods";
const LATE_MODS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/early_mods";
const EARLY_MODS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/mods";
const LIBS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/libs";

fn create_mods_dir() -> Result<()> {
    std::fs::create_dir_all(QMODS_DIR)?;
    Ok(())
}

pub fn load_mod_info() -> Result<HashMap<String, ModInfo>> {
    create_mods_dir()?;

    // TODO: Handle conflicting IDs
    let mut result = HashMap::new();
    for stat in std::fs::read_dir(QMODS_DIR)? {
        let entry = match stat {
            Ok(entry) => entry,
            Err(_) => continue // Ignore innacessible mods
        };

        if !entry.file_type()?.is_file() {
            continue;
        }

        let mod_json = read_mod_json(entry.path())?;

        result.insert(mod_json.id.clone(), mod_json);
    }

    update_mods_status(result.values_mut()).context("Failed to check if mods were enabled")?;
    Ok(result)
}

fn update_mods_status<'a>(mods: impl Iterator<Item = &'a mut ModInfo>) -> Result<()> {
    let early_mod_files = list_dir_files(EARLY_MODS_DIR)?;
    let late_mod_files = list_dir_files(LATE_MODS_DIR)?;
    let libraries = list_dir_files(LIBS_DIR)?;

    for mod_info in mods {
        mod_info.is_enabled = mod_info.mod_files.iter().all(|file| early_mod_files.contains(file))
            && mod_info.library_files.iter().all(|file| libraries.contains(file))
            && mod_info.late_mod_files.iter().all(|file| late_mod_files.contains(file));
    }

    Ok(())
}

fn list_dir_files(path: impl AsRef<Path>) -> Result<HashSet<String>> {
    std::fs::create_dir_all(&path).context("Failed to create SOs directory")?;

    Ok(std::fs::read_dir(&path)?.filter_map(|file| match file {
        Ok(file) => file.file_name().into_string().ok(),
        Err(_) => None
    }).collect())
}

fn read_mod_json(from: impl AsRef<Path>) -> Result<ModInfo> {
    let mod_file = std::fs::File::open(from).context("Failed to open mod archive")?;
    let mut zip = ZipFile::open(mod_file).context("Mod was invalid ZIP archive")?;

    let json_data = match zip.read_file("mod.json")? {
        Some(data) => data,
        None => return Err(anyhow!("Mod contained no mod.json manifest"))
    };

    Ok(serde_json::from_slice(&json_data)?)
}