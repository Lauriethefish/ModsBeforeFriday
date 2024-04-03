mod manifest;
use std::{cell::RefCell, collections::{HashMap, HashSet}, fs::File, path::{Path, PathBuf}, rc::Rc};

use log::{error, info, warn};
pub use manifest::*;

use anyhow::{Context, Result, anyhow};
use semver::Version;

use crate::{download_file, zip::ZipFile};

const QMODS_DIR: &str = "/sdcard/ModsBeforeFriday/Mods";
const LATE_MODS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/mods";
const EARLY_MODS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/early_mods";
const LIBS_DIR: &str = "/sdcard/ModData/com.beatgames.beatsaber/Modloader/libs";

pub struct Mod {
    manifest: ModInfo,
    installed: bool,
    zip: ZipFile<File>,
    loaded_from: PathBuf
}

impl Mod {
    pub fn installed(&self) -> bool {
        self.installed
    }

    pub fn manifest(&self) -> &ModInfo {
        &self.manifest
    }

}

pub struct ModManager {
    mods: HashMap<String, Rc<RefCell<Mod>>>,
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

    pub fn get_mods(&self) -> impl Iterator<Item = &Rc<RefCell<Mod>>> {
        self.mods.values()
    }

    pub fn get_mod(&self, id: &str) -> Option<&Rc<RefCell<Mod>>> {
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
            let loaded_mod = Self::load_mod_from(mod_path)?;

            // TODO: Report error of conflicting ID
            if self.mods.contains_key(&loaded_mod.manifest.id) {
                continue;
            }
    
            self.mods.insert(loaded_mod.manifest.id.clone(), Rc::new(RefCell::new(loaded_mod)));
        }

