//! Module for mod management within MBF.

mod manifest;
mod util;
mod loaded_mod;

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ffi::OsString,
    fs::OpenOptions,
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
    rc::Rc,
};

use jsonschema::JSONSchema;
use log::{debug, error, info, warn};
pub use manifest::*;
pub use loaded_mod::Mod;

use anyhow::{anyhow, Context, Result};
use mbf_res_man::{
    external_res,
    models::{ModRepo, ModRepoMod},
    res_cache::ResCache,
};
use mbf_zip::ZipFile;
use semver::Version;

use crate::{downloads, parameters::PARAMETERS};

/// The JSON schema for the `mod.json` file within a qmod.
/// This is the same schema used by QuestPatcher.
const QMOD_SCHEMA: &str = include_str!("qmod_schema.json");
/// The maximum `_QPVersion` that MBF will accept in `mod.json`.
///
/// NB: The schema also checks that this property is an allowed value, however it is good
/// if we can detect a version that's too new/old manually to give a more helpful error message
/// than "schema validation failed."
const MAX_SCHEMA_VERSION: Version = Version::new(1, 2, 0);

/// A structure to manage QMODs installed on Beat Saber.
pub struct ModManager<'cache> {
    /// A map of mod IDs to mods.
    mods: HashMap<String, Rc<RefCell<Mod>>>,
    /// The JSON schema used to validate the QMOD manifest.
    schema: JSONSchema,
    /// The folder in which QMOD files are found given the current game version.
    qmods_dir: String,
    /// The full versionName of the current Beat Saber version.
    game_version: String,
    /// Resource cache for fetching the mod repo for downloading dependencies.
    res_cache: &'cache ResCache<'cache>,
    /// The mod repository used by MBF, if it has been loaded already.
    mod_repo: Option<ModRepo>,
}

impl<'cache> ModManager<'cache> {
    /// Creates a new [ModManager].
    ///
    /// # Arguments
    /// * `game_version` - The full `versionName` of the installed Beat Saber app.
    /// * `res_cache` - MBF resource cache.
    ///
    /// # Returns
    /// A new [ModManager] with specified parameters. The instance returned *will not* have mods loaded
    /// yet, and this must be done by invoking [ModManager::load_mods].
    pub fn new(game_version: String, res_cache: &'cache ResCache) -> Self {
        Self {
            mods: HashMap::new(),
            schema: JSONSchema::options()
                .compile(
                    &serde_json::from_str::<serde_json::Value>(QMOD_SCHEMA)
                        .expect("QMOD schema should be valid JSON"),
                )
                .expect("QMOD schema should be a valid JSON schema"),
            // Each game version stores its QMODs in a different directory.
            qmods_dir: (&PARAMETERS.qmods).replace('$', &game_version),
            game_version,
            res_cache,
            mod_repo: None,
        }
    }

    /// Removes ALL mod/early-mod and library files, ensuring that all installed mods are removed from the game.
    pub fn wipe_all_mods(&mut self) -> Result<()> {
        self.mods.clear();

        // Wipe all mod directories, if they exist.
        let to_remove = [
            &PARAMETERS.old_qmods,
            &PARAMETERS.late_mods,
            &PARAMETERS.early_mods,
            &PARAMETERS.libs,
            &self.qmods_dir,
        ];
        for path in to_remove {
            let path = Path::new(path);
            if path.exists() {
                std::fs::remove_dir_all(path).context("Failed to delete mod folder")?;
            }
        }

        // Ensure that the mods directories exist after the operation.
        self.create_mods_dir()?;
        Ok(())
    }

    /// Gets all loaded mods.
    /// # Returns
    /// An iterator over all currently loaded mods.
    pub fn get_mods(&self) -> impl Iterator<Item = &Rc<RefCell<Mod>>> {
        self.mods.values()
    }

    /// Gets a mod by its ID.
    /// # Arguments
    /// * `id` - The ID of the mod to fetch.
    /// # Returns
    /// The mod with ID `id`, or [Option::None] if no such mod existed.
    pub fn get_mod(&self, id: &str) -> Option<&Rc<RefCell<Mod>>> {
        self.mods.get(id)
    }

