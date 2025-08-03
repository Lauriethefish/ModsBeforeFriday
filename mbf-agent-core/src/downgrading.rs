//! Implementation of diff-chaining to allow any version of Beat Saber to be downgraded to any other, assuming there is a diff
//! from each version to the previous version
//! 
//! This module also allows more "direct" diffs to be created for faster downgrading, say several versions at once.
//! A breadth-first-search is used to determine the shortest route (smallest number of diffs) in the database to downgrade one version to another.

use std::{collections::{HashMap, VecDeque}, fs::OpenOptions, io::{BufReader, BufWriter, Read}, path::{Path, PathBuf}};

use log::info;
use mbf_res_man::{models::{Diff, VersionDiffs}, res_cache::ResCache};
use anyhow::{Result, Context, anyhow};
use mbf_res_man::external_res;
use mbf_zip::ZIP_CRC;
use std::ffi::OsStr;

use crate::downloads;

// Gets a map of all accessible Beat Saber versions from the given version
// Key is Beat Saber version, value is the sequence of diffs that need to be applied to reach that version.
// Always returns the sequences with the smallest number of diffs
pub fn get_all_accessible_versions(res_cache: &ResCache, from_version: &str) -> Result<HashMap<String, Vec<VersionDiffs>>> {
    let diff_index_edges = get_diff_index_graph(res_cache)
        .context("Loading diff index")?;

    // Holds the path of diffs used to reach each Beat Saber version
    let mut predecessor_map: HashMap<String, Vec<VersionDiffs>> = HashMap::new();
    predecessor_map.insert(from_version.to_owned(), Vec::new()); // No diffs needed to reach the current version.

    let mut queue = VecDeque::new();
    queue.push_back(from_version.to_string());

    // Apply breath first search
    // TODO: This does tons of copying though it doesn't really matter much since there aren't too many versions
    while let Some(curr_ver) = queue.pop_front() {
        if let Some(edges) = diff_index_edges.get(&curr_ver) { 
            for diff in edges {
                if !predecessor_map.contains_key(&diff.to_version) {
                    let mut path = predecessor_map.get(&curr_ver)
                        .expect("Current version should always have a path").clone();
                    path.push(diff.clone());

                    predecessor_map.insert(diff.to_version.clone(), path);
                    queue.push_back(diff.to_version.clone());
                }
            }
        }
    }

    predecessor_map.remove(from_version); // Don't want to show this version in the list of downgrade versions
    Ok(predecessor_map)
    
}

// Loads the diff index as a graph, where each entry in the HashMap lists the edges from each node (Beat Saber versions are nodes)
fn get_diff_index_graph(res_cache: &ResCache) -> Result<HashMap<String, Vec<VersionDiffs>>> {
    let diff_index = mbf_res_man::external_res::get_diff_index(res_cache)
            .context("Fetching downgrading information")?;
    
    let mut diff_index_edges: HashMap<String, Vec<VersionDiffs>> = HashMap::new();
    for diff in diff_index {
        if let Some(accessible) = diff_index_edges.get_mut(&diff.from_version) {
            accessible.push(diff);
        }   else {
            diff_index_edges.insert(diff.from_version.to_owned(), vec![diff]);
        }
    }

    Ok(diff_index_edges)
}

// Downgrades the APK file at `temp_path` and the OBB files at `obb_backup_paths`.
// Determines the sequence of diffs to apply automatically.
// The destination APK is written to `temp_path` and the destination OBBs are written to the same directory as the current OBBs.
// Returns the paths to the downgraded OBB files for restoring once Beat Saber is reinstalled.
pub fn get_and_apply_diff_sequence(from_version: &str, to_version: &str,
    temp_path: &Path, temp_apk_path: &Path, obb_backup_paths: Vec<PathBuf>,
    res_cache: &ResCache)
    -> Result<Vec<PathBuf>> {
    info!("Working out diff sequence for {from_version} --> {to_version}");
    let diff_sequences = get_all_accessible_versions(res_cache, from_version)
        .context("Determining diff sequence")?;
    let diffs = diff_sequences.get(to_version)
        .ok_or(anyhow!("No diff sequence found for version. Why did the frontend let us select it?!"))?;

    apply_diff_sequence(diffs, temp_path, &temp_apk_path, obb_backup_paths)
        .context("Downgrading")
}

// Downgrades the APK file at `temp_path` and the OBB files at `obb_backup_paths`.
// The destination APK is written to `temp_path` and the destination OBBs are written to the same directory as the current OBBs.
// Returns the paths to the downgraded OBB files for restoring once Beat Saber is reinstalled.
pub fn apply_diff_sequence(diffs: &[VersionDiffs], temp_path: &Path,
    temp_apk_path: &Path,
    mut obb_backup_paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    info!("DOWNGRADING BEAT SABER: This may take a LONG time");
    
    for (i, diff) in diffs.iter().enumerate() {
        info!("Applying diffs set {}/{} ({} --> {})", i + 1, diffs.len(), diff.from_version, diff.to_version);

        obb_backup_paths = apply_version_diff(diff, temp_path, &temp_apk_path, obb_backup_paths)
            .context("Applying diff")?;
    }

    Ok(obb_backup_paths)
}