        self.update_mods_status().context("Failed to check if mods installed after loading")?;
        Ok(())
    }

    fn load_mod_from(from: PathBuf) -> Result<Mod> {
        let mod_file = std::fs::File::open(&from).context("Failed to open mod archive")?;
        let mut zip = ZipFile::open(mod_file).context("Mod was invalid ZIP archive")?;

        let json_data = zip.read_file("mod.json").context("Mod had no mod.json manifest")?;

        let manifest = serde_json::from_slice(&json_data)?;
        Ok(Mod {
            manifest: manifest,
            installed: false, // Must call update_mods_status
            zip,
            loaded_from: from
        })
    }

    /// Checks whether or not each loaded mod is installed.
    pub fn update_mods_status(&mut self) -> Result<()> {
        let early_mod_files = list_dir_files(EARLY_MODS_DIR)?;
        let late_mod_files = list_dir_files(LATE_MODS_DIR)?;
        let libraries = list_dir_files(LIBS_DIR)?;
    
        for r#mod in self.mods.values() {
            let mut mod_info = (**r#mod).borrow_mut();
            let manifest = &mod_info.manifest;
            let mod_files_present = manifest.mod_files.iter().all(|file| early_mod_files.contains(file))
                && manifest.library_files.iter().all(|file| libraries.contains(file))
                && manifest.late_mod_files.iter().all(|file| late_mod_files.contains(file));

            mod_info.installed = mod_files_present;
        }
    
        Ok(())
    }

    /// Installs the mod with the given ID.
    /// This will install dependencies if necessary.
    pub fn install_mod(&mut self, id: &str) -> Result<()> {
        // Install the mod's dependencies if applicable
        let mod_rc =  self.mods.get(id)
            .ok_or(anyhow!("Could not install mod with ID {id} as it did not exist"))?.clone();

        let mut to_install = (*mod_rc).borrow_mut();
        info!("Installing {} v{}", to_install.manifest.id, to_install.manifest.version);

        for dep in &to_install.manifest.dependencies {
            match self.mods.get(&dep.id) {
                Some(existing_dep) => {
                    let dep_ref = (**existing_dep).borrow();
                    if !dep.version_range.matches(&dep_ref.manifest.version) {
                        info!("Dependency {} is out of date, got version {} but need {}", dep.id, dep_ref.manifest.version, dep.version_range);
                        drop(dep_ref);
                        self.install_dependency(&dep, true)?;
                    }   else if !dep_ref.installed {
                        // Must install the dependency
                        drop(dep_ref);
                        self.install_mod(&dep.id)?;
                    }
                },
                None => {
                    // Install dependency
                    info!("Dependency {} was not found: installing now", dep.id);
                    self.install_dependency(&dep, false)?;
                }
            }
        }

        self.install_unchecked(&mut to_install)?;
        Ok(())
    }

    /// Installs a mod without handling dependencies
    /// i.e. just copies the necessary files.
    fn install_unchecked(&self, to_install: &mut Mod) -> Result<()> {
        copy_stated_files(&mut to_install.zip, &to_install.manifest.mod_files, EARLY_MODS_DIR)?;
        copy_stated_files(&mut to_install.zip, &to_install.manifest.library_files, LIBS_DIR)?;
        copy_stated_files(&mut to_install.zip, &to_install.manifest.late_mod_files, LATE_MODS_DIR)?;

        for file_copy in &to_install.manifest.file_copies {
            if !to_install.zip.contains_file(&file_copy.name) {
                warn!("Could not install file copy {} as it did not exist in the QMOD", file_copy.name);
                continue;
            }

            let dest_path = Path::new(&file_copy.destination);
            match dest_path.parent() {
                Some(parent) => std::fs::create_dir_all(parent)
                    .context("Failed to create destination directory for file copy")?,
                None => {}
            }

            to_install.zip.extract_file_to(&file_copy.name, &file_copy.destination)
                .context("Failed to extract file copy")?;
        }
        to_install.installed = true;

        Ok(())
    }

    /// Uninstalls a mod without handling dependencies
    /// i.e. just deletes the necessary files.
    fn uninstall_unchecked(&self, to_remove: &mut Mod) -> Result<()> {
        delete_file_names(&to_remove.manifest.mod_files, EARLY_MODS_DIR)?;
        delete_file_names(&to_remove.manifest.library_files, LIBS_DIR)?;
        delete_file_names(&to_remove.manifest.late_mod_files, LATE_MODS_DIR)?;

        for copy in &to_remove.manifest.file_copies {
            let dest_path = Path::new(&copy.destination);
            if dest_path.exists() {
                std::fs::remove_file(dest_path).context("Failed to delete copied file")?;
            }
        }
        to_remove.installed = false;

        Ok(())
    }

    /// Uninstalls the mod with the given ID.
    /// This will uninstall dependant mods if necessary
    pub fn uninstall_mod(&self, id: &str) -> Result<()> {
        let mod_rc = self.mods.get(id)
            .ok_or(anyhow!("Could not uninstall mod with ID {id} as it did not exist"))?
            .clone();
        let mut to_remove = (*mod_rc).borrow_mut();
        info!("Uninstalling {} v{}", to_remove.manifest.id, to_remove.manifest.version);

        // Uninstall all depending mods
        for (id, m) in self.mods.iter() {
            let m_ref = (**m).borrow();
            if m_ref.installed && m_ref.manifest.dependencies.iter().any(|dep| &dep.id == id) {
                info!("Uninstalling dependant mod {}", m_ref.manifest.id);
                self.uninstall_mod(id)?;
            }
        }

        self.uninstall_unchecked(&mut to_remove)?;
        Ok(())
    }

    fn install_dependency(&mut self, dep: &ModDependency, upgrading: bool) -> Result<()> {
        // Find a path to save the dependency
        let save_path = self.mods_path().as_ref().join(format!("{}-DEP.qmod", dep.id));

        let link = match &dep.mod_link {
            Some(link) => link,
            None => return Err(anyhow!("Could not download dependency {}: no link given", dep.id))
        };

        info!("Downloading dependency from {}", link);
        download_file(&save_path, &link).context("Failed to download dependency")?;
        let loaded_dep = Self::load_mod_from(save_path.clone())?;

        // TODO: check ID matches

        // Now we must carefully check that existing installed mods are compatible with this dependency!
        if upgrading {
            if !self.check_dependency_compatibility(&dep.id, &loaded_dep.manifest.version) {
                drop(loaded_dep);
                std::fs::remove_file(&save_path)?;

                return Err(anyhow!("Could not install dependency {}", dep.id))
            }
        }

        // Remove existing dependency, unchecked as we don't want to nuke any dependencies
        // by allowing remove_mod to run a regular uninstall
        if upgrading {
            info!("Removing existing version of dependency");
            let existing_version = self.mods.get(&dep.id)
                .expect("Cannot upgrade dependency as it's not already installed.");

            self.uninstall_unchecked(&mut (**existing_version).borrow_mut())?;
            self.remove_mod(&dep.id)?;
        }
        self.mods.insert(dep.id.clone(), Rc::new(RefCell::new(loaded_dep)));
        self.install_mod(&dep.id)?;
        Ok(())
    }

    pub fn remove_mod(&mut self, id: &str) -> Result<()> {
        match self.mods.get(id) {
            Some(to_remove) => {
                let to_remove_ref = (**to_remove).borrow();
                let path_to_delete = to_remove_ref.loaded_from.clone();
                if to_remove_ref.installed {
                    self.uninstall_mod(id)?;
                }

                drop(to_remove_ref);
                self.mods.remove(id);

                std::fs::remove_file(path_to_delete)?;
                Ok(())
            },
            None => Ok(())
        }
    }

    // Checks that upgrading the dependency with ID dep_id to new_version will not result in an incompatibility with an existing installed mod.
    // Returns false if any mod has an incompatibility
    // Logs any issues discovered.
    fn check_dependency_compatibility(&self, dep_id: &str, new_version: &Version) -> bool {
        let mut all_compatible = true;
        for (_, existing_mod) in &self.mods {
            let mod_ref = (**existing_mod).borrow();
            // We don't care about uninstalled mods, since they have no invariants to preserve.
            if !mod_ref.installed {
                continue;
            }

            match mod_ref.manifest.dependencies
                .iter()
                .filter(|existing_dep| existing_dep.id == dep_id)
                .next() 
            {
                Some(existing_dep) => 
                if !existing_dep.version_range.matches(new_version) {
                    all_compatible = false;
                    error!("Cannot upgrade {dep_id} to {new_version}: Mod {} depends on range {}", 
                        mod_ref.manifest.id,
                        existing_dep.version_range
                    )
                },
                None => {}
            }
        }

        all_compatible
    }
}

// Copies all of the files with names in `files` to the path `to/{file name not including directory in ZIP}`
fn copy_stated_files(zip: &mut ZipFile<File>, files: &[String], to: impl AsRef<Path>) -> Result<()> {
    for file in files {
        if !zip.contains_file(file) {
            warn!("Could not install file {file} as it wasn't found in the QMOD");
            continue;
        }

        let file_name = file.split('/').last().unwrap();
        let copy_to = to.as_ref().join(file_name);

        zip.extract_file_to(file, copy_to).context("Failed to extract mod SO")?;
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