    /// Loads the installed mods from the [paths::QMODS] directory in ModData.
    ///
    /// Also loads any legacy (non-extracted) mods found in the [paths::OLD_QMODS] directory,
    /// if the directory exists, and extracts them to the new path. [paths::OLD_QMODS] is then deleted.
    pub fn load_mods(&mut self) -> Result<()> {
        self.create_mods_dir()?;
        self.mods.clear();

        for stat in std::fs::read_dir(&self.qmods_dir)? {
            let entry = match stat {
                Ok(entry) => entry,
                Err(_) => continue, // Ignore innacessible mods
            };

            let mod_path = entry.path();
            if !entry.file_type()?.is_dir() {
                continue;
            }

            match self.load_mod_from_directory(mod_path.clone()) {
                Ok(loaded_mod) => {
                    if !self.mods.contains_key(&loaded_mod.manifest().id) {
                        self.mods.insert(
                            loaded_mod.manifest().id.clone(),
                            Rc::new(RefCell::new(loaded_mod)),
                        );
                    } else {
                        warn!(
                            "Mod at {mod_path:?} had ID {}, but a mod with this ID already existed",
                            loaded_mod.manifest().id
                        );
                    }
                }
                Err(err) => {
                    warn!("Failed to load mod from {mod_path:?}: {err}");
                    // Attempt to delete the invalid mod
                    match std::fs::remove_dir_all(&mod_path) {
                        Ok(_) => info!("Deleted invalid mod"),
                        Err(err) => warn!("Failed to delete invalid mod at {mod_path:?}: {err}"),
                    }
                }
            };
        }

        // Load mods in legacy folder.
        // This is done last as loading the new qmods extracts them to the mods directory,
        // so the code above would lead to the mods being loaded again.
        self.check_mods_installed()
            .context("Checking if mods are installed")?;
        match self.load_old_qmods() {
            // If we had old QMODs loaded in this stage, then recheck if all mods are installed again, since the random load order of the legacy QMODs may mean
            // that if a dependency of a mod existed, it might not have been loaded
            // when the dependant mod was loaded.
            Ok(had_old_qmods) => {
                if had_old_qmods {
                    self.check_mods_installed()?;
                }
            }
            Err(err) => warn!("Failed to load legacy mods: {err}"),
        }

        Ok(())
    }

    /// Checks whether each loaded mod is installed.
    /// A mod is considered installed if:
    /// 1 - All mod files, late mod files, lib files and file copies exist in their expected destinations.
    /// 2 - All of its dependencies are installed and are within the expected version range
    pub fn check_mods_installed(&mut self) -> Result<()> {
        debug!("Checking if mods are installed");
        // Set all mods to having no known install status
        // Required so that the recursive descent to check the install status of each mod is guaranteed to happen
        for mod_rc in self.mods.values() {
            mod_rc.borrow_mut().installed = None;
        }

        for mod_id in self.mods.keys() {
            let mut checked_in_pass = HashSet::new();
            self.check_mod_installed(mod_id, &mut checked_in_pass)
                .context("Checking if individual mod was installed")?;
        }

        Ok(())
    }

    /// Installs the mod with the given ID.
    /// This will install/upgrade dependencies if necessary.
    /// If the mod is already installed (see [Mod::installed]), this does nothing.
    /// # Arguments
    /// * `id` the ID of the mod to install.
    /// # Returns
    /// `Ok` if all mod files were copied successfully and all dependencies were installed correctly.
    /// Reasons for failure could include:
    /// - A dependency conflict (a dependency needs to be upgraded to install the mod but another mod will not allow
    /// the newer version to be installed.)
    /// - `id` is not the ID of an installed mod.
    ///
    /// This function will NOT fail if the mod is missing one of its stated mod/lib/late_mod files, but will instead
    /// log a warning.
    pub fn install_mod(&mut self, id: &str) -> Result<()> {
        // Install the mod's dependencies if applicable
        let mod_rc = self
            .mods
            .get(id)
            .ok_or(anyhow!(
                "Could not install mod with ID {id} as it did not exist"
            ))?
            .clone();

        let to_install = (*mod_rc).borrow();
        if to_install.installed() {
            return Ok(());
        }

        info!(
            "Installing {} v{}",
            to_install.manifest().id, to_install.manifest().version
        );

        for dep in &to_install.manifest().dependencies {
            match self.mods.get(&dep.id) {
                Some(existing_dep) => {
                    let dep_ref = (**existing_dep).borrow();
                    if !dep.version_range.matches(&dep_ref.manifest().version) {
                        info!(
                            "Dependency {} is out of date, got version {} but need {}",
                            dep.id, dep_ref.manifest().version, dep.version_range
                        );
                        drop(dep_ref);
                        self.install_dependency(&dep)?;
                    } else if !dep_ref.installed() && dep.required {
                        // Must install the dependency
                        info!("Dependency {} was not installed, reinstalling", dep.id);
                        drop(dep_ref);
                        self.install_mod(&dep.id)?;
                    }
                }
                None => {
                    if dep.required {
                        info!("Dependency {} was not found: installing now", dep.id);
                        self.install_dependency(&dep)?;
                    }
                }
            }
        }
        drop(to_install);

        mod_rc.borrow_mut().install_unchecked()?;
        Ok(())
    }

