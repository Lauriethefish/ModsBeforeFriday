//! Module that allows downloading of files in a reasonably flexible and reliable way
//! Features:
//! - Resuming downloads if they fail partway through
//! - Downloading to a Vec or to a file.
//! - Multiple download attempts.
//! - Progress reporting to the MBF logger

use std::{fs::OpenOptions, io::{self, Cursor, Read, Seek, Write}, path::Path, time::Instant};
use anyhow::{Context, Result, anyhow};
use log::{error, info, warn};

/// Various configuration settings for the file downloader.
pub struct DownloadConfig<'a> {
    /// The number of times the connection can fail before the downloader will give up.
    /// NB: If an unsuccessful status code is returned, the downloader will of course stop the request immediately instead of spamming the server.
    pub max_disconnections: u32,
    /// When the connection is lost, the downloader will wait for this specified period of time before trying again.
    pub disconnect_wait_time: std::time::Duration,
    /// The amount of time between download progress updates. Set to None to disable.
    pub progress_update_interval: Option<std::time::Duration>,
    /// Specifies the ureq agent used to carry out the downloads
    pub ureq_agent: &'a ureq::Agent
}

enum DownloadFileError {
    // An error occured when making the initial request to the server, before the body was read back.
    InitialRequest(ureq::Error),
    // Once the initial request succeeded, but before the whole body had been read, the connection was lost.
    LostConnDuringDownload(io::Error)
}

/// Carries out one attempt to download a file from a URL to the specified stream
/// `file_offset` is the number of bytes to skip from the start of the file when downloading (using the http `Range` header)
/// `progress_update` is called regularly with the number of bytes successfully written to the `to` stream thus far.
/// If the download fails partway, the caller should truncate the `to` stream to the number of successfully written bytes. They can then try to download again.
/// If the response headers indicate that the server supports partial requests, then `out_supports_ranges` will be set to `true`, otherwise it is set `false`.
/// `out_content_length` is where the content length header is written to, if specified.
/// None may be written to `out_content_length` if no content length is given in the response.
fn download_file_to_stream<T: /* params are downloaded length, total length for this range if supplied */ FnMut(usize, Option<usize>) -> ()>(
    cfg: &DownloadConfig,
    file_offset: usize,
    url: &str,
    out_supports_ranges: &mut bool,
    out_filename: &mut Option<String>,
    mut progress_update: T,
    to: impl Write) -> Result<(), DownloadFileError> {
    let mut req = cfg.ureq_agent.get(url)
        // We can't properly process gzipped or any other compressed data when using ranges to carry out a partial download.
        .set("Accept-Encoding", "identity");
    if file_offset != 0 { // No need for range header if downloading the whole file.
        // Specify that we only want the portion of the file from the specified offset
        req = req.set("Range", &format!("bytes={file_offset}-"));
    }

    let resp = req.call().map_err(|err| DownloadFileError::InitialRequest(err))?;

    *out_supports_ranges = resp.header("Accept-Ranges") == Some("bytes");
    *out_filename = get_filename_from_headers(&resp);

    let content_length: Option<usize> = match resp.header("Content-Length") {
        Some(length_str) => length_str.parse().ok(),
        None => None
    };

    // Successfully got the response, now turn it into a reader and begin to copy it to the output
    let mut reader = resp.into_reader();
    
    if let None = content_length {
        warn!("No Content-Length header provided, so MBF cannot update you on the download progress");
    }

    // Copy as many bytes as we can, regularly updating the caller on how many bytes have downloaded successfully.
    copy_stream_progress(&mut reader, to, |bytes_written| { progress_update(bytes_written, content_length)})
        .map_err(|err| DownloadFileError::LostConnDuringDownload(err))?;
    
    Ok(())
}

