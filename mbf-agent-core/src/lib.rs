pub mod axml;
pub mod data_fix;
pub mod downgrading;
pub mod downloads;
pub mod handlers;
pub mod manifest;
pub mod mod_man;
pub mod models;
pub mod parameters;
pub mod patching;

use anyhow::{Context, Result};
use downloads::DownloadConfig;
use log::{debug, warn};
use mbf_res_man::res_cache::ResCache;
use parameters::PARAMETERS;
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command, sync};

#[cfg(feature = "request_timing")]
use log::info;
#[cfg(feature = "request_timing")]
use std::time::Instant;

/// Attempts to delete legacy directories no longer used by MBF to free up space
/// Logs on failure
pub fn try_delete_legacy_dirs() {
    for dir in &PARAMETERS.legacy_dirs {
        if Path::new(dir).exists() {
            match std::fs::remove_dir_all(dir) {
                Ok(_) => debug!("Successfully removed legacy dir {dir}"),
                Err(err) => warn!("Failed to remove legacy dir {dir}: {err}"),
            }
        }
    }
}

static DOWNLOAD_CFG: sync::OnceLock<DownloadConfig> = sync::OnceLock::new();

/// Gets the default config used for downloads in MBF
pub fn get_dl_cfg() -> &'static DownloadConfig<'static> {
    DOWNLOAD_CFG.get_or_init(|| {
        DownloadConfig {
            max_disconnections: 10,
            // If downloads data successfully for 10 seconds, reset disconnection attempts
            disconnection_reset_time: Some(std::time::Duration::from_secs_f32(10.0)),
            disconnect_wait_time: std::time::Duration::from_secs_f32(5.0),
            progress_update_interval: Some(std::time::Duration::from_secs_f32(2.0)),
            ureq_agent: mbf_res_man::default_agent::get_agent(),
        }
    })
}

/// Creates a ResCache for downloading files using mbf_res_man
/// This should be reused where possible.
pub fn load_res_cache() -> Result<ResCache<'static>> {
    std::fs::create_dir_all(&PARAMETERS.res_cache).expect("Failed to create resource cache folder");
    Ok(ResCache::new(
        (&PARAMETERS.res_cache).into(),
        mbf_res_man::default_agent::get_agent(),
    ))
}

pub fn get_apk_path() -> Result<Option<String>> {
    let pm_output = Command::new("pm")
        .args(["path", &PARAMETERS.apk_id])
        .output()
        .context("Working out APK path")?;
    if 8 > pm_output.stdout.len() {
        // App not installed
        Ok(None)
    } else {
        Ok(Some(
            std::str::from_utf8(pm_output.stdout.split_at(8).1)?
                .trim_end()
                .to_owned(),
        ))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ModTag {
    patcher_name: String,
    patcher_version: Option<String>,
    modloader_name: String,
    modloader_version: Option<String>,
}
