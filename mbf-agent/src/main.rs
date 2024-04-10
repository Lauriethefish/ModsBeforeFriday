mod requests;
mod zip;
mod manifest;
mod axml;
mod patching;
mod external_res;
mod mod_man;
mod handlers;

use crate::requests::Request;
use anyhow::{Context, Result};
use const_format::formatcp;
use log::{error, Level};
use requests::Response;
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, io::{BufRead, BufReader, Read, Write}, panic, path::Path, process::Command};

// Directories accessed by the agent, in one place so that they can be easily changed.
pub const APK_ID: &str = "com.beatgames.beatsaber";
pub const QMODS_DIR: &str = "/sdcard/ModsBeforeFriday/Mods";
pub const MODLOADER_DIR: &str = formatcp!("/sdcard/ModData/{APK_ID}/Modloader");
pub const LATE_MODS_DIR: &str = formatcp!("{MODLOADER_DIR}/mods");
pub const EARLY_MODS_DIR: &str = formatcp!("{MODLOADER_DIR}/early_mods");
pub const LIBS_DIR: &str = formatcp!("{MODLOADER_DIR}/libs");
pub const APP_DATA_PATH: &str = formatcp!("/sdcard/Android/data/{APK_ID}/files");
pub const PLAYER_DATA_PATH: &str = formatcp!("{APP_DATA_PATH}/PlayerData.dat");
pub const APP_OBB_PATH: &str = formatcp!("/sdcard/Android/obb/{APK_ID}/");

pub const DATAKEEPER_PATH: &str = "/sdcard/ModData/com.beatgames.beatsaber/Mods/datakeeper/PlayerData.dat";
pub const DATA_BACKUP_PATH: &str = "/sdcard/ModsBeforeFriday/PlayerData.backup.dat";

pub const SONGS_PATH: &str = formatcp!("/sdcard/ModData/{APK_ID}/Mods/SongCore/CustomLevels");
pub const DOWNLOADS_PATH: &str = "/data/local/tmp/mbf-downloads";
pub const TEMP_PATH: &str = "/data/local/tmp/mbf-tmp";


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

fn download_file(to: impl AsRef<Path>, url: &str) -> Result<()> {
    let mut resp_body = ureq::get(url)
        .call()
        .context("Failed to request file")?
        .into_reader();

    let mut writer = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(to).context("Failed to create destination file")?;

    std::io::copy(&mut resp_body, &mut writer)?;
    Ok(())
}

fn copy_stream_progress<T: FnMut(usize) -> ()>(from: &mut impl Read,
    to: &mut impl Write,
    progress: &mut T
    ) -> Result<()> {
    let mut buffer = vec![0u8; 4096];

    let mut total_read = 0;
    loop {
        let bytes_read = from.read(&mut buffer)?;
        to.write(&buffer[0..bytes_read])?;

        if bytes_read == 0  {
            break Ok(());
        }   else {
            total_read += bytes_read;
            progress(total_read);
        }
    } 
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ModTag {
    patcher_name: String,
    patcher_version: Option<String>,
    modloader_name: String,
    modloader_version: Option<String>
}

struct ResponseLogger {}

impl log::Log for ResponseLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &log::Record) {
        // Ignore errors, logging should be infallible and we don't want to panic
        let _result = write_response(Response::LogMsg {
            message: format!("{}", record.args()),
            level: match record.level() {
                Level::Debug => requests::LogLevel::Debug,
                Level::Info => requests::LogLevel::Info,
                Level::Warn => requests::LogLevel::Warn,
                Level::Error => requests::LogLevel::Error,
                Level::Trace => requests::LogLevel::Trace
            }
        });
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

fn write_response(response: Response) -> Result<()> {
    let mut lock = std::io::stdout().lock();
    serde_json::to_writer(&mut lock, &response).context("Failed to serialize response")?;
    writeln!(lock)?;
    Ok(())
}

static LOGGER: ResponseLogger = ResponseLogger {};

fn main() -> Result<()> {
    log::set_logger(&LOGGER).expect("Failed to set up logging");
    log::set_max_level(log::LevelFilter::Info);

    let mut reader = BufReader::new(std::io::stdin());
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let req: Request = serde_json::from_str(&line)?;

    // Set a panic hook that writes the panic as a JSON Log
    // (we don't do this in catch_unwind as we get an `Any` there, which doesn't implement Display)
    panic::set_hook(Box::new(|info| error!("Request failed due to a panic!: {info}")));

    match std::panic::catch_unwind(|| handlers::handle_request(req)) {
        Ok(resp) => match resp {
            Ok(resp) => write_response(resp)?,
            Err(err) => error!("{err:?}")
        },
        Err(_) => {} // Panic will be outputted above
    };

    Ok(())
}