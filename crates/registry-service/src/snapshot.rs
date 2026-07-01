use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::registry::{NodeRecord, Workload};

/// Monotonically increasing logical sequence number.
pub type LogicalSequenceNumber = u64;

/// Immutable snapshot metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotMetadata {
    pub lsn: LogicalSequenceNumber,
    pub created_at_unix_ms: u64,
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
                lsn,
                created_at_unix_ms,
            },
            nodes,
            workloads,
        }
    }
}
