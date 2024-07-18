mod manifest;
mod lock;

use std::{cell::RefCell, collections::{HashMap, HashSet}, io::{Cursor, Read, Seek}, path::{Path, PathBuf}, rc::Rc};

use jsonschema::JSONSchema;
use lock::ModInstallLock;
use log::{error, info, warn};
pub use manifest::*;

use anyhow::{Context, Result, anyhow};
use mbf_zip::ZipFile;
use semver::Version;

use crate::{download_to_vec_with_attempts, EARLY_MODS_DIR, LATE_MODS_DIR, LIBS_DIR, OLD_QMODS_DIR};

const QMOD_SCHEMA: &str = include_str!("qmod_schema.json");
const MAX_SCHEMA_VERSION: Version = Version::new(1, 2, 0);

pub struct Mod {
    manifest: ModInfo,
    installed: bool,
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
    schema: JSONSchema,
    qmods_dir: String
}

impl ModManager {
    pub fn new(game_version: &str) -> Self {    
        Self {
            mods: HashMap::new(),
            schema: JSONSchema::options()
                .compile(&serde_json::from_str::<serde_json::Value>(QMOD_SCHEMA).expect("QMOD schema was not valid JSON"))
                .expect("QMOD schema was not a valid JSON schema"),
            qmods_dir: crate::QMODS_DIR.replace('$', game_version) // Each game version stores its QMODs in a different directory.
        }
    }

    // Removes a directory and all its files recursively, if that directory already exists.
    fn remove_dir_if_exists(path: impl AsRef<Path>) -> Result<()> {
        if path.as_ref().exists() {
            std::fs::remove_dir_all(path).context("Failed to remove directory")?;
        }

        Ok(())
    }

    // Removes ALL mod and library files and deletes ALL mods from the current game.
    pub fn wipe_all_mods(&mut self) -> Result<()> {
        let _lock = ModInstallLock::excl()?;

        // Wipe absolutely everything: clean slate
        self.mods.clear();
        Self::remove_dir_if_exists(OLD_QMODS_DIR)?;
        Self::remove_dir_if_exists(LATE_MODS_DIR)?;
        Self::remove_dir_if_exists(EARLY_MODS_DIR)?;
        Self::remove_dir_if_exists(LIBS_DIR)?;
        Self::remove_dir_if_exists(&self.qmods_dir)?;
        self.create_mods_dir()?; // Make sure the mods directories exist afterwards
        Ok(())
    }

    pub fn get_mods(&self) -> impl Iterator<Item = &Rc<RefCell<Mod>>> {
        self.mods.values()
    }

    pub fn get_mod(&self, id: &str) -> Option<&Rc<RefCell<Mod>>> {
        self.mods.get(id)
    }

