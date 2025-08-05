//! The structure for each mod loaded by MBF.
//! This module is responsible for installing/removing the mod files and representing the loaded mods
//! but does not handle mod dependencies and other responsibilities - this is the job of the [ModManager](super::ModManager)

use std::{collections::HashSet, ffi::{OsStr, OsString}, path::{Path, PathBuf}};

use crate::parameters::PARAMETERS;

use super::{util, ModInfo};
use anyhow::{Result, Context};
use log::{debug, warn};


/// Represents a mod (in QMOD format).
#[derive(Debug)]
pub struct Mod {
    /// The `mod.json` manifest of the mod.
    manifest: ModInfo,
    /// Whether all mod files exist in their expected destinations
    /// Set immediately after loading a mod, no need to wait for other mods to be loaded.
    files_exist: bool,
    /// Whether or not the mod is installed, which is only considered `true` if all of its dependencies are also installed.
    /// This is optional as this can only be determined once all mods are loaded. It is None up until this point.
    pub(super) installed: Option<bool>,
    /// The folder that this mod was loaded from,.
    loaded_from: PathBuf,
    /// Whether the mod is core or any (transitive) required dependency of a core mod.
    pub(super) is_core: bool,
}

impl Mod {
    /// Gets whether or not the mod is currently considered to be installed.
    /// This is true if all of the mod's late mod files, library files, early mod files and file copies are all present
    /// AND the same is true for all dependencies of this mod, transitively.
    /// # Returns
    /// Whether the mod is installed.
    pub fn installed(&self) -> bool {
        self.installed.expect(
            "Mod install status should have been checked
            before mod was made available through public API",
        )
    }

    /// Gets a reference to the manifest of the mod.
    /// # Returns
    /// A reference to the manifest of the mod.
    pub fn manifest(&self) -> &ModInfo {
        &self.manifest
    }

    /// Gets a boolean indicating whether the mod is a core mod.
    /// NB: This value will be false until [ModManager::set_mod_core] is called with the ID of the mod OR the ID
    /// of any mod that depends on this mod with a required dependency (transitively).
    /// # Returns
    /// Whether the mod is a core mod.
    pub fn is_core(&self) -> bool {
        self.is_core
    }

    /// Creates a new [Mod] based on the loaded mod manifest and the directory containing the
    /// extracted QMOD file.
    pub(super) fn new(manifest: ModInfo, loaded_from: PathBuf) -> Result<Self> {
        Ok(Self {
            loaded_from,
            files_exist: Self::check_if_files_copied(&manifest).context("Checking if mod installed")?,
            manifest,
            installed: None,
            is_core: false
        })
    }

    /// # Returns
    /// True if and only if all mod files exist in their expected destinations.
    pub(super) fn files_exist(&self) -> bool {
        self.files_exist
    }

    /// Installs this mod by copying all of the necessary mod files/lib files/file copies to
    /// the modloader folders and marking it as installed.
    /// 
    /// Does not install dependencies, hence the "unchecked".
    pub(super) fn install_unchecked(&mut self) -> Result<()> {
        // Copy early mods, late mods and library binaries.
        util::copy_files_from_mod_folder(
            &self.loaded_from,
            &self.manifest().mod_files,
            &PARAMETERS.early_mods,
        )?;
        util::copy_files_from_mod_folder(
            &self.loaded_from,
            &self.manifest().library_files,
            &PARAMETERS.libs,
        )?;
        util::copy_files_from_mod_folder(
            &self.loaded_from,
            &self.manifest().late_mod_files,
            &PARAMETERS.late_mods,
        )?;

        self.copy_file_copies().context("Copying auxillary files")?;

        // Update the install status of the mod.
        self.installed = Some(true);
        self.files_exist = true;

        Ok(())
    }

    /// Uninstalls this mod by deleting all copied binary files and file copies and marking it
    /// as uninstalled.
    /// 
    /// Does not uninstall dependant mods, hence the "unchecked"
    pub(super) fn uninstall_unchecked(&mut self, retained_libs: HashSet<OsString>) -> Result<()> {
        // Delete all mod binary files.
        util::remove_file_names_from_folder(
            self.manifest().mod_files.iter(),
            &PARAMETERS.early_mods,
        )?;
        util::remove_file_names_from_folder(
            self.manifest().late_mod_files.iter(),
            &PARAMETERS.late_mods,
        )?;
        util::remove_file_names_from_folder(
            // Only delete libraries not in use (!)
            self
                .manifest()
                .library_files
                .iter()
                .filter(|lib_file| !retained_libs.contains(OsStr::new(lib_file))),
            &PARAMETERS.libs,
        )?;

        // Delete all file copies.
        for copy in &self.manifest().file_copies {
            let dest_path = Path::new(&copy.destination);
            if dest_path.exists() {
                debug!("Removing file copy at destination {dest_path:?}");
                std::fs::remove_file(dest_path).context("Deleting copied file")?;
            }
        }

        // Mark as uninstalled.
        self.installed = Some(false);
        self.files_exist = false;

        Ok(())
    }

    /// Deletes the mod, and will not check first whether it needs to be uninstalled.
    pub(super) fn delete_unchecked(self) -> Result<()> {
        std::fs::remove_dir_all(self.loaded_from).context("Deleting mod extract directory")?;
        Ok(())
    } 

    /// Copies all auxillary file copies in the manifest from the extracted mod to the required destination.
    fn copy_file_copies(&self) -> Result<()> {
        // TODO: Deny certain file copy destinations.

        for file_copy in &self.manifest().file_copies {
            let file_path_in_mod = self.loaded_from.join(&file_copy.name);
            if !file_path_in_mod.exists() {
                warn!(
                    "Could not install file copy {} as it did not exist in the QMOD",
                    file_copy.name
                );
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

            debug!(
                "Installing file copy {file_path_in_mod:?} to {}",
                file_copy.destination
            );
            std::fs::copy(file_path_in_mod, &file_copy.destination)
                .context("Copying stated file copy to destination")?;
        }

        Ok(())
    }

    /// Checks if the mod is installed, in the sense that all its mod binary files and file copies
    /// exist in their required destinations.
    /// # Arguments
    /// * `manifest` - The manifest of the mod to check the install status of.
    /// # Returns
    /// `true` if and only if all early mod files, late mod files, library files and file copies exist in their expected
    /// destinations.
    fn check_if_files_copied(manifest: &ModInfo) -> Result<bool> {
        Ok(
            util::files_exist_in_dir(&PARAMETERS.early_mods, manifest.mod_files.iter())?
                && util::files_exist_in_dir(&PARAMETERS.late_mods, manifest.late_mod_files.iter())?
                && util::files_exist_in_dir(&PARAMETERS.libs, manifest.library_files.iter())?
                && manifest
                    .file_copies
                    .iter()
                    .map(|copy| &copy.destination)
                    .all(|dest| Path::new(dest).exists()),
        )
    }
}