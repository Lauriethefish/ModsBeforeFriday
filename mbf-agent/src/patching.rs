use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    axml::{self, AxmlWriter},
    data_fix::fix_colour_schemes,
    downloads,
    models::response::{AppInfo, InstallStatus, ModLoader},
    paths, ModTag, APK_ID,
};
use anyhow::{anyhow, Context, Result};
use log::{info, warn};
use mbf_res_man::{
    external_res,
    models::{Diff, VersionDiffs},
    res_cache::ResCache,
};
use mbf_zip::{signing, FileCompression, ZipFile, ZIP_CRC};

const DEBUG_CERT_PEM: &[u8] = include_bytes!("debug_cert.pem");
const LIB_MAIN: &[u8] = include_bytes!("../libs/libmain.so");
const MODLOADER: &[u8] = include_bytes!("../libs/libsl2.so");
const LEGACY_OVRPLATFORMLOADER: &[u8] = include_bytes!("../libs/libovrplatformloader.so");

const MODLOADER_NAME: &str = "libsl2.so";
const MOD_TAG_PATH: &str = "modded.json";

const LIB_MAIN_PATH: &str = "lib/arm64-v8a/libmain.so";
const LIB_UNITY_PATH: &str = "lib/arm64-v8a/libunity.so";
const LIB_OVR_PATH: &str = "lib/arm64-v8a/libovrplatformloader.so";

// Aligment to use for ZIP entries with the STORE compression method, in bytes.
// 4 is the standard value.
const STORE_ALIGNMENT: u16 = 4;

// Mods the currently installed version of the given app and reinstalls it, without doing any downgrading.
// If `manifest_only` is true, patching will only overwrite the manifest and will not add a modloader.
pub fn mod_current_apk(
    temp_path: &Path,
    app_info: &AppInfo,
    manifest_mod: String,
    manifest_only: bool,
    replace_ovr: bool,
    vr_splash_path: Option<&str>,
    res_cache: &ResCache,
) -> Result<()> {
    let libunity_path = if manifest_only {
        None
    } else {
        info!("Downloading unstripped libunity.so (this could take a minute)");
        save_libunity(res_cache, temp_path, &app_info.version).context("Preparing libunity.so")?
    };

    kill_app().context("Killing Beat Saber")?;

    info!("Copying APK to temporary location");
    let temp_apk_path = temp_path.join("mbf-tmp.apk");
    std::fs::copy(&app_info.path, &temp_apk_path).context("Copying APK to temp")?;

    info!("Saving OBB files");
    let obb_backup = temp_path.join("obbs");
    std::fs::create_dir_all(&obb_backup)?;
    let obb_backups =
        save_obbs(Path::new(paths::OBB_DIR), &obb_backup).context("Saving OBB files")?;

    patch_and_reinstall(
        libunity_path,
        &temp_apk_path,
        obb_backups,
        manifest_mod,
        manifest_only,
        replace_ovr,
        vr_splash_path,
    )
    .context("Patching and reinstalling APK")?;
    Ok(())
}

// Downgrades the APK/OBB files for the given app using the diffs provided, then reinstalls the app.
// Returns true if any DLC were found while modding the APK, false otherwise.
pub fn downgrade_and_mod_apk(
    temp_path: &Path,
    app_info: &AppInfo,
    diffs: VersionDiffs,
    manifest_mod: String,
    replace_ovr: bool,
    vr_splash_path: Option<&str>,
    res_cache: &ResCache,
) -> Result<bool> {
    // Download libunity.so *for the downgraded version*
    info!("Downloading unstripped libunity.so (this could take a minute)");
    let libunity_path =
        save_libunity(res_cache, temp_path, &diffs.to_version).context("Saving libunity.so")?;

    // Download the diff files
    let diffs_path = temp_path.join("diffs");
    std::fs::create_dir_all(&diffs_path).context("Creating diffs directory")?;
    info!("Downloading diffs needed to downgrade Beat Saber (this could take a LONG time, make a cup of tea)");
    download_diffs(&diffs_path, &diffs).context("Downloading diffs")?;

    kill_app().context("Killing Beat Saber")?;

    // Copy the APK to temp, downgrading it in the process.
    info!("Downgrading APK");
    let temp_apk_path = temp_path.join("mbf-downgraded.apk");
    apply_diff(
        Path::new(&app_info.path),
        &temp_apk_path,
        &diffs.apk_diff,
        &diffs_path,
    )
    .context("Applying diff to APK")?;

    // Downgrade the obb files, copying them to a temporary directory in the process.
    let obb_backup_dir = temp_path.join("obbs");
    std::fs::create_dir_all(&obb_backup_dir).context("Creating OBB backup directory")?;
    let mut obb_backup_paths = Vec::new();
    for obb_diff in &diffs.obb_diffs {
        let obb_path = Path::new(paths::OBB_DIR).join(&obb_diff.file_name);
        if !obb_path.exists() {
            return Err(anyhow!(
                "Obb file {} did not exist, is the Beat Saber installation corrupt",
                obb_diff.file_name
            ));
        }

        let obb_backup_path = obb_backup_dir.join(&obb_diff.output_file_name);

        info!("Downgrading obb {}", obb_diff.file_name);
        apply_diff(&obb_path, &obb_backup_path, obb_diff, &diffs_path)
            .context("Applying diff to OBB")?;
        obb_backup_paths.push(obb_backup_path);
    }

    // Beat Saber DLC asset files do not have the .obb suffix.
    // If there are any DLC, then these have been deleted by the patching process so we return true so that the user can later be informed of this.
    let contains_dlc = has_file_with_no_extension(paths::OBB_DIR).context("Checking for DLC")?;

    patch_and_reinstall(
        libunity_path,
        &temp_apk_path,
        obb_backup_paths,
        manifest_mod,
        false,
        replace_ovr,
        vr_splash_path,
    )
    .context("Patching and reinstall APK")?;
    Ok(contains_dlc)
}