/// Copies bytes from the `from` stream to the `to` stream.
/// As each buffer of data is copied, the `progress` function is called to update the caller on the number of bytes that have been copied thus far.
fn copy_stream_progress<T: FnMut(usize) -> ()>(from: &mut impl Read,
    mut to: impl Write,
    mut progress: T
    ) -> Result<(), io::Error> {
    let mut buffer = vec![0u8; 8192];

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

/// Extracts the filename from the Content-Disposition HTTP header
/// Returns None if the Content-Disposition header is missing, does not contain the filename,
/// or has any other formatting issue. 
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

/// Attempts to download a file with support for multiple attempts, continuing failed downloads,
/// and progress reporting.
/// Returns the filename, if it was provided within the response.
pub fn download_with_attempts(cfg: &DownloadConfig, mut to: impl Write + Seek, url: &str) -> Result<Option<String>> {
    let mut failed_attempts = 0;
    let mut bytes_valid: usize = 0; // The number of bytes successfully downloaded thus far.
    let mut file_name: Option<String> = None;
    let mut ranges_supported = false;

    loop {
        if failed_attempts > 0 {
            if ranges_supported {
                info!("Continuing download");
            }   else {
                warn!("Restarting entire download as the server has no support for resuming: Attempt {}", failed_attempts + 1);
            }
        }

        // Skip back to the point in the stream where the last valid byte was written
        to.seek(io::SeekFrom::Start(bytes_valid as u64))?;
        let bytes_valid_before_req = bytes_valid;

        let mut last_progress_update = Instant::now();
        let result = download_file_to_stream(cfg,
            bytes_valid,
            url,
            &mut ranges_supported,
            &mut file_name,
            |bytes_written, total_bytes| {
                bytes_valid = bytes_valid_before_req + bytes_written;

                match (cfg.progress_update_interval, total_bytes) {
                    (Some(interval), Some(length)) => {
                        let now = Instant::now();
                        if now.duration_since(last_progress_update) > interval {
                            last_progress_update = now;
                            info!("Progress: {:.2}%", ((bytes_written + bytes_valid_before_req) as f32 
                                / (bytes_valid_before_req + length) as f32) * 100.0);
                        }
                    },
                    // Cannot do progress updates, we need them to be enabled and we need the content length
                    _ => {}
                }
            },
            &mut to);

        match result {
            Ok(_) => return Ok(file_name), // Full file successfully downloaded
            Err(err) => {
                failed_attempts += 1;
                // True if there is no next attempt
                let dl_failed = failed_attempts > cfg.max_disconnections;

                // No support for ranges so we need to redownload the whole file.
                if !ranges_supported {
                    bytes_valid = 0;
                }

                match err {
                    DownloadFileError::InitialRequest(ureq_err) => match ureq_err {
                        // Do not attempt to download again if the error is not network related
                        ureq::Error::Status(code, _resp) => return Err(anyhow!("Request failed as got status {code} from server.")),
                        ureq::Error::Transport(transport_err) => {
                            if dl_failed {
                                return Err(transport_err).context("Failed to make request after all attempts exhausted");
                            }

                            // Error occured due to internet connection, can make another attempt
                            error!("Failed to make initial request: {transport_err}");
                        }
                    },
                    DownloadFileError::LostConnDuringDownload(io_error) => {
                        if dl_failed {
                            return Err(io_error).context("Lost connection mid download and ran out of download attempts");
                        }
                        error!("Failed to complete file download: {io_error}");
                    }
                };

                // Wait a little bit in the hope that the connection loss is temporary
                info!("Waiting briefly for the connection to (hopefully) come back");
                std::thread::sleep(cfg.disconnect_wait_time);
            }
        }
    }
}

/// Attempts to download a file from `url` to the file at `to` with multiple attempts, progress
/// reporting, and resuming of failed downloads.
/// If the file already exists, it is overwritten.
/// Upon success, returns the filename supplied by the server, or None if no filename was supplied.
/// If all attempts fail, the error from the last request is returned.
pub fn download_file_with_attempts(cfg: &DownloadConfig, to: impl AsRef<Path>, url: &str) -> Result<Option<String>> {
    let writer = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(to).context("Failed to create destination file")?;

    download_with_attempts(cfg, writer, url)
}

/// Attempts to download a file from `url` to a Vec with multiple attempts, progress
/// reporting, and resuming of failed downloads.
/// Upon success, returns the filename supplied by the server, or None if no filename was supplied.
/// If all attempts fail, the error from the last request is returned.
pub fn download_to_vec_with_attempts(cfg: &DownloadConfig, url: &str) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    download_with_attempts(cfg, Cursor::new(&mut output), url)?;

    Ok(output)
}