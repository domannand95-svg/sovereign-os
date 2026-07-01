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


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistryEvent {
    /// Commits a newly materialized NodeRecord to the topology.
    NodeRegistered {
        record: NodeRecord,
    },
    /// Mutates the active lifecycle state of a target node.
    StatusUpdated {
        node_id: Uuid,
        new_status: OperationalStatus,
    },
    /// Updates resource allocation ceilings.
    MetricsUpdated {
        node_id: Uuid,
        metrics: CapacityMetrics,
    },
    /// Deregisters or halts an unmapped node boundary.
    NodeTerminated {
        node_id: Uuid,
    },
}

impl RegistryEvent {
    pub fn to_active_event(&self) -> Result<active_memory::ActiveEvent, serde_json::Error> {
        let action = match self {
            RegistryEvent::NodeRegistered { .. } => "NODE_REGISTERED",
            RegistryEvent::StatusUpdated { .. } => "STATUS_UPDATED",
            RegistryEvent::MetricsUpdated { .. } => "METRICS_UPDATED",
            RegistryEvent::NodeTerminated { .. } => "NODE_TERMINATED",
        };

        let payload = serde_json::to_value(self)?;
        Ok(active_memory::ActiveEvent::new(action, payload))
    }

    pub fn from_active_event(
        active_event: &active_memory::ActiveEvent,
    ) -> Result<Self, serde_json::Error> {
        serde_json::from_value(active_event.payload.clone())
    }
}
