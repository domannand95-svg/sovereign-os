use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotMetadata {
    pub last_included_index: u64,
    pub last_included_term: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snapshot {
    pub metadata: SnapshotMetadata,
    pub data: Vec<u8>,
}

pub struct SnapshotStorage;

impl SnapshotStorage {
    pub fn save<P: AsRef<Path>>(path: P, snapshot: &Snapshot) -> io::Result<()> {
        let bytes =
            serde_json::to_vec(snapshot).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let path_ref = path.as_ref();
        let tmp_path = path_ref.with_extension("tmp");

        fs::write(&tmp_path, bytes)?;
        fs::rename(tmp_path, path_ref)
    }
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Snapshot> {
        let bytes = fs::read(path)?;

        let snapshot =
            serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(snapshot)
    }

    pub fn delete<P: AsRef<Path>>(path: P) -> io::Result<()> {
        if path.as_ref().exists() {
            fs::remove_file(path)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn snapshot_round_trip() {
        let path = env::temp_dir().join("raft_snapshot_test.json");

        let snapshot = Snapshot {
            metadata: SnapshotMetadata {
                last_included_index: 42,
                last_included_term: 7,
            },
            data: vec![1, 2, 3, 4],
        };

        SnapshotStorage::save(&path, &snapshot).unwrap();

        let loaded = SnapshotStorage::load(&path).unwrap();

        assert_eq!(snapshot, loaded);

        SnapshotStorage::delete(&path).unwrap();
    }
    #[test]
    fn delete_missing_snapshot_is_ok() {
        let path = env::temp_dir().join("raft_snapshot_missing.json");

        let _ = fs::remove_file(&path);

        SnapshotStorage::delete(&path).unwrap();
    }
}
