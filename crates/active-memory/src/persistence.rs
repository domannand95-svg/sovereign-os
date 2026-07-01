use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::error::ActiveMemoryError;
use crate::models::ActiveEvent;

#[derive(Debug, Clone)]
pub struct StorageEngine {
    path: PathBuf,
}

impl StorageEngine {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, ActiveMemoryError> {
        let path = path.into();

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        OpenOptions::new().create(true).append(true).open(&path)?;

        Ok(Self { path })
    }

    pub fn append_event(&self, event: &ActiveEvent) -> Result<(), ActiveMemoryError> {
        let mut file = OpenOptions::new().append(true).open(&self.path)?;

        let serialized = serde_json::to_string(event)?;
        writeln!(file, "{serialized}")?;

        Ok(())
    }

    pub fn load_events(&self) -> Result<Vec<ActiveEvent>, ActiveMemoryError> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line?;

            if line.trim().is_empty() {
                continue;
            }

            let event: ActiveEvent = serde_json::from_str(&line)?;
            events.push(event);
        }

        Ok(events)
    }
}
