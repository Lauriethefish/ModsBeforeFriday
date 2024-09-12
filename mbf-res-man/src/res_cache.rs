use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use log::{debug, warn};
use serde::de::DeserializeOwned;

/// We separate this out into an enum as if a file can't be fetched,
/// then it is useful to know that the *fetching* was the problem and not the *parsing*
/// so that the user can be warned of their failing internet connection.
#[derive(Debug)]
pub enum JsonPullError {
    FetchError(anyhow::Error),
    ParseError(serde_json::Error),
}

impl std::error::Error for JsonPullError {}

impl Display for JsonPullError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(e) => write!(f, "Failed to parse JSON into required type: {e}"),
            Self::FetchError(e) => write!(f, "Failed to download JSON: {e}"),
        }
    }
}

/// The file used to store a cache of the ETags of the other files, if given in the response
/// This filename cannot be used as the name of a cache file.
pub const ETAG_CACHE_FILENAME: &str = "etag_cache.json";

/// A basic cache for the content from HTTP GET requests
pub struct ResCache<'agent> {
    cache_root: PathBuf,
    agent: &'agent ureq::Agent,
    // Map of cache file names to cache ETags.
    // This cache will pass an ETag of the file if one was provided, and will also use If-Modified-Since
    // If this is none, then the ETag cache is yet to be loaded.
    etag_cache: RefCell<Option<HashMap<String, String>>>,
    etag_cache_path: PathBuf,
}

impl<'agent> ResCache<'agent> {
    /// Creates a new resource file hash.
    /// `cache_root` must be a writable directory and it must already exist.
    /// `agent` is used for HTTP request pooling.
    pub fn new(cache_root: PathBuf, agent: &'agent ureq::Agent) -> Self {
        Self {
            etag_cache_path: cache_root.join(ETAG_CACHE_FILENAME),
            cache_root,
            agent,
            etag_cache: RefCell::new(None),
        }
    }

    fn load_etag_cache(&self) -> Result<()> {
        let mut etag_ref = self.etag_cache.borrow_mut();
        if etag_ref.is_none() {
            if self.etag_cache_path.exists() {
                // Attempt to load the existing ETag cache
                let etag_cache_string = std::fs::read_to_string(&self.etag_cache_path)?;

                let loaded_cache = match serde_json::from_str(&etag_cache_string) {
                    Ok(cache) => cache,
                    Err(err) => {
                        warn!("ETag cache was invalid JSON ({err}), creating blank cache");
                        HashMap::new()
                    }
                };

                *etag_ref = Some(loaded_cache);
            } else {
                *etag_ref = Some(HashMap::new());
            }
        }

        Ok(())
    }

    // Must only be called if the ETag cache has been loaded.
    fn save_etag_cache(&self) -> Result<()> {
        let etag_cache_str = serde_json::to_string(
            &self
                .etag_cache
                .borrow()
                .as_ref()
                .expect("ETag cache should have been loaded by now"),
        )?;

        let mut cache_handle = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.etag_cache_path)?;

        cache_handle
            .write_all(etag_cache_str.as_bytes())
            .context("Writing ETag cache")?;
        Ok(())
    }

    /// Gets the agent used by the resource cache.
    pub fn get_agent(&self) -> &'agent ureq::Agent {
        self.agent
    }

    /// Gets a file from the provided URL and caches it at `cached_file_name` within the `cache_root`,
    /// if there is no cached copy already or the cached copy is out of date.
    ///
    /// If the cached copy is found to be in date, this copy will be returned instead.
    pub fn get_cached(&self, url: &str, cached_file_name: &str) -> Result<File> {
        let mut request = self.agent.get(url);
        self.load_etag_cache()?;

        let cached_path = self.cache_root.join(cached_file_name);
        if cached_path.exists() {
            let cache_last_modified = std::fs::metadata(&cached_path)
                .context("Getting metadata on cached file")?
                .modified()
                .context("Getting cached file last modified")?;

            request = request.set(
                "If-Modified-Since",
                &httpdate::fmt_http_date(cache_last_modified),
            );
        }

        let mut etag_cache_ref = self.etag_cache.borrow_mut();
        let etag_cache = etag_cache_ref.as_mut().unwrap();

        if let Some(cached_etag) = etag_cache.get(cached_file_name) {
            request = request.set("If-None-Match", &cached_etag);
        }

        let resp = request.call().context("HTTP GET to get file to cache")?;
        if resp.status() != 304 {
            // If cached file out of date. (or no cache)
            if let Some(etag) = resp.header("ETag") {
                debug!("Got ETag {etag} for {cached_file_name}");
                etag_cache.insert(cached_file_name.to_owned(), etag.to_owned());
                drop(etag_cache_ref);
                self.save_etag_cache()?;
            }

            debug!("No cache, downloading {url} to {cached_file_name}");
            // Copy response body into cache
            let mut cache_handle = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&cached_path)
                .context("Opening cache file for writing: is the directory writable?")?;

            std::io::copy(&mut resp.into_reader(), &mut cache_handle)
                .context("Copying response to cache")?;
        } else {
            // If using cache, ETag should be the same so no need to check it again.
            debug!("Using cached file {cached_file_name} for {url}");
        }

        Ok(std::fs::File::open(cached_path)?)
    }

    /// Gets a file from the provided URL and caches it at `cached_file_name` within the `cache_root`,
    /// if there is no cached copy already or the cached copy is out of date.
    /// Returns the contents of the file as a byte array.
    pub fn get_bytes_cached(&self, url: &str, cached_file_name: &str) -> Result<Vec<u8>> {
        let mut cache_handle = self.get_cached(url, cached_file_name)?;

        let mut buf = Vec::new();
        cache_handle
            .read_to_end(&mut buf)
            .context("Reading file from cache")?;

        Ok(buf)
    }

    /// Gets a file from the provided URL and caches it at `cached_file_name` within the `cache_root`,
    /// if there is no cached copy already or the cached copy is out of date.
    /// Deserializes the response as UTF8 JSON, as T.
    pub fn get_json_cached<T: DeserializeOwned>(
        &self,
        url: &str,
        cached_file_name: &str,
    ) -> Result<T, JsonPullError> {
        let json_bytes = match self.get_bytes_cached(url, cached_file_name) {
            Ok(bytes) => bytes,
            Err(fetch_err) => return Err(JsonPullError::FetchError(fetch_err)),
        };

        match serde_json::from_slice(&json_bytes) {
            Ok(result) => Ok(result),
            Err(parse_err) => Err(JsonPullError::ParseError(parse_err)),
        }
    }
}