    /// Uninstalls the mod with the given ID.
    /// This process will uninstall any mods that depend on the mod with a required dependency,.
    ///
    /// This does nothing if the mod is not already installed (see [Mod::installed])
    /// # Arguments
    /// * `id` - the ID of the mod to uninstall.
    /// # Returns
    /// `Ok` if the operation succeeded. Failure should only happen in case of an IO error,
    /// which should not happen as the MBF process should have enough permissions to access the mod directories.
    pub fn uninstall_mod(&self, id: &str) -> Result<()> {
        let mod_rc = self
            .mods
            .get(id)
            .ok_or(anyhow!(
                "Could not uninstall mod with ID {id} as it did not exist"
            ))?
            .clone();
        let to_remove = (*mod_rc).borrow();
        if !to_remove.installed() {
            return Ok(());
        }

        info!(
            "Uninstalling {} v{}",
            to_remove.manifest().id, to_remove.manifest().version
        );
        drop(to_remove); // Avoid other code that needs the mod from panicking

        // Uninstall all depending mods
        for (other_id, m) in self.mods.iter() {
            if other_id == id {
                continue;
            }

            let m_ref = (**m).borrow();
            if m_ref.installed()
                && m_ref
                    .manifest()
                    .dependencies
                    .iter()
                    .any(|dep| &dep.id == id && dep.required)
            {
                info!("Uninstalling (required) dependant mod {}", other_id);
                drop(m_ref);
                self.uninstall_mod(other_id)?;
            }
        }

        mod_rc.borrow_mut().uninstall_unchecked(self.get_retained_lib_files(id))
            .context("Uninstalling unchecked")?;
        Ok(())
    }

    /// Attempts to load a new QMOD from a stream.
    /// This will load the mod as a ZIP and validate its manifest.
    ///
    /// If a mod with this mod's ID already exists in the [ModManager], this function checks, for all mods that are dependant
    /// on the ID, that the new mod version matches their dependency constraints.
    /// If this is not the case, the operation fails.
    /// If this is the case, the existing mod will be overwritten with the new mod.
    ///
    /// The mod will not be automatically installed, [ModManager::install_mod] must be called separately.
    /// # Arguments
    /// * `mod_stream` - A readable and seekable stream to load the mod from.
    /// # Returns
    /// If successful, the ID of the loaded mod.
    pub fn try_load_new_mod(&mut self, mod_stream: impl Read + Seek) -> Result<String> {
        let mut zip = ZipFile::open(mod_stream).context("Mod was invalid ZIP archive")?;

        let json_data = zip
            .read_file("mod.json")
            .context("Mod had no mod.json manifest")?;
        let loaded_mod_manifest = self
            .load_manifest_from_slice(&json_data)
            .context("Parsing manifest")?;

        debug!(
            "Early load of new mod, ID {}, version: {}, author: {}",
            loaded_mod_manifest.id, loaded_mod_manifest.version, loaded_mod_manifest.author
        );

        // Check that upgrading the mod to the new version is actually safe...
        let id = loaded_mod_manifest.id.clone();
        if let Err(msg) = self.check_dependency_compatibility(&id, &loaded_mod_manifest.version) {
            return Err(anyhow!(
                "Could not upgrade {} to v{}: {}",
                id,
                loaded_mod_manifest.version,
                msg
            ));
        }

        // Remove the existing version of the mod,
        // unchecked as we don't want to nuke any dependant mods or any of its dependencies; we have established that the upgrade is safe.
        // by allowing remove_mod to run a regular uninstall
        if let Some(existing_mod) = self.mods.get(&id) {
            info!("Removing existing version of mod");
            existing_mod.borrow_mut().uninstall_unchecked(self.get_retained_lib_files(&id))
                .context("Uninstalling existing mod")?;
        }
        self.remove_mod(&id)?;

        // Extract the mod to the mods folder
        info!(
            "Extracting {} v{}",
            loaded_mod_manifest.id, loaded_mod_manifest.version
        );
        let extract_path = self.get_mod_extract_path(&loaded_mod_manifest);
        debug!("Extract path: {extract_path:?}");
        std::fs::create_dir_all(&extract_path).context("Creating extract directory")?;
        zip.extract_to_directory(&extract_path)
            .context("Extracting QMOD file")?;

        // Insert the mod into the HashMap of loaded mods, and now it is ready to be manipulated by the mod manager!
        let loaded_mod = Mod::new(loaded_mod_manifest, extract_path).context("Creating Mod")?;
        self.mods
            .insert(id.clone(), Rc::new(RefCell::new(loaded_mod)));

        let mut checked_in_pass = HashSet::new();
        self.check_mod_installed(&id, &mut checked_in_pass)
            .context("Checking whether new mod was installed")?;
        Ok(id)
    }

