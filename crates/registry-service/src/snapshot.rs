use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::registry::{NodeRecord, Workload};

pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// Monotonically increasing logical sequence number.
pub type LogicalSequenceNumber = u64;

/// Immutable snapshot metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotMetadata {
    pub version: u32,
    pub lsn: LogicalSequenceNumber,
    pub created_at_unix_ms: u64,
    pub checksum: String,
}

/// Complete projected registry state at a given LSN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySnapshot {
    pub metadata: SnapshotMetadata,
    pub nodes: HashMap<Uuid, NodeRecord>,
    pub workloads: HashMap<Uuid, Workload>,
}

impl RegistrySnapshot {
    pub fn new(
        lsn: LogicalSequenceNumber,
        created_at_unix_ms: u64,
        nodes: HashMap<Uuid, NodeRecord>,
        workloads: HashMap<Uuid, Workload>,
    ) -> Self {
        Self {
            metadata: SnapshotMetadata {
                version: SNAPSHOT_FORMAT_VERSION,
                lsn,
                created_at_unix_ms,
                checksum: String::new(),
            },
            nodes,
            workloads,
        }
    }

    pub fn compute_checksum(&self) -> String {
        let mut snapshot = self.clone();
        snapshot.metadata.checksum.clear();

        let bytes = serde_json::to_vec(&snapshot).unwrap_or_default();

        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in bytes {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        format!("{hash:016x}")
    }

    pub fn refresh_checksum(&mut self) {
        self.metadata.checksum = self.compute_checksum();
    }

    pub fn validate_integrity(&self) -> std::io::Result<()> {
        if self.metadata.version != SNAPSHOT_FORMAT_VERSION {
            return Err(std::io::Error::other("unsupported snapshot version"));
        }

        let expected = self.compute_checksum();
        if self.metadata.checksum != expected {
            return Err(std::io::Error::other("snapshot checksum mismatch"));
        }

        Ok(())
    }
}
