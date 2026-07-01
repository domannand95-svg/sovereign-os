use event_log::EventLog;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistryEvent {
    NodeRegistered { record: NodeRecord },
    StatusUpdated { node_id: Uuid, new_status: OperationalStatus },
    MetricsUpdated { node_id: Uuid, metrics: CapacityMetrics },
    NodeTerminated { node_id: Uuid },
}

pub struct Registry {
    log: EventLog,
    nodes: RefCell<HashMap<Uuid, NodeRecord>>,
}

impl Registry {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        let registry = Self {
            log: EventLog::open(path)?,
            nodes: RefCell::new(HashMap::new()),
        };

        for active_event in registry.log.replay()? {
            let registry_event = RegistryEvent::from_active_event(&active_event)
                .map_err(|err| RegistryError::General(format!("event replay corruption: {err}")))?;

            registry.apply_event(registry_event);
        }

        Ok(registry)
    }

    pub fn register_node(&self, node: NodeRecord) -> Result<(), RegistryError> {
        self.record_event(RegistryEvent::NodeRegistered { record: node })
    }

    pub fn append_registry_event(&self, event: RegistryEvent) -> Result<(), RegistryError> {
        self.record_event(event)
    }

    pub fn history(&self) -> Result<Vec<active_memory::ActiveEvent>, RegistryError> {
        Ok(self.log.replay()?)
    }

    pub fn get_node(&self, node_id: &Uuid) -> Option<NodeRecord> {
        self.nodes.borrow().get(node_id).cloned()
    }

    pub fn list_nodes(&self) -> Vec<NodeRecord> {
        self.nodes.borrow().values().cloned().collect()
    }

    pub fn nodes_by_status(&self, status: OperationalStatus) -> Vec<NodeRecord> {
        self.nodes
            .borrow()
            .values()
            .filter(|node| node.status == status)
            .cloned()
            .collect()
    }

    pub fn nodes_with_capability(&self, capability: &str) -> Vec<NodeRecord> {
        self.nodes
            .borrow()
            .values()
            .filter(|node| node.capabilities.iter().any(|item| item == capability))
            .cloned()
            .collect()
    }

    pub fn active_node_count(&self) -> usize {
        self.nodes.borrow().len()
    }

    fn record_event(&self, event: RegistryEvent) -> Result<(), RegistryError> {
        let active_event = event
            .to_active_event()
            .map_err(|err| RegistryError::General(format!("event serialization failed: {err}")))?;

        self.log.append_active_event(&active_event)?;
        self.apply_event(event);

        Ok(())
    }

    fn apply_event(&self, event: RegistryEvent) {
        let mut nodes = self.nodes.borrow_mut();

        match event {
            RegistryEvent::NodeRegistered { record } => {
                nodes.insert(record.node_id, record);
            }
            RegistryEvent::StatusUpdated { node_id, new_status } => {
                if let Some(node) = nodes.get_mut(&node_id) {
                    node.status = new_status;
                }
            }
            RegistryEvent::MetricsUpdated { node_id, metrics } => {
                if let Some(node) = nodes.get_mut(&node_id) {
                    node.metrics = metrics;
                }
            }
            RegistryEvent::NodeTerminated { node_id } => {
                if let Some(node) = nodes.get_mut(&node_id) {
                    node.status = OperationalStatus::Terminated;
                }
            }
        }
    }
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
