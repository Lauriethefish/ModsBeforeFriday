mod manifest;
use std::{collections::{HashMap, HashSet}, fs::File, io::Write, path::{Path, PathBuf}};

pub use manifest::*;

use anyhow::{Context, Result, anyhow};

use crate::zip::ZipFile;

const QMODS_DIR: &str = "/sdcard/ModsBeforeFriday/Mods";
const LATE_MODS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/early_mods";
const EARLY_MODS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/mods";
const LIBS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/libs";

pub struct Mod {
    manifest: ModInfo,
    installed: bool,
    zip: ZipFile<File>
}

impl Mod {
    pub fn get_installed(&self) -> bool {
        self.installed
    }

    pub fn manifest(&self) -> &ModInfo {
        &self.manifest
    }

    pub fn into_manifest(self) -> ModInfo {
        self.manifest
    }
}

pub struct ModManager {
    mods: HashMap<String, Mod>,
}

impl ModManager {
    pub fn new() -> Self {    
        Self {
            mods: HashMap::new()
        }
    }

    pub fn mods_path(&self) -> impl AsRef<Path> {
        QMODS_DIR
    }

    // Removes ALL mod and library files and deletes ALL mods from the current game.
    pub fn wipe_all_mods(&mut self) -> Result<()> {
        // Wipe absolutely everything: clean slate
        self.mods.clear();
        std::fs::remove_dir_all(LATE_MODS_DIR)?;
        std::fs::remove_dir_all(EARLY_MODS_DIR)?;
        std::fs::remove_dir_all(LIBS_DIR)?;
        std::fs::remove_dir_all(QMODS_DIR)?;
        create_mods_dir()?;
        Ok(())
    }

    pub fn into_mods(self) -> impl Iterator<Item = Mod> {
        self.mods.into_values()
    }

    pub fn get_mod(&self, id: &str) -> Option<&Mod> {
        self.mods.get(id)
    }
    
    /// Loads any mods from the QMODs directory that have not yet been loaded.
    pub fn load_mods(&mut self) -> Result<()> {
        create_mods_dir()?;
        self.mods.clear();
    
        for stat in std::fs::read_dir(QMODS_DIR)? {
            let entry = match stat {
                Ok(entry) => entry,
                Err(_) => continue // Ignore innacessible mods
            };
    
            let mod_path = entry.path();
            if !entry.file_type()?.is_file() {
                continue;
            }
            let loaded_mod = Self::read_mod(mod_path)?;

            // TODO: Report error of conflicting ID
            if self.mods.contains_key(&loaded_mod.manifest.id) {
                continue;
            }
    
            self.mods.insert(loaded_mod.manifest.id.clone(), loaded_mod);
        }

        self.update_mods_status().context("Failed to check if mods installed after loading")?;
        Ok(())
    }

    fn read_mod(from: PathBuf) -> Result<Mod> {
        let mod_file = std::fs::File::open(&from).context("Failed to open mod archive")?;
        let mut zip = ZipFile::open(mod_file).context("Mod was invalid ZIP archive")?;

        let json_data = match zip.read_file("mod.json")? {
            Some(data) => data,
            None => return Err(anyhow!("Mod contained no mod.json manifest"))
        };

        let manifest = serde_json::from_slice(&json_data)?;
        Ok(Mod {
            manifest: manifest,
            installed: false, // Must call update_mods_status
            zip
        })
    }

    /// Checks whether or not each loaded mod is installed.
    pub fn update_mods_status(&mut self) -> Result<()> {
        let early_mod_files = list_dir_files(EARLY_MODS_DIR)?;
        let late_mod_files = list_dir_files(LATE_MODS_DIR)?;
        let libraries = list_dir_files(LIBS_DIR)?;
    
        for r#mod in self.mods.values_mut() {
            let mod_info = &mut r#mod.manifest;
            r#mod.installed = mod_info.mod_files.iter().all(|file| early_mod_files.contains(file))
                && mod_info.library_files.iter().all(|file| libraries.contains(file))
                && mod_info.late_mod_files.iter().all(|file| late_mod_files.contains(file));
        }
    
        Ok(())
    }

    /// Installs a mod without handling dependencies
    /// i.e. just copies the necessary files.
    fn install_unchecked(&mut self, id: &str) -> Result<()> {
        let to_install = self.mods.get_mut(id)
            .ok_or(anyhow!("Could not install mod with ID {id} as it did not exist"))?;

        copy_stated_files(&mut to_install.zip, &to_install.manifest.mod_files, EARLY_MODS_DIR)?;
        copy_stated_files(&mut to_install.zip, &to_install.manifest.library_files, LIBS_DIR)?;
        copy_stated_files(&mut to_install.zip, &to_install.manifest.late_mod_files, LATE_MODS_DIR)?;

        Ok(())
    }

    /// Installs the mod with the given ID.
    /// This will install dependencies if necessary.
    pub fn install_mod(&mut self, id: &str) -> Result<()> {
        self.install_unchecked(id)?; // TODO
        Ok(())
    }

    /// Uninstalls a mod without handling dependencies
    /// i.e. just deletes the necessary files.
    fn uninstall_unchecked(&mut self, id: &str) -> Result<()> {
        let to_remove = self.mods.get_mut(id)
            .ok_or(anyhow!("Could not uninstall mod with ID {id} as it did not exist"))?;

        delete_file_names(&to_remove.manifest.mod_files, EARLY_MODS_DIR)?;
        delete_file_names(&to_remove.manifest.library_files, LIBS_DIR)?;
        delete_file_names(&to_remove.manifest.late_mod_files, LATE_MODS_DIR)?;

        Ok(())
    }

    /// Uninstalls the mod with the given ID.
    /// This will uninstall dependant mods if necessary
    pub fn uninstall_mod(&mut self, id: &str) -> Result<()> {
        self.uninstall_unchecked(id)?; // TODO
        Ok(())
    }
}

// Copies all of the files with names in `files` to the path `to/{file name not including directory in ZIP}`
fn copy_stated_files(zip: &mut ZipFile<File>, files: &[String], to: impl AsRef<Path>) -> Result<()> {
    for file in files {
        // TODO: Create a new Zip method to avoid reading the whole SO into memory
        let contents = match zip.read_file(file)? {
            Some(contents) => contents,
            None => continue // No file existed, skip
        };
        let file_name = file.split('/').last().unwrap();

        let copy_to = to.as_ref().join(file_name);
        let mut handle = std::fs::OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(copy_to)?;

        handle.write_all(&contents)?;
    }

    Ok(())
}

fn delete_file_names(file_paths: &[String], within: impl AsRef<Path>) -> Result<()> {
    for path in file_paths {
        let file_name = path.split('/').last().unwrap();
        let stored_path = within.as_ref().join(file_name);
        std::fs::remove_file(stored_path)?;
    }

    Ok(())
}

fn create_mods_dir() -> Result<()> {
    std::fs::create_dir_all(QMODS_DIR)?;
    std::fs::create_dir_all(LATE_MODS_DIR)?;
    std::fs::create_dir_all(EARLY_MODS_DIR)?;
    std::fs::create_dir_all(LIBS_DIR)?;

    Ok(())
}

fn list_dir_files(path: impl AsRef<Path>) -> Result<HashSet<String>> {
    Ok(std::fs::read_dir(&path)?.filter_map(|file| match file {
        Ok(file) => file.file_name().into_string().ok(),
        Err(_) => None
    }).collect())
}