use std::{fs::{File, OpenOptions}, io::{BufReader, Cursor, Read, Seek, Write}, path::{Path, PathBuf}, process::Command, time::Instant};

use anyhow::{Context, Result, anyhow};
use log::{error, info, warn};
use crate::{axml::{AxmlReader, AxmlWriter}, copy_stream_progress, external_res::{self, Diff, VersionDiffs}, requests::{AppInfo, ModLoader}, zip::{self, ZIP_CRC}, ModTag, APK_ID, APP_OBB_PATH, DATAKEEPER_PATH, DATA_BACKUP_PATH, PLAYER_DATA_PATH};
use crate::manifest::{ManifestMod, ResourceIds};
use crate::zip::{signing, FileCompression, ZipFile};

const DEBUG_CERT_PEM: &[u8] = include_bytes!("debug_cert.pem");
const LIB_MAIN: &[u8] = include_bytes!("../libs/libmain.so");
const MODLOADER: &[u8] = include_bytes!("../libs/libsl2.so");
const MODLOADER_NAME: &str = "libsl2.so";
const MOD_TAG_PATH: &str = "modded.json";

const LIB_MAIN_PATH: &str = "lib/arm64-v8a/libmain.so";
const LIB_UNITY_PATH: &str = "lib/arm64-v8a/libunity.so";
const DIFF_DOWNLOAD_ATTEMPTS: u32 = 3;

// Mods the currently installed version of the given app and reinstalls it, without doing any downgrading.
// If `manifest_only` is true, patching will only attempt to update permissions/features 
pub fn mod_current_apk(temp_path: &Path, app_info: &AppInfo, manifest_mod: ManifestMod, manifest_only: bool) -> Result<()> {
    let libunity_path = if manifest_only {
        None
    }   else    {
        info!("Downloading unstripped libunity.so (this could take a minute)");
        save_libunity(temp_path, &app_info.version).context("Failed to save libunity.so")?
    };

    kill_app()?;

    info!("Copying APK to temporary location");
    let temp_apk_path = temp_path.join("mbf-tmp.apk");
    std::fs::copy(&app_info.path, &temp_apk_path).context("Failed to copy APK to temp")?;

    info!("Saving OBB files");
    let obb_backup = temp_path.join("obbs");
    std::fs::create_dir(&obb_backup)?;
    let obb_backups = save_obbs(Path::new(APP_OBB_PATH), &obb_backup)?;

    patch_and_reinstall(libunity_path, &temp_apk_path, temp_path, obb_backups, manifest_mod, manifest_only)?;
    Ok(())
}

// Downgrades the APK/OBB files for the given app using the diffs provided, then reinstalls the app.
pub fn downgrade_and_mod_apk(temp_path: &Path,
    app_info: &AppInfo,
    diffs: VersionDiffs,
    manifest_mod: ManifestMod) -> Result<()> {
    // Download libunity.so *for the downgraded version*
    info!("Downloading unstripped libunity.so (this could take a minute)");
    let libunity_path = save_libunity(temp_path, &diffs.to_version)
        .context("Failed to save libunity.so")?;

    // Download the diff files
    let diffs_path = temp_path.join("diffs");
    std::fs::create_dir(&diffs_path)?;
    info!("Downloading diffs needed to downgrade Beat Saber (this could take a LONG time, make a cup of tea)");
    download_diffs(&diffs_path, &diffs)?;

    kill_app()?;

    // Copy the APK to temp, downgrading it in the process.
    info!("Downgrading APK");
    let temp_apk_path = temp_path.join("mbf-downgraded.apk");
    apply_diff(Path::new(&app_info.path), &temp_apk_path, &diffs.apk_diff, &diffs_path)?;

    // Downgrade the obb files, copying them to a temporary directory in the process.
    let obb_backup_dir = temp_path.join("obbs");
    std::fs::create_dir(&obb_backup_dir)?;
    let mut obb_backup_paths = Vec::new();
    for obb_diff in &diffs.obb_diffs {
        let obb_path = Path::new(APP_OBB_PATH).join(&obb_diff.file_name);
        if !obb_path.exists() {
            return Err(anyhow!("Obb file {} did not exist, is the Beat Saber installation corrupt", obb_diff.file_name));
        }

        let obb_backup_path = obb_backup_dir.join(&obb_diff.output_file_name);

        info!("Downgrading obb {}", obb_diff.file_name);
        apply_diff(&obb_path,&obb_backup_path, obb_diff, &diffs_path)?;
        obb_backup_paths.push(obb_backup_path);
    }

    patch_and_reinstall(libunity_path, &temp_apk_path, temp_path, obb_backup_paths, manifest_mod, false)?;
    Ok(())
}

pub fn kill_app() -> Result<()> {
    info!("Killing Beat Saber");
    Command::new("am")
        .args(&["force-stop", APK_ID])
        .output()?;
    Ok(())
}

