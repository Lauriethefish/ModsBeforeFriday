use std::path::{Path, PathBuf};

use crate::{
    downloads,
    mod_man::ModManager,
    models::response::{self, ImportResultType, Response}, parameters::PARAMETERS
};
use anyhow::{anyhow, Context, Result};
use log::{debug, info, warn};
use mbf_zip::ZipFile;

/// Handles `ImportUrl` [Requests](requests::Request).
///
/// # Returns
/// The [Response](requests::Response) to the request (variant `ImportResult`)
pub(super) fn handle_import_mod_url(from_url: String) -> Result<Response> {
    std::fs::create_dir_all(&PARAMETERS.mbf_downloads)?;
    let download_path = Path::new(&PARAMETERS.mbf_downloads).join("import_from_url");

    info!("Downloading {}", from_url);
    let filename: Option<String> =
        downloads::download_file_with_attempts(&crate::get_dl_cfg(), &download_path, &from_url)?;

    // Attempt to import the downloaded file as a qmod, removing the temporary file if this fails.
    handle_import(&download_path, filename)
}

/// Handles `Import` [Requests](requests::Request).
///
/// # Returns
/// The [Response](requests::Response) to the request (variant `ImportResult`)
pub(super) fn handle_import(
    from_path: impl AsRef<Path> + std::fmt::Debug,
    override_filename: Option<String>,
) -> Result<Response> {
    // Load the installed mods.
    let res_cache = crate::load_res_cache()?;
    let mut mod_manager = ModManager::new(super::get_app_version_only()?, &res_cache);
    mod_manager.load_mods()?;

    let filename = match override_filename {
        Some(filename) => filename,
        None => from_path
            .as_ref()
            .file_name()
            .ok_or(anyhow!("No filename in {from_path:?}"))?
            .to_string_lossy()
            .to_string(),
    };

    let path = from_path.as_ref().to_owned();
    info!("Attempting to import from {filename}");

    let file_ext = filename
        .split('.')
        .rev()
        .next()
        .ok_or(anyhow!("No file extension in filename {filename}"))?
        .to_string()
        .to_lowercase();

    let import_result = if file_ext == "qmod" {
        handle_import_qmod(mod_manager, path.clone())
    } else if file_ext == "zip" {
        attempt_song_import(path.clone())
    } else if file_ext == "dll" {
        // This is a PC mod file, so delete it and return this fact to the importer.
        std::fs::remove_file(path.clone()).context("Removing temporary upload file")?;
        Ok(response::ImportResultType::NonQuestModDetected)
    } else {
        attempt_file_copy(path.clone(), file_ext, mod_manager)
    };

    // Make sure to remove the temporary file in the case that importing the file failed.
    match import_result {
        Ok(result) => Ok(Response::ImportResult {
            result,
            used_filename: filename,
        }),
        Err(err) => {
            match std::fs::remove_file(path) {
                Ok(_) => {}
                Err(err) => warn!("Failed to remove temporary file: {err}"),
            }

            Err(err)
        }
    }
}

/// Attempts to import the given path as a QMOD
/// The file will be deleted if this results in a success.
///
/// # Arguments
/// * `mod_manager` - A mod manager that has all existing mods loaded already to check for compatibility issues with the new mod.
/// * `from_path` - The path to the mod to import.
///
/// # Returns
/// If successful, an [ImportResultType] of variant `ImportedMod` detailing the ID of the imported mod
/// and the new full list of installed mods.
fn handle_import_qmod(mut mod_manager: ModManager, from_path: PathBuf) -> Result<ImportResultType> {
    debug!("Loading {from_path:?} as a QMOD");
    let id = mod_manager.try_load_new_mod(std::fs::File::open(&from_path)?)?;
    std::fs::remove_file(from_path)?; // Delete temporary file.

    let installed_mods = super::mod_management::get_mod_models(mod_manager)?;

    Ok(ImportResultType::ImportedMod {
        imported_id: id,
        installed_mods,
    })
}

/// Attempts to copy the given file as a mod file copy.
/// If successful, the file will have been deleted, otherwise the file may still exist.
///
/// # Arguments
/// * `from_path` - the path to the file to import via file copy.
/// * `file_ext` - the file extension to use for the file. (which may not match that in `from_path`,
/// e.g. all files imported via URL are saved to the same temporary file name)
/// No period prefix, all lower case.
/// * `mod_manager` - The mod manager with all currently loaded mods, used to check for mod copy extensions that can be used to import the file.
///
/// # Returns
/// If successful, an [ImportResultType] of variant `ImportedSong` to pass back to the frontend.
fn attempt_file_copy(
    from_path: PathBuf,
    file_ext: String,
    mod_manager: ModManager,
) -> Result<ImportResultType> {
    // TODO: Handle case where multiple mods have a copy extension.
    for m in mod_manager.get_mods() {
        let mod_ref = (**m).borrow();
        match mod_ref
            .manifest()
            .copy_extensions
            .iter()
            .filter(|ext| ext.extension.eq_ignore_ascii_case(&file_ext))
            .next()
        {
            Some(copy_ext) => {
                info!("Copying to {}", copy_ext.destination);
                let dest_folder = Path::new(&copy_ext.destination);
                std::fs::create_dir_all(dest_folder)
                    .context("Creating destination folder for file copy")?;
                let dest_path = dest_folder.join(from_path.file_name().unwrap());

                // Rename is not used as these may be in separate volumes.
                std::fs::copy(&from_path, &dest_path).context("Copying mod file copy extension")?;
                std::fs::remove_file(&from_path)?;

                return Ok(ImportResultType::ImportedFileCopy {
                    copied_to: dest_path.to_string_lossy().to_string(),
                    mod_id: mod_ref.manifest().id.to_string(),
                });
            }
            None => {}
        }
    }

    Err(anyhow!(
        "File extension `.{}` was not recognised by any mod",
        file_ext
    ))
}

/// Attempts to import a file as a song.
///
/// This function will check that the file is a valid ZIP file and that it contains a file named `info.dat` or `Info.dat`.
/// It will not do any further verification that the song file is valid.
///
/// If successful, the file is deleted.
///
/// # Arguments
/// * `from_path` - The path to the song file.
///
/// # Returns
/// If successful, an [ImportResultType] of variant `ImportedFileCopy`, detailing the destination path the file was copied to
/// and the mod that specified this destination path.
fn attempt_song_import(from_path: PathBuf) -> Result<ImportResultType> {
    let song_handle = std::fs::File::open(&from_path)?;
    let mut zip = ZipFile::open(song_handle).context("Song was invalid ZIP file")?;

    if zip.contains_file("info.dat") || zip.contains_file("Info.dat") {
        let extract_path = Path::new(&PARAMETERS.custom_levels)
            .join(from_path.file_stem().expect("Must have file stem"));

        if extract_path.exists() {
            std::fs::remove_dir_all(&extract_path).context("Deleting existing song")?;
        }

        std::fs::create_dir_all(&extract_path)?;
        let entry_names = zip
            .iter_entry_names()
            // TODO: This is not nice for performance
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        for entry_name in entry_names {
            zip.extract_file_to(&entry_name, extract_path.join(&entry_name))?;
        }

        drop(zip);
        std::fs::remove_file(from_path)?;
        Ok(ImportResultType::ImportedSong)
    } else {
        Err(anyhow!(
            "ZIP file was not a song; Unclear know how to import it"
        ))
    }
}
