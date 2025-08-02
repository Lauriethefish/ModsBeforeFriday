use std::{
    ffi::OsStr, fs::{File, OpenOptions}, io::{Cursor, Read, Write}, path::{Path, PathBuf}, process::Command
};

use crate::{
    axml::{self, AxmlWriter}, data_fix::fix_colour_schemes, downgrading, downloads, models::response::{AppInfo, InstallStatus, ModLoader}, parameters::PARAMETERS, ModTag
};
use anyhow::{Context, Result};
use log::{debug, info, warn};
use mbf_res_man::{
    external_res,
    res_cache::ResCache,
};
use mbf_zip::{signing, FileCompression, ZipFile};

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

// Mods the given app and reinstalls it, optionally applying patches to downgrade the game, if `downgrade_to` is not None.
// If `manifest_only` is true, patching will only overwrite the manifest and will not add a modloader.
// If this returns true, reinstalling the game deleted installed DLC, which will need to be redownloaded.
pub fn mod_beat_saber(
    temp_path: &Path,
    app_info: &AppInfo,
    downgrade_to: Option<String>,
    manifest_mod: String,
    manifest_only: bool,
    device_pre_v51: bool,
    vr_splash_path: Option<&str>,
    res_cache: &ResCache,
) -> Result<bool> {
    let libunity_path = if manifest_only {
        None
    } else {
        info!("Downloading unstripped libunity.so (this could take a minute)");
        save_libunity(res_cache, temp_path, downgrade_to.as_ref().unwrap_or(&app_info.version))
            .context("Preparing libunity.so")?
    };

    kill_app().context("Killing Beat Saber")?;

    info!("Copying APK to temporary location");
    let temp_apk_path = temp_path.join("mbf-tmp.apk");
    std::fs::copy(&app_info.path, &temp_apk_path).context("Copying APK to temp")?;

    // Make sure the APK is writable.  Sometimes Android will mark it as read-only and
    // the resulting copy will inherit those permissions.
    ensure_can_write_apk(&temp_apk_path).context("Marking APK as writable")?;

    info!("Saving OBB files");
    let obb_backup = temp_path.join("obbs");
    std::fs::create_dir_all(&obb_backup)?;
    let mut obb_backups =
        save_obbs(Path::new(&PARAMETERS.obb_dir), &obb_backup,
        downgrade_to.is_none()).context("Saving OBB files")?;

    // Beat Saber DLC asset files do not have the .obb suffix.
    // If there are any DLC, then these have been deleted by the patching process so we return true so that the user can later be informed of this.
    // They just need to redownload the relevant DLC - they aren't deleted permanently.
    let contains_dlc = has_file_with_no_extension(&PARAMETERS.obb_dir).context("Checking for DLC")?;

    // Determine a diff sequence and apply it, if we're downgrading
    if let Some(to_version) = downgrade_to.clone() {
        obb_backups = downgrading::get_and_apply_diff_sequence(&app_info.version,
            &to_version,
            temp_path,
            &temp_apk_path,
            obb_backups, res_cache).context("Downgrading game")?;
    }

    patch_and_reinstall(
        libunity_path,
        &temp_apk_path,
        obb_backups,
        manifest_mod,
        manifest_only,
        device_pre_v51,
        vr_splash_path,
    )
    .context("Patching and reinstalling APK")?;

    Ok(contains_dlc && downgrade_to.is_some()) // We only delete DLC if we're downgrading.
}

fn ensure_can_write_apk(temp_apk_path: &Path) -> Result<()> {
    let mut permissions = std::fs::metadata(&temp_apk_path).context("Reading temp APK permissions")?.permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&temp_apk_path, permissions).context("Making temp APK writable")?;
    Ok(())
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
    Command::new("am").args(&["force-stop", &PARAMETERS.apk_id]).output()?;
    Ok(())
}

fn patch_and_reinstall(
    libunity_path: Option<PathBuf>,
    temp_apk_path: &Path,
    obb_paths: Vec<PathBuf>,
    manifest_mod: String,
    manifest_only: bool,
    device_pre_v51: bool,
    vr_splash_path: Option<&str>,
) -> Result<()> {
    info!("Patching APK");
    patch_apk_in_place(
        &temp_apk_path,
        libunity_path,
        manifest_mod,
        manifest_only,
        device_pre_v51,
        vr_splash_path,
    )
    .context("Patching APK")?;

    if Path::new(&PARAMETERS.player_data).exists() {
        info!("Backing up player data");
        backup_player_data().context("Backing up player data")?;
    } else {
        info!("No player data to backup");
    }

    if Path::new(&PARAMETERS.datakeeper_player_data).exists() {
        info!("Fixing colour schemes in backed up PlayerData.dat");
        match fix_colour_schemes(&PARAMETERS.datakeeper_player_data) {
            Ok(_) => {}
            Err(err) => warn!("Failed to fix colour schemes: {err}"),
        }
    }

    reinstall_modded_app(&temp_apk_path, device_pre_v51).context("Reinstalling modded APK")?;
    std::fs::remove_file(temp_apk_path)?;

    info!("Restoring OBB files");
    restore_obb_files(Path::new(&PARAMETERS.obb_dir), obb_paths).context("Restoring OBB files")?;

    // Player data is not restored back to the `files` directory as we cannot correctly set its permissions so that BS can access it.
    // (which causes a black screen that can only be fixed by manually deleting the file)

    Ok(())
}

