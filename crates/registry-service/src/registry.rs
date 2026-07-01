use event_log::EventLog;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

use crate::RegistryError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationalStatus {
    Initializing,
    Active,
    Dormant,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapacityMetrics {
    pub total_compute_cores: u32,
    pub allocated_compute_cores: u32,
    pub total_memory_bytes: u64,
    pub allocated_memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeRecord {
    pub node_id: Uuid,
    pub status: OperationalStatus,
    pub capabilities: Vec<String>,
    pub metrics: CapacityMetrics,
}

pub struct Registry {
    log: EventLog,
}

impl Registry {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        Ok(Self {
            log: EventLog::open(path)?,
        })
    }

    pub fn register_node(&self, node: NodeRecord) -> Result<(), RegistryError> {
        self.log.record_transition(
            "NODE_REGISTERED",
            "registry-service",
            node.node_id.to_string(),
        )?;

        Ok(())
    }

    pub fn history(&self) -> Result<Vec<active_memory::ActiveEvent>, RegistryError> {
        Ok(self.log.replay()?)
    }
}