// Returns true if the given folder contains any files with no file extension.
fn has_file_with_no_extension(obb_dir: impl AsRef<Path>) -> Result<bool> {
    for err_or_stat in std::fs::read_dir(obb_dir)? {
        if let Ok(stat) = err_or_stat {
            let path = stat.path();

            if let None = path.extension() {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

pub fn kill_app() -> Result<()> {
    info!("Killing Beat Saber");
    Command::new("am").args(&["force-stop", APK_ID]).output()?;
    Ok(())
}

fn patch_and_reinstall(
    libunity_path: Option<PathBuf>,
    temp_apk_path: &Path,
    obb_paths: Vec<PathBuf>,
    manifest_mod: String,
    manifest_only: bool,
    replace_ovr: bool,
    vr_splash_path: Option<&str>,
) -> Result<()> {
    info!("Patching APK");
    patch_apk_in_place(
        &temp_apk_path,
        libunity_path,
        manifest_mod,
        manifest_only,
        replace_ovr,
        vr_splash_path,
    )
    .context("Patching APK")?;

    if Path::new(paths::PLAYER_DATA).exists() {
        info!("Backing up player data");
        backup_player_data().context("Backing up player data")?;
    } else {
        info!("No player data to backup");
    }

    if Path::new(paths::DATAKEEPER_PLAYER_DATA).exists() {
        info!("Fixing colour schemes in backed up PlayerData.dat");
        match fix_colour_schemes(paths::DATAKEEPER_PLAYER_DATA) {
            Ok(_) => {}
            Err(err) => warn!("Failed to fix colour schemes: {err}"),
        }
    }

    reinstall_modded_app(&temp_apk_path).context("Reinstalling modded APK")?;
    std::fs::remove_file(temp_apk_path)?;

    info!("Restoring OBB files");
    restore_obb_files(Path::new(paths::OBB_DIR), obb_paths).context("Restoring OBB files")?;

    // Player data is not restored back to the `files` directory as we cannot correctly set its permissions so that BS can access it.
    // (which causes a black screen that can only be fixed by manually deleting the file)

    Ok(())
}

pub fn backup_player_data() -> Result<()> {
    info!("Copying to {}", paths::AUX_DATA_BACKUP);

    std::fs::create_dir_all(Path::new(paths::AUX_DATA_BACKUP).parent().unwrap())?;
    std::fs::copy(paths::PLAYER_DATA, paths::AUX_DATA_BACKUP)?;

    if Path::new(paths::DATAKEEPER_PLAYER_DATA).exists() {
        warn!("Did not backup PlayerData.dat to datakeeper folder as there was already a PlayerData.dat there. 
            The player data is still safe in {}", paths::AUX_DATA_BACKUP);
    } else {
        info!("Copying to {}", paths::DATAKEEPER_PLAYER_DATA);
        std::fs::create_dir_all(Path::new(paths::DATAKEEPER_PLAYER_DATA).parent().unwrap())?;
        std::fs::copy(paths::PLAYER_DATA, paths::DATAKEEPER_PLAYER_DATA)?;
    }

    Ok(())
}

fn reinstall_modded_app(temp_apk_path: &Path) -> Result<()> {
    info!("Reinstalling modded app");
    Command::new("pm")
        .args(["uninstall", APK_ID])
        .output()
        .context("Uninstalling vanilla APK")?;

    Command::new("pm")
        .args(["install", &temp_apk_path.to_string_lossy()])
        .output()
        .context("Installing modded APK")?;

    info!("Granting external storage permission");
    Command::new("appops")
        .args(["set", "--uid", APK_ID, "MANAGE_EXTERNAL_STORAGE", "allow"])
        .output()?;

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

// Loads the file from from_path into memory, verifies it matches the checksum of the given diff,
// applies the diff and then outputs it to to_path
fn apply_diff(from_path: &Path, to_path: &Path, diff: &Diff, diffs_path: &Path) -> Result<()> {
    let diff_content = read_file_vec(diffs_path.join(&diff.diff_name))
        .context("Diff could not be opened. Was it downloaded")?;

    let patch = qbsdiff::Bspatch::new(&diff_content).context("Diff file was invalid")?;

    let file_content = read_file_vec(from_path).context("Reading diff file from disk")?;

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

fn save_libunity(
    res_cache: &ResCache,
    temp_path: impl AsRef<Path>,
    version: &str,
) -> Result<Option<PathBuf>> {
    let url = match external_res::get_libunity_url(res_cache, APK_ID, version)? {
        Some(url) => url,
        None => return Ok(None), // No libunity for this version
    };

    let libunity_path = temp_path.as_ref().join("libunity.so");
    downloads::download_file_with_attempts(&crate::get_dl_cfg(), &libunity_path, &url)
        .context("Downloading unstripped libunity.so")?;

    Ok(Some(libunity_path))
}

// Moves the OBB file to a backup location and returns the path that the OBB needs to be restored to
fn save_obbs(obb_dir: &Path, obb_backups_path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for err_or_stat in std::fs::read_dir(obb_dir)? {
        if let Ok(stat) = err_or_stat {
            let path = stat.path();

            // Rename doesn't work due to different mount points
            let obb_backup_path = obb_backups_path.join(path.file_name().unwrap());
            std::fs::copy(&path, &obb_backup_path)?;
            std::fs::remove_file(&path)?;

            paths.push(obb_backup_path);
        }
    }

    Ok(paths)
}

// Copies the contents of `obb_backups` back to `restore_dir`, creating it if it doesn't already exist.
fn restore_obb_files(restore_dir: &Path, obb_backups: Vec<PathBuf>) -> Result<()> {
    std::fs::create_dir_all(restore_dir)?;
    for backup_path in obb_backups {
        // Cannot use a `rename` since the mount points are different
        info!("Restoring {:?}", backup_path);
        std::fs::copy(
            &backup_path,
            restore_dir.join(backup_path.file_name().unwrap()),
        )?;
        std::fs::remove_file(backup_path)?;
    }

    Ok(())
}

pub fn get_modloader_path() -> Result<PathBuf> {
    let modloaders_path = format!("/sdcard/ModData/{APK_ID}/Modloader/");

    std::fs::create_dir_all(&modloaders_path)?;
    Ok(PathBuf::from(modloaders_path).join(MODLOADER_NAME))
}

// Copies the modloader to the correct directory on the quest
pub fn install_modloader() -> Result<()> {
    let loader_path = get_modloader_path()?;
    info!("Installing modloader to {loader_path:?}");

    let mut handle = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(loader_path)?;
    handle.write_all(MODLOADER)?;
    Ok(())
}

/// Checks the installed libsl2.so to see if it is present and up to date.
pub fn get_modloader_status() -> Result<InstallStatus> {
    let loader_path = get_modloader_path()?;

    info!("Checking if modloader is up to date");
    if loader_path.exists() {
        // Load the existing modloader into memory
        let mut existing_loader_bytes = Vec::<u8>::new();
        std::fs::File::open(loader_path)
            .context("Opening existing modloader (to read) to check if up to date")?
            .read_to_end(&mut existing_loader_bytes)
            .context("Reading existing modloader")?;

        // Check if it's all up-to-date
        if existing_loader_bytes == MODLOADER {
            Ok(InstallStatus::Ready)
        } else {
            Ok(InstallStatus::NeedUpdate)
        }
    } else {
        Ok(InstallStatus::Missing)
    }
}

fn patch_apk_in_place(
    path: impl AsRef<Path>,
    libunity_path: Option<PathBuf>,
    manifest_mod: String,
    manifest_only: bool,
    replace_ovr: bool,
    vr_splash_path: Option<&str>,
) -> Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .context("Opening temporary APK for writing")?;

    let mut zip = ZipFile::open(file).unwrap();
    zip.set_store_alignment(STORE_ALIGNMENT);

    info!("Applying manifest mods");
    patch_manifest(&mut zip, manifest_mod).context("Patching manifest")?;

    let (priv_key, cert) = signing::load_cert_and_priv_key(DEBUG_CERT_PEM);

    if !manifest_only {
        info!("Adding libmainloader");
        zip.delete_file(LIB_MAIN_PATH);
        zip.write_file(
            LIB_MAIN_PATH,
            &mut Cursor::new(LIB_MAIN),
            FileCompression::Deflate,
        )?;
        add_modded_tag(
            &mut zip,
            ModTag {
                patcher_name: "ModsBeforeFriday".to_string(),
                patcher_version: Some("0.1.0".to_string()), // TODO: Get this from the frontend maybe?
                modloader_name: "Scotland2".to_string(), // TODO: This should really be Libmainloader because SL2 isn't inside the APK
                modloader_version: None, // Temporary, but this field is universally considered to be optional so this should be OK.
            },
        )?;

        info!("Adding unstripped libunity.so (this may take up to a minute)");
        match libunity_path {
            Some(unity_path) => {
                let mut unity_stream =
                    File::open(unity_path).context("Opening unstripped libunity.so")?;
                zip.write_file(LIB_UNITY_PATH, &mut unity_stream, FileCompression::Deflate)?;
            }
            None => warn!("No unstripped unity added to the APK! This might cause issues later"),
        }

        if replace_ovr {
            info!("Replacing ovrplatformloader");
            zip.write_file(
                LIB_OVR_PATH, 
                &mut Cursor::new(LEGACY_OVRPLATFORMLOADER), 
                FileCompression::Deflate
            )?;
        }
    }

    if let Some(splash_path) = vr_splash_path {
        info!("Applying custom splash screen");
        let mut vr_splash_file =
            std::fs::File::open(splash_path).context("Opening vr splash image")?;

        zip.write_file(
            "assets/vr_splash.png",
            &mut vr_splash_file,
            FileCompression::Store,
        )?;
    }

    info!("Signing");
    zip.save_and_sign_v2(&cert, &priv_key)
        .context("Saving/signing APK")?;

    Ok(())
}

fn add_modded_tag(to: &mut ZipFile<File>, tag: ModTag) -> Result<()> {
    let saved_tag = serde_json::to_vec_pretty(&tag)?;
    to.write_file(
        MOD_TAG_PATH,
        &mut Cursor::new(saved_tag),
        FileCompression::Deflate,
    )?;
    Ok(())
}

pub fn get_modloader_installed(apk: &mut ZipFile<File>) -> Result<Option<ModLoader>> {
    if apk.contains_file(MOD_TAG_PATH) {
        let tag_data = apk.read_file(MOD_TAG_PATH).context("Reading mod tag")?;
        let mod_tag: ModTag = match serde_json::from_slice(&tag_data) {
            Ok(tag) => tag,
            Err(err) => {
                warn!("Mod tag was invalid JSON: {err}... Assuming unknown modloader");
                return Ok(Some(ModLoader::Unknown));
            }
        };

        Ok(Some(
            if mod_tag.modloader_name.eq_ignore_ascii_case("QuestLoader") {
                ModLoader::QuestLoader
            } else if mod_tag.modloader_name.eq_ignore_ascii_case("Scotland2") {
                // TODO: It's a bit problematic that "Scotland2" is the standard for the contents of modded.json
                // (Since the actual loader inside the APK is libmainloader, which could load any modloader, not just SL2).
                ModLoader::Scotland2
            } else {
                ModLoader::Unknown
            },
        ))
    } else if apk.iter_entry_names().any(|entry| entry.contains("modded")) {
        Ok(Some(ModLoader::Unknown))
    } else {
        Ok(None)
    }
}

/// Checks that there is at least one file with extension .obb in the
/// `/sdcard/Android/obb/com.beatgames.beatsaber` folder.
///
/// MBF only supports BS versions >1.35.0, which all use OBBs so if the obb is not present
/// the installation is invalid and we need to prompt the user to uninstall it.
pub fn check_obb_present() -> Result<bool> {
    if !Path::new(paths::OBB_DIR).exists() {
        return Ok(false);
    }

    // Check if any of the files in the OBB directory have extension OBB
    Ok(std::fs::read_dir(paths::OBB_DIR)?.any(|stat_res| {
        stat_res.is_ok_and(|path| {
            path.path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("obb"))
        })
    }))
}

fn patch_manifest(zip: &mut ZipFile<File>, additional_properties: String) -> Result<()> {
    let mut xml_reader = xml::EventReader::new(Cursor::new(additional_properties.as_bytes()));

    let mut data_output = Cursor::new(Vec::new());
    let mut axml_writer = AxmlWriter::new(&mut data_output);

    axml::xml_to_axml(&mut axml_writer, &mut xml_reader)
        .context("Converting XML back to (binary) AXML")?;
    axml_writer
        .finish()
        .context("Saving AXML (binary) manifest")?;

    zip.delete_file("AndroidManifest.xml");
    zip.write_file(
        "AndroidManifest.xml",
        &mut data_output,
        FileCompression::Deflate,
    )
    .context("Writing modified manifest")?;

    Ok(())
}
