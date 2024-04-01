mod requests;
mod zip;
mod manifest;
mod axml;
mod patching;
mod bmbf_res;

use std::{io::{BufRead, BufReader, Cursor}, process::Command};

use axml::AxmlReader;
use bmbf_res::CoreModsError;
use manifest::ManifestInfo;
use requests::{AppInfo, Request, Response};
use anyhow::{anyhow, Context, Result};

const APK_ID: &str = "com.beatgames.beatsaber";

fn handle_request(request: Request) -> Result<Response> {
    match request {
        Request::GetModStatus => handle_get_mod_status(),
        Request::Patch => handle_patch()
    }
}

fn handle_get_mod_status() -> Result<Response> {
    

    Ok(Response::ModStatus { 
        app_info: get_app_info()?,
        supported_versions: get_supported_versions()?
    })
}

fn get_supported_versions() -> Result<Option<Vec<String>>> {
     // Fetch the core mods from the resources repo
     let core_mods = match bmbf_res::fetch_core_mods() {
        Ok(mods) => mods,
        Err(CoreModsError::FetchError(_)) => return Ok(None),
        Err(CoreModsError::ParseError(err)) => return Err(err)
     };

    let supported_versions: Vec<String> = core_mods.into_keys().filter(|version| {
        let mut iter = version.split('.');
        let _major = iter.next().unwrap();
        let _minor = iter.next().unwrap();

        _minor.parse::<i64>().expect("Invalid version in core mod index") >= 35
    }).collect();

    
    Ok(Some(supported_versions))
}

fn get_app_info() -> Result<Option<AppInfo>> {
    let apk_path = match get_apk_path().context("Failed to find APK path")? {
        Some(path) => path,
        None => return Ok(None)
    };

    let apk_reader = std::fs::File::open(apk_path)?;
    let mut apk = zip::ZipFile::open(apk_reader).context("Failed to read APK as ZIP")?;

    let is_modded = apk
        .iter_entry_names()
        .any(|entry| entry.contains("modded"));

    let manifest = match apk.read_file("AndroidManifest.xml")? {
        Some(contents) => contents,
        None => return Err(anyhow!("Manifest not found in APK"))
    };
    let mut manifest_reader = Cursor::new(manifest);

    let mut axml_reader = AxmlReader::new(&mut manifest_reader)?;
    let info = ManifestInfo::read(&mut axml_reader)?;

    Ok(Some(AppInfo {
        is_modded,
        version: info.package_version
    }))    
}

fn handle_patch() -> Result<Response> {
    patching::mod_current_apk().context("Failed to patch APK")?;
    patching::install_modloader().context("Failed to save modloader")?;

    Ok(Response::Patched)
}

pub fn get_apk_path() -> Result<Option<String>> {
    let pm_output = Command::new("pm")
        .args(["path", APK_ID])
        .output()
        .context("Failed to get APK path")?;
    if 8 > pm_output.stdout.len() {
        // App not installed
        Ok(None)
    }   else {
        Ok(Some(std::str::from_utf8(pm_output.stdout.split_at(8).1)?
            .trim_end()
            .to_owned()))
    }
}

fn main() -> anyhow::Result<()> {
    let mut reader = BufReader::new(std::io::stdin());
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let req: Request = serde_json::from_str(&line)?;

    let resp = handle_request(req)?;
    serde_json::to_writer(std::io::stdout(), &resp)?;
    Ok(())
}