    /// Creates the mods, libs and early_mods directories, 
    /// and the Packages directory that stores the extracted QMODs for the current game version.
    fn create_mods_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.qmods_dir)?;
        std::fs::create_dir_all(LATE_MODS_DIR)?;
        std::fs::create_dir_all(EARLY_MODS_DIR)?;
        std::fs::create_dir_all(LIBS_DIR)?;
    
        Ok(())
    }
    
    /// Loads any mods from the QMODs directory that have not yet been loaded.
    pub fn load_mods(&mut self) -> Result<()> {
        let _lock = ModInstallLock::shared()?;

        self.create_mods_dir()?;
        self.mods.clear();
    
        for stat in std::fs::read_dir(&self.qmods_dir)? {
            let entry = match stat {
                Ok(entry) => entry,
                Err(_) => continue // Ignore innacessible mods
            };
    
            let mod_path = entry.path();
            if !entry.file_type()?.is_dir() {
                continue;
            }

            match self.load_mod_from_directory(mod_path.clone()) {
                Ok(loaded_mod) =>  if !self.mods.contains_key(&loaded_mod.manifest.id) {
                    self.mods.insert(loaded_mod.manifest.id.clone(), Rc::new(RefCell::new(loaded_mod)));
                }   else    {
                    warn!("Mod at {mod_path:?} had ID {}, but a mod with this ID already existed", loaded_mod.manifest.id);
                },
                Err(err) => {

                    warn!("Failed to load mod from {mod_path:?}: {err}");
                    // Attempt to delete the invalid mod
                    let _excl_lock: ModInstallLock = ModInstallLock::excl()?; // Need an exclusive lock as we are writing to the directory
                    match std::fs::remove_dir_all(&mod_path) {
                        Ok(_) => info!("Deleted invalid mod"),
                        Err(err) => warn!("Failed to delete invalid mod at {mod_path:?}: {err}")
                    }
                }
            };
        }

        // Load mods in legacy folder.
        // This is done last as loading the new qmods extracts them to the mods directory,
        // so the code above would lead to the mods being loaded again.
        if let Err(err) = self.load_old_qmods() {
            warn!("Failed to load legacy mods: {err}");
        }

        self.update_mods_status().context("Failed to check if mods installed after loading")?;
        Ok(())
    }

    /// Attempts to load QMODs found in the legacy ModsBeforeFriday directory.
    /// This will extract them in the new mods directory.
    /// The QMODs and the directory itself are then deleted.
    /// Will do nothing if the old mods directory does not exist.
    fn load_old_qmods(&mut self) -> Result<()> {
        if !Path::new(OLD_QMODS_DIR).exists() {
            return Ok(());
        }

        warn!("Migrating mods from legacy folder");
        // Writing to mods directory so need exclusive lock
        let _lock = ModInstallLock::excl()?;
        for stat_result in std::fs::read_dir(OLD_QMODS_DIR)
            .context("Failed to read old QMODs directory")? {
            let stat = stat_result?;

            let mod_stream = std::fs::File::open(stat.path())
                .context("Failed to open legacy mod")?;
            // Attempt to load a mod from each file
            match self.try_load_new_mod_internal(mod_stream) {
                Ok(new_mod) => info!("Successfully migrated legacy mod {new_mod}"),
                Err(err) => warn!("Failed to migrate legacy mod at {:?}: {}", stat.path(), err),
            }

            // Delete the file either way
            std::fs::remove_file(stat.path()).context("Failed to delete legacy mod")?;
        }
        std::fs::remove_dir(OLD_QMODS_DIR)?;

        Ok(())
    }

    fn load_mod_from_directory(&self, from: PathBuf) -> Result<Mod> {
        let manifest_path = from.join("mod.json");
        if !manifest_path.exists() {
            return Err(anyhow!("Mod at {from:?} had no mod.json manifest"));
        }

        let mut json_data = Vec::new();
        std::fs::File::open(manifest_path).context("Failed to open manifest")?
            .read_to_end(&mut json_data).context("Failed to read manifest")?;

        Ok(Mod {
            manifest: self.load_manifest_from_slice(&json_data).context("Failed to parse manifest")?,
            installed: false, // Must call update_mods_status
            loaded_from: from
        })
    }

    fn load_manifest_from_slice(&self, manifest_slice: &[u8]) -> Result<ModInfo> {
        let manifest_value = serde_json::from_slice::<serde_json::Value>(manifest_slice)?;
        // Check that the QMOD isn't a newer schema version than we support
        // NB: Validating against the schema will catch this, but we would like to provide a nicer error message
        match manifest_value.get("_QPVersion") {
            Some(serde_json::Value::String(schema_ver)) => {
                let sem_version = semver::Version::parse(&schema_ver)
                    .context("Failed to parse specified QMOD schema version")?;

                if sem_version > MAX_SCHEMA_VERSION {
                    return Err(anyhow!("QMOD specified schema version {sem_version} which was newer than the maximum supported version {MAX_SCHEMA_VERSION}. Is MBF out of date, or did the mod developer make a mistake?"));
                }
            },
            _ => return Err(anyhow!("Could not load mod as its manifest did not specify a QMOD schema version"))
        }

        // Now validate against the schema
        if let Err(errors) = self.schema.validate(&manifest_value) {
            let mut log_builder = String::new();

            for error in errors {
                log_builder.push_str(&format!("Validation error: {}\n", error));
                log_builder.push_str(&format!("Instance path: {}\n", error.instance_path));
            }

            return Err(anyhow!("QMOD schema validation failed: \n{log_builder}"))
        }

        Ok(serde_json::from_value(manifest_value)
            .expect("Failed to parse as QMOD manifest, despite being valid according to schema. This is a bug"))
    }

    /// Checks whether or not each loaded mod is installed.
    pub fn update_mods_status(&mut self) -> Result<()> {
        let _lock = ModInstallLock::shared()?;

        let early_mod_files = list_dir_files(EARLY_MODS_DIR)?;
        let late_mod_files = list_dir_files(LATE_MODS_DIR)?;
        let libraries = list_dir_files(LIBS_DIR)?;
    
        for r#mod in self.mods.values() {
            let mut mod_info = (**r#mod).borrow_mut();
            let manifest = &mod_info.manifest;
            let mod_files_present = manifest.mod_files.iter().all(|file| early_mod_files.contains(file))
                && manifest.library_files.iter().all(|file| libraries.contains(file))
                && manifest.late_mod_files.iter().all(|file| late_mod_files.contains(file))
                && manifest.file_copies.iter().map(|copy| &copy.destination)
                    .all(|dest| Path::new(dest).exists());

            mod_info.installed = mod_files_present;
        }
    
        Ok(())
    }

    /// Installs the mod with the given ID.
    /// This will install dependencies if necessary.
    pub fn install_mod(&mut self, id: &str) -> Result<()> {
        let _lock = ModInstallLock::excl()?;
        self.install_mod_internal(id)
    }

    fn install_mod_internal(&mut self, id: &str) -> Result<()> {
        // Install the mod's dependencies if applicable
        let mod_rc =  self.mods.get(id)
            .ok_or(anyhow!("Could not install mod with ID {id} as it did not exist"))?.clone();

        let to_install = (*mod_rc).borrow();
        if to_install.installed() {
            return Ok(());
        }

        info!("Installing {} v{}", to_install.manifest.id, to_install.manifest.version);

        for dep in &to_install.manifest.dependencies {
            match self.mods.get(&dep.id) {
                Some(existing_dep) => {
                    let dep_ref = (**existing_dep).borrow();
                    if !dep.version_range.matches(&dep_ref.manifest.version) {
                        info!("Dependency {} is out of date, got version {} but need {}", dep.id, dep_ref.manifest.version, dep.version_range);
                        drop(dep_ref);
                        self.install_dependency(&dep)?;
                    }   else if !dep_ref.installed && dep.required {
                        // Must install the dependency
                        info!("Dependency {} was not installed, reinstalling", dep.id);
                        drop(dep_ref);
                        self.install_mod_internal(&dep.id)?;
                    }
                },
                None => if dep.required {
                    info!("Dependency {} was not found: installing now", dep.id);
                    self.install_dependency(&dep)?;
                }
            }
        }
        drop(to_install);

        self.install_unchecked(&mut (*mod_rc).borrow_mut())?;
        Ok(())
    }

    /// Installs a mod without handling dependencies
    /// i.e. just copies the necessary files.
    fn install_unchecked(&self, to_install: &mut Mod) -> Result<()> {
        copy_stated_files(&to_install.loaded_from, &to_install.manifest.mod_files, EARLY_MODS_DIR)?;
        copy_stated_files(&to_install.loaded_from, &to_install.manifest.library_files, LIBS_DIR)?;
        copy_stated_files(&to_install.loaded_from, &to_install.manifest.late_mod_files, LATE_MODS_DIR)?;

        for file_copy in &to_install.manifest.file_copies {
            let file_path_in_mod = to_install.loaded_from.join(&file_copy.name);
            if !file_path_in_mod.exists() {
                warn!("Could not install file copy {} as it did not exist in the QMOD", file_copy.name);
                continue;
            }

            let dest_path = Path::new(&file_copy.destination);
            match dest_path.parent() {
                Some(parent) => std::fs::create_dir_all(parent)
                    .context("Failed to create destination directory for file copy")?,
                None => {}
            }

            if Path::new(&file_copy.destination).exists() {
                std::fs::remove_file(&file_copy.destination)
                    .context("Failed to remove existing copied file")?;
            }

            std::fs::copy(file_path_in_mod, &file_copy.destination)
                .context("Failed to copy stated file copy")?;
        }
        to_install.installed = true;

        Ok(())
    }

    /// Uninstalls a mod without handling dependencies
    /// i.e. just deletes the necessary files.
    fn uninstall_unchecked(&self, id: &str) -> Result<()> {
        // Gather a set of all library SOs being used by other mods.
        let mut retained_libs = HashSet::new();
        for (other_id, other_mod) in &self.mods {
            if other_id == id {
                continue;
            }

            for lib_path in other_mod.borrow()
                .manifest
                .library_files
                .iter() 
            {
                retained_libs.insert(get_so_name(&lib_path).to_string());
            }
        }

        let mut to_remove = (**self.mods.get(id).unwrap()).borrow_mut();
        delete_file_names(&to_remove.manifest.mod_files, HashSet::new(), EARLY_MODS_DIR)?;
        delete_file_names(&to_remove.manifest.late_mod_files, HashSet::new(), LATE_MODS_DIR)?;
        // Only delete libraries not in use (!)
        delete_file_names(&to_remove.manifest.library_files, retained_libs, LIBS_DIR)?;
        
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
    /// This will uninstall dependant mods if necessary.
    pub fn uninstall_mod(&self, id: &str) -> Result<()> {
        let _lock = ModInstallLock::excl()?;
        self.uninstall_mod_internal(id)
    }

    fn uninstall_mod_internal(&self, id: &str) -> Result<()> {
        let mod_rc = self.mods.get(id)
            .ok_or(anyhow!("Could not uninstall mod with ID {id} as it did not exist"))?
            .clone();
        let to_remove = (*mod_rc).borrow();
        if !to_remove.installed {
            return Ok(());
        }

        info!("Uninstalling {} v{}", to_remove.manifest.id, to_remove.manifest.version);
        drop(to_remove); // Avoid other code that needs the mod from panicking

        // Uninstall all depending mods
        for (other_id, m) in self.mods.iter() {
            if other_id == id {
                continue;
            }

            let m_ref = (**m).borrow();
            if m_ref.installed && m_ref.manifest.dependencies.iter().any(|dep| &dep.id == id && dep.required) {
                info!("Uninstalling (required) dependant mod {}", other_id);
                drop(m_ref);
                self.uninstall_mod_internal(other_id)?;
            }
        }

        self.uninstall_unchecked(id)?;
        Ok(())
    }

    fn install_dependency(&mut self, dep: &ModDependency) -> Result<()> {
        let link = match &dep.mod_link {
            Some(link) => link,
            None => return Err(anyhow!("Could not download dependency {}: no link given", dep.id))
        };

        info!("Downloading dependency from {}", link);
        let dependency_bytes = download_to_vec_with_attempts(&link)
            .context("Failed to download dependency")?;

        self.try_load_new_mod_internal(Cursor::new(dependency_bytes))?;
        self.install_mod_internal(&dep.id)?;
        Ok(())
    }

    /// Loads a new mod from a QMOD file.
    /// Returns the mod ID
    pub fn try_load_new_mod(&mut self, mod_stream: impl Read + Seek) -> Result<String> {
        let _lock = ModInstallLock::excl()?;

        self.try_load_new_mod_internal(mod_stream)
    }

    fn try_load_new_mod_internal(&mut self, mod_stream: impl Read + Seek) -> Result<String> {
        let mut zip = ZipFile::open(mod_stream).context("Mod was invalid ZIP archive")?;

        let json_data = zip.read_file("mod.json").context("Mod had no mod.json manifest")?;
        let loaded_mod_manifest = self.load_manifest_from_slice(&json_data)
            .context("Failed to parse manifest")?;

        // Check that upgrading the mod to the new version is actually safe...
        let id = loaded_mod_manifest.id.clone();
        if !self.check_dependency_compatibility(&id, &loaded_mod_manifest.version) {
            return Err(anyhow!("Could not upgrade {} to v{}", id, loaded_mod_manifest.version))
        }

        // Remove the existing version of the mod, 
        // unchecked as we don't want to nuke any dependant mods or any of its dependencies; we have established that the upgrade is safe.
        // by allowing remove_mod to run a regular uninstall
        if self.mods.contains_key(&id) {
            info!("Removing existing version of mod");
            self.uninstall_unchecked(&id)?;
        }
        self.remove_mod_internal(&id)?;

        // Extract the mod to the mods folder
        info!("Extracting {} v{}", loaded_mod_manifest.id, loaded_mod_manifest.version);
        let extract_path = self.get_mod_extract_path(&loaded_mod_manifest);
        std::fs::create_dir_all(&extract_path).context("Failed to create extract path")?;
        zip.extract_to_directory(&extract_path).context("Failed to extract QMOD file")?;

        // Insert the mod into the HashMap of loaded mods, and now it is ready to be manipulated by the mod manager!
        let loaded_mod = Mod {
            installed: false,
            manifest: loaded_mod_manifest,
            loaded_from: extract_path
        };
        self.mods.insert(id.clone(), Rc::new(RefCell::new(loaded_mod)));
        Ok(id)
    }

    // Finds a path to extract the mod with the given manifest.
    // This will have folder name {ID}_v{VERSION} unless a folder of this name already exists (which it shouldn't really )
    fn get_mod_extract_path(&self, manifest: &ModInfo) -> PathBuf {
        let mut i = 1;
        loop {
            let mut folder_name = format!("{}_v{}", manifest.id, manifest.version);
            if i > 1 {
                warn!("When finding path to extract {} v{}, the folder name {folder_name} was already occupied,
                    \n... despite no mod existing with the ID and version in the folder name. 
                    \nThis shouldn't cause anything bad but somebody is naming folders in a way that is very silly indeed!", manifest.id, manifest.version);

                folder_name.push('_');
                folder_name.push_str(&i.to_string());
            }

            let extract_path = Path::new(&self.qmods_dir).join(folder_name);
            if !extract_path.exists() {
                break extract_path
            }

            i += 1;
        }
    }

    // Fully deletes a mod from the Quest (uninstalling it if installed then deleting the extracted QMOD)
    pub fn remove_mod(&mut self, id: &str) -> Result<()> {
        let _lock = ModInstallLock::excl()?;

        self.remove_mod_internal(id)
    }

    fn remove_mod_internal(&mut self, id: &str) -> Result<()> {
        match self.mods.get(id) {
            Some(to_remove) => {
                let to_remove_ref: std::cell::Ref<Mod> = (**to_remove).borrow();
                let path_to_delete = to_remove_ref.loaded_from.clone();

                drop(to_remove_ref);
                self.uninstall_mod_internal(id)?;
                self.mods.remove(id);

                std::fs::remove_dir_all(path_to_delete)?;
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

fn get_so_name(path: &str) -> &str {
    path.split('/').last().unwrap()
}

// Deletes the files corresponding to the given SO files in a QMOD from the given folder
// (i.e. will only consider the SO file name, not the full path in the ZIP)
// `exclude` is a set of all of the SO file names that must not be deleted, (as they are being used by another mod).
fn delete_file_names(file_paths: &[String], exclude: HashSet<String>, within: impl AsRef<Path>) -> Result<()> {
    for path in file_paths {
        let file_name = get_so_name(path);
        if exclude.contains(file_name) {
            continue;
        }

        let stored_path = within.as_ref().join(file_name);
        if stored_path.exists() {
            std::fs::remove_file(stored_path)?;
        }
    }

    Ok(())
}

// Copies all of the files with names in `files` to the path `to/{file name not including directory in ZIP}`
fn copy_stated_files(mod_folder: impl AsRef<Path>, files: &[String], to: impl AsRef<Path>) -> Result<()> {
    for file in files {
        let file_location = mod_folder.as_ref().join(file);

        if !file_location.exists() {
            warn!("Could not install file {file} as it wasn't found in the QMOD");
            continue;
        }

        let file_name = file.split('/').last().unwrap();
        let copy_to = to.as_ref().join(file_name);

        if copy_to.exists() {
            std::fs::remove_file(&copy_to).context("Failed to remove existing mod file")?;
        }
        std::fs::copy(file_location, copy_to).context("Failed to copy SO")?;
    }

    Ok(())
}

fn list_dir_files(path: impl AsRef<Path>) -> Result<HashSet<String>> {
    Ok(std::fs::read_dir(&path)?.filter_map(|file| match file {
        Ok(file) => file.file_name().into_string().ok(),
        Err(_) => None
    }).collect())
}