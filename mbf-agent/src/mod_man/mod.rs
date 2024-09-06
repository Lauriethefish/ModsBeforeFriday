mod manifest;
use std::{cell::RefCell, collections::{HashMap, HashSet}, fs::OpenOptions, io::{Cursor, Read, Seek}, path::{Path, PathBuf}, rc::Rc};

use jsonschema::JSONSchema;
use log::{debug, error, info, warn};
pub use manifest::*;

use anyhow::{Context, Result, anyhow};
use mbf_zip::ZipFile;
use semver::Version;

use crate::{downloads, EARLY_MODS_DIR, LATE_MODS_DIR, LIBS_DIR, NOMEDIA_PATH, OLD_QMODS_DIR};

const QMOD_SCHEMA: &str = include_str!("qmod_schema.json");
const MAX_SCHEMA_VERSION: Version = Version::new(1, 2, 0);

pub struct Mod {
    manifest: ModInfo,
    // Whether all mod files exist in their expected destinations
    // Set immediately after loading a mod.
    files_exist: bool,
    // Whether or not the mod is installed, which is only considered `true` if all of its dependencies are also installed.
    // This is optional as this can only be determined once all mods are loaded. It is None up until this point.
    installed: Option<bool>,
    loaded_from: PathBuf,
    // Whether the mod is core or a required dependency of a core mod (perhaps indirectly)
    is_core: bool
}


impl Mod {
    pub fn installed(&self) -> bool {
        self.installed.expect("Mod install status should have been checked
            before mod was made available through public API")
    }

    pub fn manifest(&self) -> &ModInfo {
        &self.manifest
    }

    pub fn is_core(&self) -> bool {
        self.is_core
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
            std::fs::remove_dir_all(path)?;
        }

        Ok(())
    }

    // Removes ALL mod and library files and deletes ALL mods from the current game.
    pub fn wipe_all_mods(&mut self) -> Result<()> {
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
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(NOMEDIA_PATH).context("Creating nomedia file")?;
    
        Ok(())
    }
    
    /// Loads any mods from the QMODs directory that have not yet been loaded.
    pub fn load_mods(&mut self) -> Result<()> {
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
        self.check_mods_installed().context("Checking if mods are installed")?;
        match self.load_old_qmods() {
            // If we had old QMODs loaded in this stage, then recheck if all mods are installed again, since the random load order of the legacy QMODs may mean
            // that if a dependency of a mod existed, it might not have been loaded
            // when the dependant mod was loaded.
            Ok(had_old_qmods) => if had_old_qmods {
                self.check_mods_installed()?;
            }
            Err(err) => warn!("Failed to load legacy mods: {err}")
        }

        Ok(())
    }

    /// Attempts to load QMODs found in the legacy ModsBeforeFriday directory.
    /// This will extract them in the new mods directory.
    /// The QMODs and the directory itself are then deleted.
    /// Will do nothing if the old mods directory does not exist.
    /// Returns true if any old QMODs were found
    fn load_old_qmods(&mut self) -> Result<bool> {
        if !Path::new(OLD_QMODS_DIR).exists() {
            return Ok(false);
        }

        warn!("Migrating mods from legacy folder");
        let mut found_qmod = false;
        for stat_result in std::fs::read_dir(OLD_QMODS_DIR)
            .context("Reading old QMODs directory")? {
            let stat = stat_result?;

            let mod_stream = std::fs::File::open(stat.path())
                .context("Opening legacy mod")?;
            debug!("Migrating {:?}", stat.path());

            // Attempt to load a mod from each file
            match self.try_load_new_mod(mod_stream) {
                Ok(new_mod) => info!("Successfully migrated legacy mod {new_mod}"),
                Err(err) => warn!("Failed to migrate legacy mod at {:?}: {}", stat.path(), err),
            }

            // Delete the file either way
            found_qmod = true;
            std::fs::remove_file(stat.path()).context("Deleting legacy mod")?;
        }
        std::fs::remove_dir(OLD_QMODS_DIR)?;

        Ok(found_qmod)
    }

