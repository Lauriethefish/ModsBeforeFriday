//! Basic cache of the (ZIP CRC-32) hash of files accessed by mbf-res-man (supports any passed algorithm)
//! Avoids the rehashing of large files such as diffs each time we check the GH release is up to date.

use std::{alloc::System, collections::HashMap, fs::OpenOptions, io::Write, path::{Path, PathBuf}, time::{Instant, SystemTime, UNIX_EPOCH}};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub struct HashCache<H: Copy + Clone> {
    hash_func: fn(std::fs::File) -> Result<H>,
    cache: Option<HashMap<PathBuf, CachedHash<H>>>,
    cache_path: PathBuf,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
struct CachedHash<H> {
    hash: H,
    generation_timestamp: u128,
}

impl<H> HashCache<H> where
    H: serde::Serialize + for<'a> serde::Deserialize<'a> + Copy {
    pub fn new(hash_func: fn(std::fs::File) -> Result<H>, cache_path: PathBuf) -> Self {
        Self {
            hash_func,
            cache: None,
            cache_path,
        }
    }

    fn to_unix_timestamp_millis(time: SystemTime) -> Result<u128> {
        Ok(time.duration_since(UNIX_EPOCH).context("Time set before unix epoch")?
            .as_millis())
    }

    fn ensure_cache_loaded(&mut self) -> Result<()> {
        if self.cache.is_none() {
            if self.cache_path.exists() {
                let cache_string = std::fs::read_to_string(&self.cache_path)?;
    
                self.cache = Some(serde_json::from_str(&cache_string)?)
            }   else {
                self.cache = Some(HashMap::new());
            };
        }

        Ok(())
    }

    pub fn get_file_hash(&mut self, path: &Path) -> Result<H> {
        self.ensure_cache_loaded()?;
        let cache = self.cache.as_mut().expect("Cache loaded with ensure_cache_loaded");

        let file_last_modified = Self::to_unix_timestamp_millis(std::fs::metadata(path)?.modified()?)?;
        match cache.get(path) {
            // Check whether the cache is out of date, if not use the existing hash
            Some(hash) => if hash.generation_timestamp >= file_last_modified {
                return Ok(hash.hash);
            },
            None => {}
        }

        // No existing cached hash, open the file and hash it.
        let file_handle = std::fs::File::open(path)?;

        let hash = (self.hash_func)(file_handle)?;
        cache.insert(path.to_owned(), CachedHash {
            hash,
            generation_timestamp: Self::to_unix_timestamp_millis(SystemTime::now())?
        });
        Ok(hash)
    }

    pub fn save(&self) -> Result<()> {
        match self.cache.as_ref() {
            Some(cache) => {
                let cache_str = serde_json::to_string(&cache)?;

                let mut file_handle = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(&self.cache_path)?;
                file_handle.write_all(cache_str.as_bytes())?;

                Ok(())
            },
            None => Ok(())
        }
    }
}

impl HashCache<u32> {
    pub fn new_crc32(cache_path: PathBuf) -> Self {
        HashCache::new(mbf_zip::crc_of_stream, cache_path)
    }
}