pub fn backup_player_data() -> Result<()> {
    info!("Copying to {}", &PARAMETERS.aux_data_backup);

    std::fs::create_dir_all(Path::new(&PARAMETERS.aux_data_backup).parent().unwrap())?;
    std::fs::copy(&PARAMETERS.player_data, &PARAMETERS.aux_data_backup)?;

    if Path::new(&PARAMETERS.datakeeper_player_data).exists() {
        warn!("Did not backup PlayerData.dat to datakeeper folder as there was already a PlayerData.dat there.
            The player data is still safe in {}", &PARAMETERS.aux_data_backup);
    } else {
        info!("Copying to {}", &PARAMETERS.datakeeper_player_data);
        std::fs::create_dir_all(Path::new(&PARAMETERS.datakeeper_player_data).parent().unwrap())?;
        std::fs::copy(&PARAMETERS.player_data, &PARAMETERS.datakeeper_player_data)?;
    }

    Ok(())
}

fn reinstall_modded_app(
    temp_apk_path: &Path,
    device_pre_v51: bool,
) -> Result<()> {
    info!("Reinstalling modded app");
    Command::new("pm")
        .args(["uninstall", &PARAMETERS.apk_id])
        .output()
        .context("Uninstalling vanilla APK")?;

    Command::new("pm")
        .args(["install", &temp_apk_path.to_string_lossy()])
        .output()
        .context("Installing modded APK")?;

    info!("Granting external storage permission");
    Command::new("appops")
        .args(["set", "--uid", &PARAMETERS.apk_id, "MANAGE_EXTERNAL_STORAGE", "allow"])
        .output()?;

    // Quest 1 specific permissions
    if device_pre_v51 {
        info!("Granting WRITE_EXTERNAL_STORAGE and READ_EXTERNAL_STORAGE (Quest 1)");
        Command::new("pm")
            .args(["grant", &PARAMETERS.apk_id, "android.permission.WRITE_EXTERNAL_STORAGE"])
            .output()?;
        Command::new("pm")
            .args(["grant", &PARAMETERS.apk_id, "android.permission.READ_EXTERNAL_STORAGE"])
            .output()?;    
    }

    Ok(())
}

fn save_libunity(
    res_cache: &ResCache,
    temp_path: impl AsRef<Path>,
    version: &str,
) -> Result<Option<PathBuf>> {
    let url = match external_res::get_libunity_url(res_cache, &PARAMETERS.apk_id, version)? {
        Some(url) => url,
        None => return Ok(None), // No libunity for this version
    };

    let libunity_path = temp_path.as_ref().join("libunity.so");
    downloads::download_file_with_attempts(&crate::get_dl_cfg(), &libunity_path, &url)
        .context("Downloading unstripped libunity.so")?;

    Ok(Some(libunity_path))
}

// Moves the OBB file to a backup location and returns the path that the OBB needs to be restored to
fn save_obbs(obb_dir: &Path, obb_backups_path: &Path, include_dlc: bool) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for err_or_stat in std::fs::read_dir(obb_dir)? {
        if let Ok(stat) = err_or_stat {
            let path = stat.path();
            if (include_dlc || path.extension() == Some(OsStr::new("obb"))) && path.is_file() {
                debug!("Saving OBB path {:?}", path);
                // Make a backup copy of the obb to restore later after patching.
                let obb_backup_path = obb_backups_path.join(path.file_name().unwrap());
                std::fs::copy(&path, &obb_backup_path)?;

                paths.push(obb_backup_path);
            }
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
    let modloaders_path = format!("{}/", &PARAMETERS.modloader_dir);

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
    device_pre_v51: bool,
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

        if device_pre_v51 {
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
    if !Path::new(&PARAMETERS.obb_dir).exists() {
        return Ok(false);
    }

    // Check if any of the files in the OBB directory have extension OBB
    Ok(std::fs::read_dir(&PARAMETERS.obb_dir)?.any(|stat_res| {
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
