use std::{fs::{File, OpenOptions}, io::{Cursor, Seek, Write}, path::{Path, PathBuf}, process::Command};

use anyhow::{Context, Result, anyhow};
use log::{info, warn};
use crate::{axml::{AxmlReader, AxmlWriter}, external_res, requests::AppInfo, zip};
use crate::manifest::{ManifestMod, ResourceIds};
use crate::zip::{signing, FileCompression, ZipFile};

const DEBUG_CERT_PEM: &[u8] = include_bytes!("debug_cert.pem");
const LIB_MAIN: &[u8] = include_bytes!("libmain.so");
const MODLOADER: &[u8] = include_bytes!("libsl2.so");
const MODLOADER_NAME: &str = "libsl2.so";

const LIB_MAIN_PATH: &str = "lib/arm64-v8a/libmain.so";
const LIB_UNITY_PATH: &str = "lib/arm64-v8a/libunity.so";
const APK_ID: &str = "com.beatgames.beatsaber";
const TEMP_PATH: &str = "/data/local/tmp/mbf-tmp";

pub fn mod_current_apk(app_info: &AppInfo) -> Result<()> {
    let temp_path = Path::new(TEMP_PATH);
    std::fs::create_dir_all(TEMP_PATH)?;

    info!("Downloading unstripped libunity.so");
    let libunity_path = save_libunity(temp_path, app_info).context("Failed to save libunity.so")?;

    info!("Copying APK to temporary location");
    let temp_apk_path = temp_path.join("mbf-tmp.apk");
    std::fs::copy(&app_info.path, &temp_apk_path).context("Failed to copy APK to temp")?;

    info!("Patching APK at {:?}", temp_path);
    patch_apk_in_place(&temp_apk_path, libunity_path)?;

    let obb_dir = PathBuf::from(format!("/sdcard/Android/obb/{APK_ID}/"));
    let obb_backup = temp_path.join("backup.obb");

    info!("Saving OBB file");
    let obb_restore_path = save_obb(&obb_dir, &obb_backup)?;

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

    // Cannot use a `rename` since the mount points are different
    info!("Restoring OBB file");
    std::fs::create_dir_all(obb_dir)?;
    std::fs::copy(&obb_backup, &obb_restore_path)?;
    std::fs::remove_file(obb_backup)?;
    std::fs::remove_file(temp_apk_path)?;

    Ok(())
}

fn save_libunity(temp_path: impl AsRef<Path>, app_info: &AppInfo) -> Result<Option<PathBuf>> {
    let mut libunity_stream = match external_res::get_libunity_stream(APK_ID, &app_info.version)? {
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
fn save_obb(obb_dir: &Path, obb_backup_path: &Path) -> Result<PathBuf> {
    for err_or_stat in std::fs::read_dir(obb_dir)? {
        if let Ok(stat) = err_or_stat {
            let path = stat.path();
            let ext = path.extension();
            if ext.is_some_and(|ext| ext == "obb") {
                // Rename doesn't work due to different mount points
                std::fs::copy(&path, obb_backup_path)?;
                std::fs::remove_file(&path)?;
                
                return Ok(path)
            }
        }
    }

    Err(anyhow!("Could not find an OBB to save"))
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

fn patch_apk_in_place(path: impl AsRef<Path>, libunity_path: Option<PathBuf>) -> Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("Failed to open APK");
    
    let mut zip = zip::ZipFile::open(file).unwrap();

    patch_manifest(&mut zip).context("Failed to patch manifest")?;

    let (priv_key, cert) = signing::load_cert_and_priv_key(DEBUG_CERT_PEM);

    zip.delete_file(LIB_MAIN_PATH);
    zip.write_file(LIB_MAIN_PATH, &mut Cursor::new(LIB_MAIN), FileCompression::Deflate)?;
    zip.write_file("ModsBeforeFriday.modded", &mut Cursor::new([]), FileCompression::Store)?;
    match libunity_path {
        Some(unity_path) => {
            let mut unity_stream = File::open(unity_path)?;
            zip.write_file(LIB_UNITY_PATH, &mut unity_stream, FileCompression::Deflate)?;
        },
        None => warn!("No unstripped unity added to the APK! This might cause issues later")
    }


    zip.save_and_sign_v2(&cert, &priv_key).context("Failed to save APK")?;

    Ok(())
}

fn patch_manifest(zip: &mut ZipFile<File>) -> Result<()> {
    let contents = zip.read_file("AndroidManifest.xml").context("APK had no manifest")?;
    let mut cursor = Cursor::new(contents);
    let mut reader = AxmlReader::new(&mut cursor).context("Failed to read AXML manifest")?;
    let mut data_output = Cursor::new(Vec::new());
    let mut writer = AxmlWriter::new(&mut data_output);

    let manifest = ManifestMod::new()
        .debuggable(true)
        .with_permission("android.permission.MANAGE_EXTERNAL_STORAGE");

    let res_ids = ResourceIds::load()?;
    
    
    manifest.apply_mod(&mut reader, &mut writer, &res_ids).context("Failed to apply mod")?;

    writer.finish().context("Failed to save AXML manifest")?;

    
    cursor.seek(std::io::SeekFrom::Start(0))?;

    zip.delete_file("AndroidManifest.xml");
    zip.write_file(
        "AndroidManifest.xml",
        &mut data_output,
        FileCompression::Deflate
    ).context("Failed to write modified manifest")?;

    Ok(())
}
