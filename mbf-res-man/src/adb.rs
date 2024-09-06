//! Utilities for interacting with the Android Debug Bridge in order to pull versions of Beat Saber from the Quest.

use std::{ffi::OsStr, path::Path, process::{Command, Output}};

use anyhow::{Context, Result, anyhow};

#[cfg(windows)]
const ADB_EXE_PATH: &str = "adb.exe";

#[cfg(not(windows))]
const ADB_EXE_PATH: &str = "adb";

fn invoke_adb(args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Result<Output> {
    let output = Command::new(ADB_EXE_PATH)
        .args(args)
        .output().context("Invoking ADB executable")?;

    if output.status.success() {
        Ok(output)
    }   else {
        Err(anyhow!("Invoked ADB and got non-zero exit code. stderr: {}, stdout: {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        ))
    }
}

fn get_trimmed_string(from: Vec<u8>) -> String {
    String::from_utf8_lossy(&from).trim().to_string()
}

// Gets the version name of the app with the given package ID installed on the quest.
// Returns None if the app is not installed.
pub fn get_package_version(package_id: &str) -> Result<Option<String>> {
    let output = invoke_adb(&["shell", "dumpsys", "package", package_id])
        .context("Running dumpsys")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Locate the version within the dumpsys output
    let version_idx = match stdout.find("versionName=") {
        Some(location) => location,
        None => return Ok(None) // Not installed
    };

    // Find the next new line at the end of the version name
    // NB: Add the version_idx due to the sliced string.
    let next_newline = stdout[version_idx..].find('\n').unwrap_or(stdout.len()) + version_idx;

    // Trim to remove carriage return (`\r`)
    Ok(Some(stdout[(version_idx + 12)..next_newline].trim().to_string()))
}

// Downloads the APK for the app with the specified package ID to the specified location
pub fn download_apk(package_id: &str, to: &str) -> Result<()> {
    let path_output = invoke_adb(&["shell", "pm", "path", package_id])
        .context("Getting package location")?;

    let app_path = get_trimmed_string(path_output.stdout)[8..].to_string();

    invoke_adb(&["pull", &app_path, to])?;
    Ok(())
}

// Downloads all of the OBB files for the given package ID to the specified path.
pub fn download_obbs(package_id: &str, to_folder: &Path) -> Result<()> {
    let obb_folder = format!("/sdcard/Android/obb/{package_id}/");
    let ls_output = invoke_adb(&["shell", "ls", &obb_folder]).context("Listing available OBBs")?;

    for obb_name in get_trimmed_string(ls_output.stdout).split('\n') {
        let obb_path = format!("{obb_folder}{obb_name}");

        let obb_save_path = to_folder.join(obb_name).to_string_lossy().to_string();

        invoke_adb(&["pull", &obb_path, &obb_save_path]).context("Downloading OBB")?;
    }
    
    Ok(())
}

// Uninstalls the app with the given package ID.
pub fn uninstall_package(id: &str) -> Result<()> {
    invoke_adb(&["uninstall", id])?;
    Ok(())
}

// Installs the APK with the given path onto the Quest.
pub fn install_apk(path: &str) -> Result<()> {
    invoke_adb(&["install", path]).context("Installing package")?;
    Ok(())
}

// Pushes the file with the given path onto the given path on the Quest.
pub fn push_file(path: &str, to: &str) -> Result<()> {
    invoke_adb(&["push", path, to]).context("Pushing file")?;
    Ok(())
}