    fn load_mod_from_directory(&self, from: PathBuf) -> Result<Mod> {
        let manifest_path = from.join("mod.json");
        if !manifest_path.exists() {
            return Err(anyhow!("Mod at {from:?} had no mod.json manifest"));
        }

        let mut json_data = Vec::new();
        std::fs::File::open(manifest_path).context("Opening manifest (mod.json) in mod folder.")?
            .read_to_end(&mut json_data).context("Reading manifest")?;

        let manifest = self.load_manifest_from_slice(&json_data).context("Parsing manifest as JSON")?;
        Ok(Mod {
            files_exist: Self::check_if_mod_files_exist(&manifest).context("Checking if mod files exist when loading mod")?,
            manifest,
            installed: None, // Must call update_mods_status
            loaded_from: from,
            is_core: false // Only set when the user of the ModManager calls set_core_mod
        })
    }

    fn load_manifest_from_slice(&self, manifest_slice: &[u8]) -> Result<ModInfo> {
        let manifest_value = serde_json::from_slice::<serde_json::Value>(manifest_slice)?;
        // Check that the QMOD isn't a newer schema version than we support
        // NB: Validating against the schema will catch this, but we would like to provide a nicer error message
        match manifest_value.get("_QPVersion") {
            Some(serde_json::Value::String(schema_ver)) => {
                let sem_version = semver::Version::parse(&schema_ver)
                    .context("Parsing specified QMOD schema (sem)version")?;

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

    /// Checks whether each loaded mod is installed.
    /// A mod is considered installed if:
    /// 1 - All mod files, late mod files, lib files and file copies exist in their expected destinations.
    /// 2 - All of its dependencies are installed and are within the expected version range
    /// This method uses the existing state of `Mod#files_exist` for each mod, i.e. it will not re-check whether the mod files exist for each mod.
    pub fn check_mods_installed(&mut self) -> Result<()> {
        debug!("Checking if mods are installed");
        // Set all mods to having no known install status
        // Required so that the recursive descent to check the install status of each mod is guaranteed to happen
        for mod_rc in self.mods.values() {
            mod_rc.borrow_mut().installed = None;
        }

        for mod_id in self.mods.keys() {
            let mut checked_in_pass = HashSet::new();
            self.check_mod_installed(mod_id, &mut checked_in_pass).context("Checking if individual mod was installed")?;
        }

        Ok(())
    }

    // Updates whether the mod with the given ID is currently installed.
    // This will recursively check whether any dependent mods are installed.
    // `checked_in_path` is used to detect recursive dependencies - which are not allowed and will trigger an error.
    // Returns the new state of Mod#installed for the mod.
    fn check_mod_installed(&self, id: &str, checked_in_pass: &mut HashSet<String>) -> Result<bool> {
        if !checked_in_pass.insert(id.to_string())   {
            return Err(anyhow!("Recursive dependency detected. Mod with ID {id} depends on itself, directly or indirectly. This is not permitted"));
        }

        let mod_rc = self.mods.get(id).ok_or(anyhow!("No mod with ID {id} found"))?;

        let mod_ref = mod_rc.borrow();
        let installed = self.check_mod_installed_internal(&*mod_ref, checked_in_pass)?;

        drop(mod_ref);
        mod_rc.borrow_mut().installed = Some(installed);

        Ok(installed)
    }

    // Does the actual checking of whether a mod is installed.
    // Returns whether the mod should be set as installed or not.
    fn check_mod_installed_internal(&self, mod_ref: &Mod, checked_in_path: &mut HashSet<String>) -> Result<bool> {
        // If the mod does not have its files in the necessary destinations, then this mod definitely is not installed, so no need to check dependencies.
        if !mod_ref.files_exist {
            return Ok(false);
        }

        for dependency in &mod_ref.manifest.dependencies {
            match self.get_mod(&dependency.id) {
                Some(dep_rc) => {
                    let dep_ref = dep_rc.borrow();

                    // Check if the dependency is within the required version range, if not then the mod definitely isn't installed.
                    if !dependency.version_range.matches(&dep_ref.manifest.version) {
                        return Ok(false);
                    }

                    // If the dependency exists and is within the required range, we need to verify that it is installed also
                    if !match dep_ref.installed {
                        None => {
                            drop(dep_ref);
                            self.check_mod_installed(&dependency.id, checked_in_path).context("Checking if dependency was installed")?
                        },
                        Some(installed) => installed
                    }  {
                        return Ok(false);
                    }

                },
                None => return Ok(false)
            }
        }

        Ok(true)
    }

    /// Checks whether the mod with the provided mod manifest has all of its
    /// file copies, mod, late_mod and lib files in their expected destinations,
    /// .. and updates this in the Mod structure.
    fn check_if_mod_files_exist(manifest: &ModInfo) -> Result<bool> {
        Ok(files_exist_in_dir(EARLY_MODS_DIR, manifest.mod_files.iter())?
            && files_exist_in_dir(LATE_MODS_DIR, manifest.late_mod_files.iter())?
            && files_exist_in_dir(LIBS_DIR, manifest.library_files.iter())?
            && manifest.file_copies.iter().map(|copy| &copy.destination)
                .all(|dest| Path::new(dest).exists()))
    }

    /// Installs the mod with the given ID.
    /// This will install dependencies if necessary.
    pub fn install_mod(&mut self, id: &str) -> Result<()> {
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
                    }   else if !dep_ref.installed() && dep.required {
                        // Must install the dependency
                        info!("Dependency {} was not installed, reinstalling", dep.id);
                        drop(dep_ref);
                        self.install_mod(&dep.id)?;
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
                    .context("Creating destination directory for file copy")?,
                None => {}
            }

            if Path::new(&file_copy.destination).exists() {
                std::fs::remove_file(&file_copy.destination)
                    .context("Removing existing copied file")?;
            }

            debug!("Installing file copy {file_path_in_mod:?} to {}", file_copy.destination);
            std::fs::copy(file_path_in_mod, &file_copy.destination)
                .context("Copying stated file copy to destination")?;
        }
        to_install.installed = Some(true);
        to_install.files_exist = true;

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

            // Only retain libs for installed mods.
            let other_mod_ref = other_mod.borrow();
            if !other_mod_ref.installed() {
                continue;
            }

            for lib_path in other_mod_ref
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
                debug!("Removing file copy at destination {dest_path:?}");
                std::fs::remove_file(dest_path).context("Deleting copied file")?;
            }
        }
        to_remove.installed = Some(false);
        to_remove.files_exist = false;

