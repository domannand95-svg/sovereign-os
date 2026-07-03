use crate::LedgerEntry;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PersistenceError {
    Serialization,
    Storage,
    CorruptedData,
}

pub trait PersistenceEngine {
    type Error;

    fn append(&mut self, entry: &LedgerEntry) -> Result<(), Self::Error>;

    fn load(&self) -> Result<Vec<LedgerEntry>, Self::Error>;

    fn flush(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryPersistence {
    entries: Vec<LedgerEntry>,
}

impl MemoryPersistence {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl PersistenceEngine for MemoryPersistence {
    type Error = PersistenceError;

    fn append(&mut self, entry: &LedgerEntry) -> Result<(), Self::Error> {
        self.entries.push(entry.clone());
        Ok(())
    }

    fn load(&self) -> Result<Vec<LedgerEntry>, Self::Error> {
        Ok(self.entries.clone())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LedgerEvent, LedgerHeader};

    fn entry(index: u64) -> LedgerEntry {
        LedgerEntry {
            header: LedgerHeader {
                index,
                term: 1,
                timestamp: index * 100,
                previous_hash: [0; 32],
                payload_hash: [index as u8; 32],
            },
            event: LedgerEvent::AgentRegistered {
                agent_id: [index as u8; 16],
            },
        }
    }

    #[test]
    fn memory_persistence_starts_empty() {
        let store = MemoryPersistence::new();

        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn memory_persistence_appends_and_loads_entries() {
        let mut store = MemoryPersistence::new();
        let first = entry(1);

        store.append(&first).unwrap();

        let loaded = store.load().unwrap();

        assert_eq!(loaded, vec![first]);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn memory_persistence_flush_is_noop_success() {
        let mut store = MemoryPersistence::new();

        assert_eq!(store.flush(), Ok(()));
    }

    fn get_unique_json_test_path(suffix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sovereign_test_{}_{}.json",
            suffix,
            std::process::id()
        ))
    }

    #[test]
    fn test_json_persistence_missing_file() {
        let temp_path = get_unique_json_test_path("missing");
        let _ = std::fs::remove_file(&temp_path);

        let adapter = JsonFilePersistence::new(&temp_path);
        let result = adapter.load();

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_json_persistence_round_trip() {
        let temp_path = get_unique_json_test_path("round_trip");
        let _ = std::fs::remove_file(&temp_path);

        let mut adapter = JsonFilePersistence::new(&temp_path);
        let test_entry = entry(1);

        adapter.append(&test_entry).unwrap();

        let loaded = adapter.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], test_entry);

        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_json_persistence_fifo_ordering() {
        let temp_path = get_unique_json_test_path("fifo");
        let _ = std::fs::remove_file(&temp_path);

        let mut adapter = JsonFilePersistence::new(&temp_path);
        let entry1 = entry(1);
        let entry2 = entry(2);

        adapter.append(&entry1).unwrap();
        adapter.append(&entry2).unwrap();

        let loaded = adapter.load().unwrap();
        assert_eq!(loaded, vec![entry1, entry2]);

        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_json_persistence_corruption_halt() {
        use std::io::Write;

        let temp_path = get_unique_json_test_path("corruption");
        let _ = std::fs::remove_file(&temp_path);

        let mut adapter = JsonFilePersistence::new(&temp_path);
        adapter.append(&entry(1)).unwrap();

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&temp_path)
            .unwrap();

        writeln!(file, "INVALID NON-JSON NOISE LINE").unwrap();

        let result = adapter.load();
        assert!(matches!(result, Err(PersistenceError::CorruptedData)));

        let _ = std::fs::remove_file(&temp_path);
    }

}


pub struct JsonFilePersistence {
    pub storage_path: std::path::PathBuf,
}

impl JsonFilePersistence {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self {
            storage_path: path.as_ref().to_path_buf(),
        }
    }
}

impl PersistenceEngine for JsonFilePersistence {
    type Error = PersistenceError;

    fn append(&mut self, entry: &LedgerEntry) -> Result<(), Self::Error> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.storage_path)
            .map_err(|_| PersistenceError::Storage)?;

        let json_line = serde_json::to_string(entry)
            .map_err(|_| PersistenceError::Serialization)?;

        writeln!(file, "{}", json_line)
            .map_err(|_| PersistenceError::Storage)?;

        file.flush()
            .map_err(|_| PersistenceError::Storage)?;

        Ok(())
    }

    fn load(&self) -> Result<Vec<LedgerEntry>, Self::Error> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

            if !self.storage_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.storage_path)
            .map_err(|_| PersistenceError::Storage)?;

        if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            return Ok(Vec::new());
        }

        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line_result in reader.lines() {
            let line = line_result.map_err(|_| PersistenceError::Storage)?;
            if line.trim().is_empty() {
                continue;
            }

            let entry: LedgerEntry = serde_json::from_str(&line)
                .map_err(|_| PersistenceError::CorruptedData)?;

            entries.push(entry);
        }

        Ok(entries)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
