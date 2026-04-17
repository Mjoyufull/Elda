use std::fs::OpenOptions;
use std::fs::{self, File};
use std::path::Path;

use fs4::fs_std::FileExt;

use crate::error::DbError;

#[derive(Debug)]
pub struct MutationLock {
    file: File,
}

impl MutationLock {
    pub fn acquire(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        file.lock_exclusive()?;

        Ok(Self { file })
    }
}

impl Drop for MutationLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
