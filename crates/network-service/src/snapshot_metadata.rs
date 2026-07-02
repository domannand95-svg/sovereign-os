use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DurableSnapshotMetadata {
    pub last_confirmed_offset: u64,
}

impl DurableSnapshotMetadata {
    pub fn save(
        &self,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let file = File::create(path)?;

        serde_json::to_writer_pretty(file, self)
            .map_err(|err| io::Error::other(err.to_string()))?;

        Ok(())
    }

    pub fn load(
        path: impl AsRef<Path>,
    ) -> std::io::Result<Self> {
        let file = File::open(path)?;

        serde_json::from_reader(file)
            .map_err(|err| io::Error::other(err.to_string()))
    }
}