        Ok(())
    }

    /// Uninstalls the mod with the given ID.
    /// This will uninstall dependant mods if necessary
    pub fn uninstall_mod(&self, id: &str) -> Result<()> {
        let mod_rc = self.mods.get(id)
            .ok_or(anyhow!("Could not uninstall mod with ID {id} as it did not exist"))?
            .clone();
        let to_remove = (*mod_rc).borrow();
        if !to_remove.installed() {
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
            if m_ref.installed() && m_ref.manifest.dependencies.iter().any(|dep| &dep.id == id && dep.required) {
                info!("Uninstalling (required) dependant mod {}", other_id);
                drop(m_ref);
                self.uninstall_mod(other_id)?;
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
        let dependency_bytes = downloads::download_to_vec_with_attempts(&crate::get_dl_cfg(), &link)
            .context("Downloading dependency")?;

        self.try_load_new_mod(Cursor::new(dependency_bytes))?;
        self.install_mod(&dep.id)?;
        Ok(())
    }

    /// Loads a new mod from a QMOD file.
    /// Returns the mod ID
    pub fn try_load_new_mod(&mut self, mod_stream: impl Read + Seek) -> Result<String> {
        let mut zip = ZipFile::open(mod_stream).context("Mod was invalid ZIP archive")?;

        let json_data = zip.read_file("mod.json").context("Mod had no mod.json manifest")?;
        let loaded_mod_manifest = self.load_manifest_from_slice(&json_data)
            .context("Parsing manifest")?;

        debug!("Early load of new mod, ID {}, version: {}, author: {}", loaded_mod_manifest.id, loaded_mod_manifest.version, loaded_mod_manifest.author);

        // Check that upgrading the mod to the new version is actually safe...
        let id = loaded_mod_manifest.id.clone();
        if let Err(msg) = self.check_dependency_compatibility(&id, &loaded_mod_manifest.version) {
            return Err(anyhow!("Could not upgrade {} to v{}: {}", id, loaded_mod_manifest.version, msg))
        }

        // Remove the existing version of the mod, 
        // unchecked as we don't want to nuke any dependant mods or any of its dependencies; we have established that the upgrade is safe.
        // by allowing remove_mod to run a regular uninstall
        if self.mods.contains_key(&id) {
            info!("Removing existing version of mod");
            self.uninstall_unchecked(&id)?;
        }
        self.remove_mod(&id)?;

        // Extract the mod to the mods folder
        info!("Extracting {} v{}", loaded_mod_manifest.id, loaded_mod_manifest.version);
        let extract_path = self.get_mod_extract_path(&loaded_mod_manifest);
        debug!("Extract path: {extract_path:?}");
        std::fs::create_dir_all(&extract_path).context("Creating extract directory")?;
        zip.extract_to_directory(&extract_path).context("Extracting QMOD file")?;

        // Insert the mod into the HashMap of loaded mods, and now it is ready to be manipulated by the mod manager!
        let loaded_mod = Mod {
            installed: None,
            files_exist: Self::check_if_mod_files_exist(&loaded_mod_manifest)?,
            manifest: loaded_mod_manifest,
            loaded_from: extract_path,
            is_core: false
        };
        self.mods.insert(id.clone(), Rc::new(RefCell::new(loaded_mod)));

        let mut checked_in_pass = HashSet::new();
        self.check_mod_installed(&id, &mut checked_in_pass).context("Checking whether new mod was installed")?;
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

    pub fn remove_mod(&mut self, id: &str) -> Result<()> {
        match self.mods.get(id) {
            Some(to_remove) => {
                let to_remove_ref: std::cell::Ref<Mod> = (**to_remove).borrow();
                let path_to_delete = to_remove_ref.loaded_from.clone();

                drop(to_remove_ref);
                self.uninstall_mod(id)?;
                self.mods.remove(id);

                std::fs::remove_dir_all(path_to_delete)?;
                Ok(())
            },
            None => Ok(())
        }
    }

    /// Sets a particular mod ID as being a core mod.
    /// This will also make all dependencies of the mod core.
    /// Does nothing if the mod with the given ID doesn't exist or is already marked as core.
    pub fn set_mod_core(&self, id: &str) {
        if let Some(mod_rc) = self.mods.get(id) {
            let mut mod_ref = match mod_rc.try_borrow_mut() {
                Ok(mod_ref) => mod_ref,
                Err(_) => {
                    warn!("Failed to set mod as core as it was already borrowed: this is due to a cyclical dependency: {id} depends on itself");
                    return;
                }
            };

            mod_ref.is_core = true;

            // Recursively mark all dependencies as core.
            for dependency in &mod_ref.manifest.dependencies {
                if dependency.required {
                    self.set_mod_core(&dependency.id);
                }
            }
        }
    }

    // Checks that upgrading the dependency with ID dep_id to new_version will not result in an incompatibility with an existing installed mod.
    // Gives Err with a string containing the list of incompatibilities found, if any.
    // Gives Ok if no incompatibilities are found.
    // Also logs any issues discovered.
    fn check_dependency_compatibility(&self, dep_id: &str, new_version: &Version) -> Result<(), String> {
        let mut incompatibilities = String::new();
        let mut all_compatible = true;
        for (_, existing_mod) in &self.mods {
            let mod_ref = (**existing_mod).borrow();
            // We don't care about uninstalled mods, since they have no invariants to preserve.
            if !mod_ref.installed() {
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
                    let incompat_msg = format!("Mod {} depends on range {}", 
                        mod_ref.manifest.id,
                        existing_dep.version_range
                    );

                    error!("Cannot upgrade {dep_id} to {new_version}: {incompat_msg}");
                    // Append each message to the overall error.
                    incompatibilities.push_str(&incompat_msg);
                    incompatibilities.push('\n');
                },
                None => {}
            }
        }

        if all_compatible {
            Ok(())
        }   else {
            incompatibilities.pop();
            Err(incompatibilities)
        }
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
            debug!("Keeping lib {file_name} as it's used by another mod");
            continue;
        }

        let stored_path = within.as_ref().join(file_name);
        if stored_path.exists() {
            debug!("Removing {file_name}");
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

        debug!("Copying {file_name} to {copy_to:?}");

        if copy_to.exists() {
            std::fs::remove_file(&copy_to).context("Removing existing mod file")?;
        }
        std::fs::copy(file_location, copy_to).context("Copying SO for mod")?;
    }

    Ok(())
}

// For each file path in `file_names`:
// - Gets only the file name of this path if it has a stem.
// - Appends this file_name to `dir_path`.
// - Checks if the file exists
// Returns true iff all of the files in `file_names` exist within `dir_path` as above.
fn files_exist_in_dir(dir_path: impl AsRef<Path>, mut file_names: impl Iterator<Item = impl AsRef<Path>>) -> Result<bool> {
    let dir_path = dir_path.as_ref();
    Ok(file_names.all(|name| 
        dir_path.join(name.as_ref()
            .file_name()
            .expect("Mod file names should not be blank"))
            .exists()))
}