// Downgrades one version of Beat Saber to another, including the APK file and all OBB files
// Passed the path to the temporary APK and paths to each of the existing OBBs for downgrading.
fn apply_version_diff(diffs: &VersionDiffs, temp_path: &Path,
    temp_apk_path: &Path, obb_backup_paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    // Download the diff files
    let diffs_path = temp_path.join("diffs");
    std::fs::create_dir_all(&diffs_path).context("Creating diffs directory")?;
    info!("Downloading diffs");
    download_diffs(&diffs_path, &diffs).context("Downloading diffs")?;

    // Copy the APK to temp, downgrading it in the process.
    info!("Downgrading APK");
    apply_diff(
        // If there is already a "downgraded" APK, then we have already applied one diff, so we downgrade THIS APK to the next version.
        &temp_apk_path,
        &temp_apk_path,
        &diffs.apk_diff,
        &diffs_path,
    )
    .context("Applying diff to APK")?;

    let mut dest_obb_paths = Vec::new();
    for obb_diff in &diffs.obb_diffs {
        // Find the OBB matching the filename in the diff
        let existing_obb = obb_backup_paths.iter()
            .find(|p| p.file_name() == Some(OsStr::new(&obb_diff.file_name)))
            .ok_or(anyhow!("No obb file {} found - is the diff index wrong", obb_diff.file_name))?;

        // Determine a suitable destination path.
        let obbs_folder = existing_obb.parent().unwrap();
        let dest_obb = obbs_folder.join(&obb_diff.output_file_name);

        apply_diff(&existing_obb, &dest_obb, obb_diff, &diffs_path)
            .context("Applying diff to OBB")?;
        std::fs::remove_file(existing_obb).context("Deleting old OBB")?; // Save storage space!
        dest_obb_paths.push(dest_obb);
    }

    

    // Delete diffs when we're done to avoid using too much storage.
    std::fs::remove_dir_all(diffs_path)?;

    Ok(dest_obb_paths)
}

// Loads the file from from_path into memory, verifies it matches the checksum of the given diff,
// applies the diff and then outputs it to to_path
// `from_path` and `to_path` can be the same if you like. I give you permission.
fn apply_diff(from_path: &Path, to_path: &Path, diff: &Diff, diffs_path: &Path) -> Result<()> {
    let diff_content = read_file_vec(diffs_path.join(&diff.diff_name))
        .context("Diff could not be opened. Was it downloaded")?;

    let patch = qbsdiff::Bspatch::new(&diff_content).context("Diff file was invalid")?;

    let file_content = read_file_vec(from_path).context("Reading original file from disk")?;

    // Verify the CRC32 hash of the file content.
    info!("Verifying installation is unmodified");
    let before_crc = ZIP_CRC.checksum(&file_content);
    if before_crc != diff.file_crc {
        return Err(anyhow!("File CRC {} did not match expected value of {}. 
            Your installation is corrupted, so MBF can't downgrade it. Reinstall Beat Saber to fix this issue!
            Alternatively, if your game is pirated, purchase a legitimate copy of the game.", before_crc, diff.file_crc));
    }

    // Carry out the downgrade
    info!("Applying patch (This step may take a few minutes)");
    let mut output_handle = BufWriter::new(
        OpenOptions::new()
            .truncate(true)
            .create(true)
            .read(true)
            .write(true)
            .open(to_path)?,
    );
    patch.apply(&file_content, &mut output_handle)?;

    // TODO: Verify checksum on the result of downgrading?

    Ok(())
}

// Downloads the deltas needed for downgrading with the given version_diffs.
// The diffs are saved with names matching `diff_name` in the `Diff` struct.
fn download_diffs(to_path: impl AsRef<Path>, version_diffs: &VersionDiffs) -> Result<()> {
    for diff in version_diffs.obb_diffs.iter() {
        info!("Downloading diff for OBB {}", diff.file_name);
        download_diff_retry(diff, &to_path)?;
    }

    info!("Downloading diff for APK");
    download_diff_retry(&version_diffs.apk_diff, to_path)?;

    Ok(())
}

// Attempts to download the given diff DIFF_DOWNLOAD_ATTEMPTS times, returning an error if the final attempt fails.
fn download_diff_retry(diff: &Diff, to_dir: impl AsRef<Path>) -> Result<()> {
    let url = external_res::get_diff_url(diff);
    let output_path = to_dir.as_ref().join(&diff.diff_name);

    downloads::download_file_with_attempts(&crate::get_dl_cfg(), &output_path, &url)
        .context("Downloading diff file")?;
    Ok(())
}

// Reads the content of the given file path as a Vec
fn read_file_vec(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let handle = std::fs::File::open(path)?;

    let mut file_content = Vec::with_capacity(handle.metadata()?.len() as usize);
    let mut reader = BufReader::new(handle);
    reader.read_to_end(&mut file_content)?;

    Ok(file_content)
}