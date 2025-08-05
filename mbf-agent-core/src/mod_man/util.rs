//! Utilities for mod management.

use anyhow::{Context, Result};
use log::{debug, warn};
use std::path::Path;

/// Checks if all files with the specified file names exist within a directory.
///
/// # Arguments
/// * `dir_path` - Path to the directory to check.
/// * `file_paths` - An iterator over the file paths within `dir_path` to check the existance of.
/// This function will remove all components of each path in `file_paths` except the component after the last slash.
/// (i.e. it only uses the file name)
///
/// # Returns
/// `Ok(true)` if all required files existed, `Ok(false)` if at least one file did not exist,
/// or an `Err` of an IO error occured.
pub(super) fn files_exist_in_dir(
    dir_path: impl AsRef<Path>,
    mut file_paths: impl Iterator<Item = impl AsRef<Path>>,
) -> Result<bool> {
    let dir_path = dir_path.as_ref();
    Ok(file_paths.all(|name| {
        dir_path
            .join(
                name.as_ref()
                    .file_name()
                    .expect("Mod file names should not be blank"),
            )
            .exists()
    }))
}

/// Used to copy stated mods/libs/early_mod files from a mod folder to the modloader folder.
/// # Arguments
/// * `mod_folder` - The folder that contains the SO files for the mod being installed.
/// * `files` - The paths of the mods/libs/early_mod files within `mod_folder`.
/// These paths will be appended to `mod_folder` to get the full file path.
/// * `modloader_folder` - The destination to copy the files to.
/// For each file in `files`, the file name of the file (i.e. last path segment) is joined after `modloader_folder` to get the destination path.
pub(super) fn copy_files_from_mod_folder(
    mod_folder: impl AsRef<Path>,
    files: &[impl AsRef<Path>],
    modloader_folder: impl AsRef<Path>,
) -> Result<()> {
    for file in files {
        let file = file.as_ref();
        let file_location = mod_folder.as_ref().join(file);

        if !file_location.exists() {
            warn!("Could not install file {file:?} as it wasn't found in the QMOD");
            continue;
        }

        let file_name = file
            .file_name()
            .context("Mod file should have a file name")?;
        let copy_to = modloader_folder.as_ref().join(file_name);

        debug!("Copying {file_name:?} to {copy_to:?}");

        if copy_to.exists() {
            std::fs::remove_file(&copy_to).context("Removing existing mod file")?;
        }
        std::fs::copy(file_location, copy_to).context("Copying SO for mod")?;
    }

    Ok(())
}

/// Removes all files within a specified folder that have the same file name as one
/// of the files specified.
/// # Arguments
/// * `from` - Path to the directory to remove files from.
/// * `file_paths` - An iterator over file paths. For each path, the file name is determined,
/// joined to the end of `from` and the file at this path is deleted if it exists.
pub(super) fn remove_file_names_from_folder(
    file_paths: impl Iterator<Item = impl AsRef<Path>>,
    from: impl AsRef<Path>,
) -> Result<()> {
    for path in file_paths {
        if let Some(file_name) = path.as_ref().file_name() {
            let stored_path = from.as_ref().join(file_name);
            if stored_path.exists() {
                debug!("Removing {file_name:?}");
                std::fs::remove_file(stored_path)?;
            }
        }
    }

    Ok(())
}
