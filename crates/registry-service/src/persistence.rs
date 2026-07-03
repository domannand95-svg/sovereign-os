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
        Ok(Vec::new())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