    /// Uninstalls (if installed) and deletes the mod with the specified ID.
    /// If no mod exists with this ID, then the operation does nothing.
    ///
    /// This will uninstall any mods with a required dependency on this mod and any of their dependencies transitively.
    /// # Arguments
    /// * `id` - the ID of the mod to delete.
    pub fn remove_mod(&mut self, id: &str) -> Result<()> {
        if self.mods.contains_key(id) {
            self.uninstall_mod(id)?;
            let to_remove = self.mods.remove(id).unwrap();
            let owned_mod = Rc::try_unwrap(to_remove)
                .expect("Should be only one reference")
                .into_inner();

            owned_mod.delete_unchecked().context("Deleting mod files")?;
        }

        Ok(())
    }

    /// Sets a particular mod ID as being a core mod.
    /// This will also make all required dependencies of the mod core, transitively.
    /// Does nothing if the mod with the given ID doesn't exist or is already marked as core.
    /// # Arguments
    /// * `id` - The ID of the mod to set as core.
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
            for dependency in &mod_ref.manifest().dependencies {
                if dependency.required {
                    self.set_mod_core(&dependency.id);
                }
            }
        }
    }

    /// Attempts to load QMODs found in the legacy ModsBeforeFriday directory.
    /// This will extract them in the new mods directory.
    /// The QMODs and the directory itself are then deleted.
    /// Will do nothing if the old mods directory does not exist.
    /// Returns true if any old QMODs were found
    fn load_old_qmods(&mut self) -> Result<bool> {
        if !Path::new(&PARAMETERS.old_qmods).exists() {
            return Ok(false);
        }

        warn!("Migrating mods from legacy folder");
        let mut found_qmod = false;
        for stat_result in
            std::fs::read_dir(&PARAMETERS.old_qmods).context("Reading old QMODs directory")?
        {
            let stat = stat_result?;

            let mod_stream = std::fs::File::open(stat.path()).context("Opening legacy mod")?;
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
        std::fs::remove_dir(&PARAMETERS.old_qmods)?;

        Ok(found_qmod)
    }

    fn load_mod_from_directory(&self, from: PathBuf) -> Result<Mod> {
        let manifest_path = from.join("mod.json");
        if !manifest_path.exists() {
            return Err(anyhow!("Mod at {from:?} had no mod.json manifest"));
        }

        let mut json_data = Vec::new();
        std::fs::File::open(manifest_path)
            .context("Opening manifest (mod.json) in mod folder.")?
            .read_to_end(&mut json_data)
            .context("Reading manifest")?;

        let manifest = self
            .load_manifest_from_slice(&json_data)
            .context("Parsing manifest as JSON")?;
        Ok(Mod::new(manifest, from).context("Creating Mod")?)
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
            }
            _ => {
                return Err(anyhow!(
                    "Could not load mod as its manifest did not specify a QMOD schema version"
                ))
            }
        }

        // Now validate against the schema
        if let Err(errors) = self.schema.validate(&manifest_value) {
            let mut log_builder = String::new();

            for error in errors {
                log_builder.push_str(&format!("Validation error: {}\n", error));
                log_builder.push_str(&format!("Instance path: {}\n", error.instance_path));
            }

            return Err(anyhow!("QMOD schema validation failed: \n{log_builder}"));
        }

        Ok(serde_json::from_value(manifest_value)
            .expect("Failed to parse as QMOD manifest, despite being valid according to schema. This is a bug"))
    }

    /// Used to avoid removing library files that are still in use by another mod when uninstalling a mod.
    /// # Arguments
    /// * `uninstalling_id` - The ID of the mod that is being uninstalled.
    /// Library files belonging to this mod ID will NOT be included.
    /// # Returns
    /// A HashSet of the file name (with extension) of all library files in use by an installed mod that DOES NOT
    /// have ID `uninstalling_id`.
    fn get_retained_lib_files(&self, uninstalling_id: &str) -> HashSet<OsString> {
        let mut retained_libs: HashSet<OsString> = HashSet::new();
        for (id, other_mod) in &self.mods {
            if id == uninstalling_id {
                continue;
            }

            // Only retain libs for installed mods.
            let other_mod_ref = other_mod.borrow();
            if !other_mod_ref.installed() {
                continue;
            }

            for lib_path in other_mod_ref.manifest().library_files.iter() {
                if let Some(file_name) = Path::new(lib_path).file_name() {
                    retained_libs.insert(file_name.to_owned());
                }
            }
        }

        retained_libs
    }

    fn install_dependency(&mut self, dep: &ModDependency) -> Result<()> {
        // First check if we can find a copy of the dependency in the mod repo, since this is the preferred option
        // The mod repo will likely have a more up-to-date version of the dependency than the dependency downloadIfMissing
        let link = if let Some(dep_url) = self.try_get_dep_from_mod_repo(dep) {
            dep_url
        } else {
            match &dep.mod_link {
                Some(link) => link.clone(),
                None => return Err(anyhow!("Could not download dependency {}: no link given and could not find in mod repo", dep.id))
            }
        };

        info!("Downloading dependency from {}", link);
        let dependency_bytes =
            downloads::download_to_vec_with_attempts(&crate::get_dl_cfg(), &link)
                .context("Downloading dependency")?;

        self.try_load_new_mod(Cursor::new(dependency_bytes))?;
        self.install_mod(&dep.id)?;
        Ok(())
    }

    // Checks the mod repository for the latest dependency matching `dep`
    // Returns the URL of the dependency if found
    // Gives None if the mod repository could not be accessed, or no matching mod was found in the repository.
    fn try_get_dep_from_mod_repo(&mut self, dep: &ModDependency) -> Option<String> {
        let mut latest_dep: Option<ModRepoMod> = None;
        let mut update_with_latest = |repo_mod: &ModRepoMod| {
            if repo_mod.id == dep.id && dep.version_range.matches(&repo_mod.version) {
                // Check if the current latest mod is newer than this one, stop if so
                match latest_dep.as_ref() {
                    Some(existing_latest) => {
                        if existing_latest.version > repo_mod.version {
                            return;
                        }
                    }
                    None => {}
                }

                latest_dep = Some(repo_mod.clone())
            }
        };

        // Borrow checker limitation: get_or_load_mod_repo returns an immutable reference but the borrow
        // checker still treats self as being mutably borrowed, so we can't access the game version.
        let game_ver_clone = self.game_version.clone();

        match self.get_or_load_mod_repo() {
            Ok(mod_repo) => {
                // Consider both global mods, which work on any game version, and the game version specific mods for our version

                if let Some(global_mods) = mod_repo.get("global") {
                    global_mods.iter().for_each(&mut update_with_latest);
                }

                if let Some(version_mods) = mod_repo.get(&game_ver_clone) {
                    version_mods.iter().for_each(&mut update_with_latest);
                }

                match latest_dep {
                    Some(dep) => {
                        info!(
                            "Found download URL for {} v{} in mod repo",
                            dep.id, dep.version
                        );
                        Some(dep.download)
                    }
                    None => {
                        debug!(
                            "Mod repo had no matching dependency for {} range {}",
                            dep.id, dep.version_range
                        );
                        None
                    }
                }
            }
            Err(err) => {
                warn!(
                    "Could not check for latest {} range {} from mod repo: {err}",
                    dep.id, dep.version_range
                );
                None
            }
        }
    }

    /// Updates whether the mod with the given ID is currently installed.
    /// This will recursively check whether any dependent mods are installed.
    // `checked_in_path` is used to detect recursive dependencies - which are not allowed and will trigger an error.
    // Returns the new state of Mod#installed for the mod.
    fn check_mod_installed(&self, id: &str, checked_in_pass: &mut HashSet<String>) -> Result<bool> {
        if !checked_in_pass.insert(id.to_string()) {
            return Err(anyhow!("Recursive dependency detected. Mod with ID {id} depends on itself, directly or indirectly. This is not permitted"));
        }

        let mod_rc = self
            .mods
            .get(id)
            .ok_or(anyhow!("No mod with ID {id} found"))?;

        let mod_ref = mod_rc.borrow();
        let installed = self.check_mod_installed_internal(&*mod_ref, checked_in_pass)?;

        drop(mod_ref);
        mod_rc.borrow_mut().installed = Some(installed);

        Ok(installed)
    }

    // Does the actual checking of whether a mod is installed.
    // Returns whether the mod should be set as installed or not.
    fn check_mod_installed_internal(
        &self,
        mod_ref: &Mod,
        checked_in_path: &mut HashSet<String>,
    ) -> Result<bool> {
        // If the mod does not have its files in the necessary destinations, then this mod definitely is not installed, so no need to check dependencies.
        if !mod_ref.files_exist() {
            return Ok(false);
        }

        for dependency in &mod_ref.manifest().dependencies {
            match self.get_mod(&dependency.id) {
                Some(dep_rc) => {
                    let dep_ref = dep_rc.borrow();

                    // If the dependency exists and is within the required range, we need to verify that it is installed also
                    if !match dep_ref.installed {
                        None => {
                            drop(dep_ref);
                            self.check_mod_installed(&dependency.id, checked_in_path)
                                .context("Checking if dependency was installed")?
                        }
                        Some(installed) => installed,
                    } {
                        // If dependency not installed, mod is installed only if the dependency is optional (i.e. non required)
                        return Ok(!dependency.required);
                    }   else if !dependency.version_range.matches(&dep_rc.borrow().manifest().version) { // If dependency is installed
                        // If dependency not within required range needs fixing no matter if the dependency is required.
                        return Ok(false);
                    }
                }
                None => return Ok(!dependency.required), // If dependency not present, the mod is still installed if the dependency isn't required
            }
        }

        Ok(true)
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
                break extract_path;
            }

            i += 1;
        }
    }

    /// Creates the mods, libs and early_mods directories,
    /// and the Packages directory that stores the extracted QMODs for the current game version.
    fn create_mods_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.qmods_dir)?;
        std::fs::create_dir_all(&PARAMETERS.late_mods)?;
        std::fs::create_dir_all(&PARAMETERS.early_mods)?;
        std::fs::create_dir_all(&PARAMETERS.libs)?;
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(&PARAMETERS.moddata_nomedia)
            .context("Creating nomedia file")?;

        Ok(())
    }

    // Checks that upgrading the dependency with ID dep_id to new_version will not result in an incompatibility with an existing installed mod.
    // Gives Err with a string containing the list of incompatibilities found, if any.
    // Gives Ok if no incompatibilities are found.
    // Also logs any issues discovered.
    fn check_dependency_compatibility(
        &self,
        dep_id: &str,
        new_version: &Version,
    ) -> Result<(), String> {
        let mut incompatibilities = String::new();
        let mut all_compatible = true;
        for (_, existing_mod) in &self.mods {
            let mod_ref = (**existing_mod).borrow();
            // We don't care about uninstalled mods, since they have no invariants to preserve.
            if !mod_ref.installed() {
                continue;
            }

            match mod_ref
                .manifest()
                .dependencies
                .iter()
                .filter(|existing_dep| existing_dep.id == dep_id)
                .next()
            {
                Some(existing_dep) => {
                    if !existing_dep.version_range.matches(new_version) {
                        all_compatible = false;
                        let incompat_msg = format!(
                            "Mod {} depends on range {}",
                            mod_ref.manifest().id, existing_dep.version_range
                        );

                        error!("Cannot upgrade {dep_id} to {new_version}: {incompat_msg}");
                        // Append each message to the overall error.
                        incompatibilities.push_str(&incompat_msg);
                        incompatibilities.push('\n');
                    }
                }
                None => {}
            }
        }

        if all_compatible {
            Ok(())
        } else {
            incompatibilities.pop();
            Err(incompatibilities)
        }
    }

    /// Downloads (or loads from the cache), the mod repo and caches it to `self.mod_repo`
    /// # Returns
    /// A reference to the loaded mod repo, if successful.
    fn get_or_load_mod_repo(&mut self) -> Result<&ModRepo> {
        if self.mod_repo.is_none() {
            self.mod_repo =
                Some(external_res::get_mod_repo(&self.res_cache).context("Downloading mod repo")?)
        }

        Ok(self.mod_repo.as_ref().expect("Just loaded mod repo"))
    }
}
