//! Manages the lockfile for the Packages directory, 
//! used to ensure that separate installers do not attempt to write to the directory at the same time.

use std::fs::{File, OpenOptions};
use std::path::Path;
use anyhow::Result;

use anyhow::Context;
use fs2::FileExt;
use log::info;

use crate::QMODS_LOCK_PATH;

pub struct ModInstallLock {
    lock_file: File
}

impl Drop for ModInstallLock {
    fn drop(&mut self) {
        let _ = self.lock_file.unlock();
    }
}

impl ModInstallLock {
    /// Creates a shared mod install lock.
    pub fn shared() -> Result<ModInstallLock> {
        Self::lock(false)
    }

    /// Creates an exclusive mod install lock.
    pub fn excl() -> Result<ModInstallLock> {
        Self::lock(true)
    }

    /// Creates a mod install lock, which will be released when the returned struct is dropped.
    pub fn lock(exclusive: bool) -> Result<ModInstallLock> {
        std::fs::create_dir_all(Path::new(QMODS_LOCK_PATH).parent()
            .expect("QMODs lock should never be in the root directory"))
            .context("Failed to create directory containing lockfile")?;

        let lock_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(QMODS_LOCK_PATH).context("Failed to open lockfile")?;

        info!("Waiting for lock on mod installation");
        
        if exclusive {
            lock_file.lock_exclusive().context("Failed to obtain exclusive lock on installed mods")?;
        }   else {
            lock_file.lock_shared().context("Failed to obtain shared lock on installed mods")?;
        }

        Ok(ModInstallLock {
            lock_file
        })
    }
}