fn patch_and_reinstall(libunity_path: Option<PathBuf>,
    temp_apk_path: &Path,
    temp_path: &Path,
    obb_paths: Vec<PathBuf>,
    manifest_mod: ManifestMod,
    manifest_only: bool) -> Result<()> {
    info!("Patching APK at {:?}", temp_path);
    patch_apk_in_place(&temp_apk_path, libunity_path, manifest_mod, manifest_only)?;

    if Path::new(PLAYER_DATA_PATH).exists() {
        info!("Backing up player data");
        backup_player_data().context("Failed to backup player data")?;
    }   else    {
        info!("No player data to backup");
    }

    reinstall_modded_app(&temp_apk_path)?;
    std::fs::remove_file(temp_apk_path)?;

    info!("Restoring OBB files");
    restore_obb_files(Path::new(APP_OBB_PATH), obb_paths)?;

    // Player data is not restored back to the `files` directory as we cannot correctly set its permissions so that BS can access it.
    // (which causes a black screen that can only be fixed by manually deleting the file)

    Ok(())
}

pub fn backup_player_data() -> Result<()> {
    info!("Copying to {}", DATA_BACKUP_PATH);

    std::fs::create_dir_all(Path::new(DATA_BACKUP_PATH).parent().unwrap())?;
    std::fs::copy(PLAYER_DATA_PATH, DATA_BACKUP_PATH)?;

    if Path::new(DATAKEEPER_PATH).exists() {
        warn!("Did not backup PlayerData.dat to datakeeper folder as there was already a PlayerData.dat there. 
            The player data is still safe in {}", DATA_BACKUP_PATH);
    }   else    {
        info!("Copying to {}", DATAKEEPER_PATH);
        std::fs::create_dir_all(Path::new(DATAKEEPER_PATH).parent().unwrap())?;
        std::fs::copy(PLAYER_DATA_PATH, DATAKEEPER_PATH)?;
    }

    Ok(())
}

fn reinstall_modded_app(temp_apk_path: &Path) -> Result<()> {
    info!("Reinstalling modded app");
    Command::new("pm")
        .args(["uninstall", APK_ID])
        .output()
        .context("Failed to uninstall vanilla APK")?;

    Command::new("pm")
        .args(["install", &temp_apk_path.to_string_lossy()])
        .output()
        .context("Failed to install modded APK")?;

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
fn apply_diff(from_path: &Path,
    to_path: &Path,
    diff: &Diff,
    diffs_path: &Path) -> Result<()> {
    let diff_content = read_file_vec(diffs_path.join(&diff.diff_name))
        .context("Diff could not be opened. Was it downloaded")?;

    let patch = qbsdiff::Bspatch::new(&diff_content)
        .context("Diff file was invalid")?;

    let file_content = read_file_vec(from_path)?;

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
    let mut output_handle = OpenOptions::new()
        .truncate(true)
        .create(true)
        .read(true)
        .write(true)
        .open(to_path)?;
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
    let mut attempt = 1;
    loop {
        match download_diff(diff, &to_dir) {
            Ok(_) => return Ok(()),
            Err(err) => if attempt == DIFF_DOWNLOAD_ATTEMPTS {
                break Err(err);
            }   else    {
                error!("Failed to download {}: {err}\nTrying again...", diff.diff_name);
            }
        }

        attempt += 1;
    }
}

// Downloads a diff to the given directory, using the file name given in the `Diff` struct.
fn download_diff(diff: &Diff, to_dir: impl AsRef<Path>) -> Result<()> {
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(to_dir.as_ref().join(&diff.diff_name))?;

    let (mut resp, length) = external_res::get_diff_reader(diff)?;

    if let Some(length) = length {
        let mut last_progress_update = Instant::now();
        copy_stream_progress(&mut resp, &mut output, &mut |bytes_copied| {
            let now = Instant::now();
            if now.duration_since(last_progress_update).as_secs_f32() > 2.0 {
                last_progress_update = now;
                info!("Progress: {:.2}%", (bytes_copied as f32 / length as f32) * 100.0);
            }
        })?;

    }   else {
        warn!("Diff repository returned no Content-Length, so cannot show download progress");
        std::io::copy(&mut resp, &mut output)?;
    }
    Ok(())
}

fn save_libunity(temp_path: impl AsRef<Path>, version: &str) -> Result<Option<PathBuf>> {
    let mut libunity_stream = match external_res::get_libunity_stream(APK_ID, version)? {
        Some(stream) => stream,
        None => return Ok(None) // No libunity for this version
    };

    let libunity_path = temp_path.as_ref().join("libunity.so");
    let mut libunity_handle = OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(&libunity_path)?;

    std::io::copy(&mut libunity_stream, &mut libunity_handle)?;

    Ok(Some(libunity_path))
}

// Moves the OBB file to a backup location and returns the path that the OBB needs to be restored to
fn save_obbs(obb_dir: &Path, obb_backups_path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for err_or_stat in std::fs::read_dir(obb_dir)? {
        if let Ok(stat) = err_or_stat {
            let path = stat.path();
            let ext = path.extension();
            // Make sure that we check the extension is OBB: We don't backup DLCs (no extension) since this might cause further issues and they can easily be redownloaded.
            if ext.is_some_and(|ext| ext == "obb") {
                // Rename doesn't work due to different mount points
                let obb_backup_path = obb_backups_path.join(path.file_name().unwrap());
                std::fs::copy(&path, &obb_backup_path)?;
                std::fs::remove_file(&path)?;
                
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
        std::fs::copy(&backup_path, restore_dir.join(backup_path.file_name().unwrap()))?;
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
        .open(loader_path)?;
    handle.write_all(MODLOADER)?;
    Ok(())
}

fn patch_apk_in_place(path: impl AsRef<Path>, libunity_path: Option<PathBuf>, manifest_mod: ManifestMod, manifest_only: bool) -> Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("Failed to open APK");
        
    let mut zip = zip::ZipFile::open(file).unwrap();

    info!("Applying manifest mods");
    patch_manifest(&mut zip, manifest_mod).context("Failed to patch manifest")?;

    let (priv_key, cert) = signing::load_cert_and_priv_key(DEBUG_CERT_PEM);

    if !manifest_only {
        info!("Adding libmainloader");
        zip.delete_file(LIB_MAIN_PATH);
        zip.write_file(LIB_MAIN_PATH, &mut Cursor::new(LIB_MAIN), FileCompression::Deflate)?;
        add_modded_tag(&mut zip, ModTag {
            patcher_name: "ModsBeforeFriday".to_string(),
            patcher_version: Some("0.1.0".to_string()), // TODO: Get this from the frontend maybe?
            modloader_name: "Scotland2".to_string(), // TODO: This should really be Libmainloader because SL2 isn't inside the APK
            modloader_version: None // Temporary, but this field is universally considered to be option so this should be OK.
        })?;

        info!("Adding unstripped libunity.so");
        match libunity_path {
            Some(unity_path) => {
                let mut unity_stream = File::open(unity_path)?;
                zip.write_file(LIB_UNITY_PATH, &mut unity_stream, FileCompression::Deflate)?;
            },
            None => warn!("No unstripped unity added to the APK! This might cause issues later")
        }
    }

    info!("Signing");
    zip.save_and_sign_v2(&cert, &priv_key).context("Failed to save APK")?;

    Ok(())
}

fn add_modded_tag(to: &mut ZipFile<File>, tag: ModTag) -> Result<()> {
    let saved_tag = serde_json::to_vec_pretty(&tag)?;
    to.write_file(MOD_TAG_PATH,
        &mut Cursor::new(saved_tag),
        FileCompression::Deflate
    )?;
    Ok(())
}

pub fn get_modloader_installed(apk: &mut ZipFile<File>) -> Result<Option<ModLoader>> {
    if apk.contains_file(MOD_TAG_PATH) {
        let tag_data = apk.read_file(MOD_TAG_PATH).context("Failed to read mod tag")?;
        let mod_tag: ModTag = match serde_json::from_slice(&tag_data) {
            Ok(tag) => tag,
            Err(err) => {
                warn!("Mod tag was invalid JSON: {err}... Assuming unknown modloader");
                return Ok(Some(ModLoader::Unknown))
            }
        };

        Ok(Some(if mod_tag.modloader_name.eq_ignore_ascii_case("QuestLoader") {
            ModLoader::QuestLoader
        }   else if mod_tag.modloader_name.eq_ignore_ascii_case("Scotland2") {
            // TODO: It's a bit problematic that "Scotland2" is the standard for the contents of modded.json
            // (Since the actual loader inside the APK is libmainloader, which could load any modloader, not just SL2).
            ModLoader::Scotland2
        }   else {
            ModLoader::Unknown
        }))
    }   else if apk.iter_entry_names().any(|entry| entry.contains("modded")) {
        Ok(Some(ModLoader::Unknown))
    }   else {
        Ok(None)
    }
}

fn patch_manifest(zip: &mut ZipFile<File>, additional_properties: ManifestMod) -> Result<()> {
    let contents = zip.read_file("AndroidManifest.xml").context("APK had no manifest")?;
    let mut cursor = Cursor::new(contents);
    let mut reader = AxmlReader::new(&mut cursor).context("Failed to read AXML manifest")?;
    let mut data_output = Cursor::new(Vec::new());
    let mut writer = AxmlWriter::new(&mut data_output);

    let manifest = additional_properties
        .debuggable(true)
        .with_permission("android.permission.MANAGE_EXTERNAL_STORAGE");

    let res_ids = ResourceIds::load()?;
    
    
    let modified = manifest.apply_mod(&mut reader, &mut writer, &res_ids).context("Failed to apply mod")?;

    writer.finish().context("Failed to save AXML manifest")?;

    if !modified {
        info!("Manifest unmodified, not saving");
        return Ok(());
    }

    
    cursor.seek(std::io::SeekFrom::Start(0))?;

    zip.delete_file("AndroidManifest.xml");
    zip.write_file(
        "AndroidManifest.xml",
        &mut data_output,
        FileCompression::Deflate
    ).context("Failed to write modified manifest")?;

    Ok(())
}
