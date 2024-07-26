mod requests;
mod manifest;
mod axml;
mod patching;
mod mod_man;
mod handlers;
mod data_fix;

use crate::requests::Request;
use anyhow::{Context, Result};
use const_format::formatcp;
use log::{error, info, warn, Level};
use requests::Response;
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, io::{BufRead, BufReader, Cursor, Read, Write}, panic, path::Path, process::Command, time::Instant};

// Directories accessed by the agent, in one place so that they can be easily changed.
pub const APK_ID: &str = "com.beatgames.beatsaber";
// `$` is replaced with the game version
pub const QMODS_DIR: &str = formatcp!("/sdcard/ModData/{APK_ID}/Packages/$");
pub const NOMEDIA_PATH: &str = formatcp!("/sdcard/ModData/{APK_ID}/.nomedia");
pub const MODLOADER_DIR: &str = formatcp!("/sdcard/ModData/{APK_ID}/Modloader");
pub const LATE_MODS_DIR: &str = formatcp!("{MODLOADER_DIR}/mods");
pub const EARLY_MODS_DIR: &str = formatcp!("{MODLOADER_DIR}/early_mods");
pub const LIBS_DIR: &str = formatcp!("{MODLOADER_DIR}/libs");
pub const APP_DATA_PATH: &str = formatcp!("/sdcard/Android/data/{APK_ID}/files");
pub const PLAYER_DATA_PATH: &str = formatcp!("{APP_DATA_PATH}/PlayerData.dat");
pub const PLAYER_DATA_BAK_PATH: &str = formatcp!("{APP_DATA_PATH}/PlayerData.dat.bak");
pub const APP_OBB_PATH: &str = formatcp!("/sdcard/Android/obb/{APK_ID}/");

pub const DATAKEEPER_PATH: &str = "/sdcard/ModData/com.beatgames.beatsaber/Mods/datakeeper/PlayerData.dat";
pub const DATA_BACKUP_PATH: &str = "/sdcard/ModsBeforeFriday/PlayerData.backup.dat";
pub const OLD_QMODS_DIR: &str = "/sdcard/ModsBeforeFriday/Mods";

pub const SONGS_PATH: &str = formatcp!("/sdcard/ModData/{APK_ID}/Mods/SongCore/CustomLevels");
pub const DOWNLOADS_PATH: &str = "/data/local/tmp/mbf-downloads";
pub const TEMP_PATH: &str = "/data/local/tmp/mbf-tmp";

// The number of attempts for all downloads before considering them failed and therefore failing the relevant operation.
pub const DOWNLOAD_ATTEMPTS: u32 = 3;
// The number of seconds between download progress updates.
pub const PROGRESS_UPDATE_INTERVAL: f32 = 2.0;

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

// Attempts to download a file with multiple attempts (3 currently)
// Returns the file name if given in the response, otherwise Ok(None)
fn download_file_with_attempts(to: impl AsRef<Path>, url: &str) -> Result<Option<String>> {
    let mut attempt = 0;
    let to = to.as_ref();
    loop {
        attempt += 1;

        // Recreate the writer with each attempt in order to truncate the file.
        let writer = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(to).context("Failed to create destination file")?;
        match download_file_one_attempt(writer, url) {
            Ok(filename) => return Ok(filename),
            Err(err) => if attempt == DOWNLOAD_ATTEMPTS {
                return Err(err).context("Failed to download file after maximum attempts")
            }   else    {
                warn!("Failed to download file {url}: {err}. Trying again...")
            }
        }
    }
}

// Attempts to download a file to a byte vector with multiple attempts (3 currently)
fn download_to_vec_with_attempts(url: &str) -> Result<Vec<u8>> {
    let mut attempt = 0;
    loop {
        attempt += 1;

        let mut output = Vec::new();
        match download_file_one_attempt(Cursor::new(&mut output), url) {
            Ok(_) => return Ok(output),
            Err(err) => if attempt == DOWNLOAD_ATTEMPTS {
                return Err(err).context("Failed to download file after maximum attempts")
            }   else    {
                warn!("Failed to download file {url}: {err}. Trying again...")
            }
        }
    }
}

fn download_file_one_attempt(mut output: impl Write, url: &str) -> Result<Option<String>> {
    let resp = mbf_res_man::external_res::get_agent().get(url)
        .call()
        .context("Failed to request file")?;

    let content_len = match resp.header("Content-Length") {
        Some(length) => length.parse::<usize>().ok(),
        None => None
    };

    // Extract the file name from the response.
    let filename = get_filename_from_headers(&resp);

    let mut resp_body = resp.into_reader();

    match content_len {
        Some(length) => {
            // Update the frontend with some indication of progress of the download.
            // This will do nothing for small downloads, since they should take less than 5 seconds to complete.
            let mut last_progress_update = Instant::now();
            copy_stream_progress(&mut resp_body, &mut output, &mut |bytes_copied| {
                let now = Instant::now();
                if now.duration_since(last_progress_update).as_secs_f32() > PROGRESS_UPDATE_INTERVAL {
                    last_progress_update = now;
                    info!("Progress: {:.2}%", (bytes_copied as f32 / length as f32) * 100.0);
                }
            })?;
        },
        None => {
            warn!("No Content-Length header, so cannot report download progress");
            std::io::copy(&mut resp_body, &mut output)?;
        }
    }
   
    Ok(filename)
}

fn get_filename_from_headers(resp: &ureq::Response) -> Option<String> {
    match resp.header("Content-Disposition") {
        // Locate the filename within the header
        Some(cont_dis) => match cont_dis.find("filename=") {
            Some(index) => { 
                let with_quotes = cont_dis[(index + 9)..]
                    .split(";") // Remove any subsequent data after the filename
                    .next()
                    .unwrap() // Guaranteed not to panic as there is always at least 1 segment of string
                    .trim();
                // Remove quotes *if there are any* (seems to be inconsistent)
                let start_idx = if with_quotes.chars().next() == Some('"') 
                    { 1 } else { 0 };
                let end_idx = if with_quotes.chars().rev().next() == Some('"') 
                    { with_quotes.len() - 1 } else { with_quotes.len() };

                // Remove the opening and closing quotes
                Some(with_quotes[start_idx..end_idx].to_string())
            },
            None => None
        },
        None => None
    }
}

fn copy_stream_progress<T: FnMut(usize) -> ()>(from: &mut impl Read,
    to: &mut impl Write,
    progress: &mut T
    ) -> Result<()> {
    let mut buffer = vec![0u8; 4096];

    let mut total_read = 0;
    loop {
        let bytes_read = from.read(&mut buffer)?;
        to.write_all(&buffer[0..bytes_read])?;

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