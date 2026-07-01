use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use crate::ActiveMemoryError;

#[derive(Debug, Clone)]
pub struct StorageEngine {
    path: PathBuf,
}

impl StorageEngine {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ActiveMemoryError> {
        let path = path.as_ref().to_path_buf();

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        Ok(Self { path })
    }
}
