use crate::log_replication::LogEntry;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum DurableLogError {
    #[error("I/O failure: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization failure: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct DurableLog {
    path: PathBuf,
}

impl DurableLog {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, DurableLogError> {
        let path = path.into();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        OpenOptions::new().create(true).append(true).open(&path)?;

        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, entry: &LogEntry) -> Result<(), DurableLogError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let encoded = serde_json::to_string(entry)?;
        writeln!(file, "{encoded}")?;
        file.flush()?;

        Ok(())
    }

    pub fn append_many(&self, entries: &[LogEntry]) -> Result<(), DurableLogError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        for entry in entries {
            let encoded = serde_json::to_string(entry)?;
            writeln!(file, "{encoded}")?;
        }

        file.flush()?;
        Ok(())
    }
    pub fn load(&self) -> Result<Vec<LogEntry>, DurableLogError> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;

            if line.trim().is_empty() {
                continue;
            }

            let entry: LogEntry = serde_json::from_str(&line)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    pub fn rewrite(&self, entries: &[LogEntry]) -> Result<(), DurableLogError> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        for entry in entries {
            let encoded = serde_json::to_string(entry)?;
            writeln!(file, "{encoded}")?;
        }

        file.flush()?;
        Ok(())
    }

    pub fn truncate_after(&self, index: u64) -> Result<Vec<LogEntry>, DurableLogError> {
        let retained: Vec<LogEntry> = self
            .load()?
            .into_iter()
            .filter(|entry| entry.index <= index)
            .collect();

        self.rewrite(&retained)?;
        Ok(retained)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_log_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sovereign-os-{name}-{}.jsonl", Uuid::new_v4()))
    }
    #[test]
    fn test_durable_log_append_and_load() {
        let path = temp_log_path("append-load");
        let log = DurableLog::open(&path).unwrap();

        let entry = LogEntry {
            index: 1,
            term: 2,
            command: "schedule:alpha".to_string(),
        };

        log.append(&entry).unwrap();

        let loaded = log.load().unwrap();
        assert_eq!(loaded, vec![entry]);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_durable_log_append_many_preserves_order() {
        let path = temp_log_path("append-many");
        let log = DurableLog::open(&path).unwrap();

        let entries = vec![
            LogEntry {
                index: 1,
                term: 1,
                command: "cmd:a".to_string(),
            },
            LogEntry {
                index: 2,
                term: 1,
                command: "cmd:b".to_string(),
            },
            LogEntry {
                index: 3,
                term: 2,
                command: "cmd:c".to_string(),
            },
        ];

        log.append_many(&entries).unwrap();

        let loaded = log.load().unwrap();
        assert_eq!(loaded, entries);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_durable_log_rewrite_replaces_contents() {
        let path = temp_log_path("rewrite");
        let log = DurableLog::open(&path).unwrap();

        log.append(&LogEntry {
            index: 1,
            term: 1,
            command: "old".to_string(),
        })
        .unwrap();

        let replacement = vec![LogEntry {
            index: 1,
            term: 2,
            command: "new".to_string(),
        }];

        log.rewrite(&replacement).unwrap();

        let loaded = log.load().unwrap();
        assert_eq!(loaded, replacement);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_durable_log_truncate_after_index() {
        let path = temp_log_path("truncate");
        let log = DurableLog::open(&path).unwrap();

        let entries = vec![
            LogEntry {
                index: 1,
                term: 1,
                command: "a".to_string(),
            },
            LogEntry {
                index: 2,
                term: 1,
                command: "b".to_string(),
            },
            LogEntry {
                index: 3,
                term: 2,
                command: "c".to_string(),
            },
        ];

        log.append_many(&entries).unwrap();

        let retained = log.truncate_after(2).unwrap();
        assert_eq!(retained.len(), 2);

        let loaded = log.load().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[1].index, 2);

        let _ = fs::remove_file(path);
    }
}
