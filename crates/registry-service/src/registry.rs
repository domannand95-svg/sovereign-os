fn snapshot_path<P: AsRef<std::path::Path>>(ledger: P) -> std::path::PathBuf {
    let mut p = ledger.as_ref().to_path_buf();
    p.set_extension("snap");
    p
}

use crate::AllocationRequest;
use event_log::EventLog;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use crate::RegistryError;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum OperationalStatus {
    Initializing,
    Active,
    Dormant,
    Terminated,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CapacityMetrics {
    pub total_compute_cores: u32,
    pub allocated_compute_cores: u32,
    pub total_memory_bytes: u64,
    pub allocated_memory_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct NodeRecord {
    pub node_id: Uuid,
    pub status: OperationalStatus,
    pub capabilities: Vec<String>,
    pub metrics: CapacityMetrics,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum WorkloadState {
    Pending,
    Running { assigned_node_id: Uuid },
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Workload {
    pub workload_id: Uuid,
    pub priority: u32,
    pub requirements: AllocationRequest,
    pub state: WorkloadState,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum RegistryEvent {
    NodeRegistered {
        record: NodeRecord,
    },
    StatusUpdated {
        node_id: Uuid,
        new_status: OperationalStatus,
    },
    MetricsUpdated {
        node_id: Uuid,
        metrics: CapacityMetrics,
    },
    NodeTerminated {
        node_id: Uuid,
    },
    WorkloadScheduled {
        workload: Workload,
        node_id: Uuid,
    },
    WorkloadCompleted {
        workload_id: Uuid,
        node_id: Uuid,
    },
}

pub struct Registry {
    log: EventLog,
    nodes: RefCell<HashMap<Uuid, NodeRecord>>,
    workloads: RefCell<HashMap<Uuid, Workload>>,
}

impl Registry {
    pub fn append_governed_event(&mut self, event: RegistryEvent) -> Result<(), RegistryError> {
        let active_event = event
            .to_active_event()
            .map_err(|e| RegistryError::General(e.to_string()))?;

        self.log.append_active_event(&active_event)?;
        self.apply_event(event);
        self.maybe_snapshot()?;
        Ok(())
    }

    pub fn maybe_snapshot(&self) -> Result<(), RegistryError> {
        let event_count = self
            .log
            .len()
            .map_err(|e| RegistryError::General(e.to_string()))?;

        if event_count > 0 && event_count % 500 == 0 {
            let snap_path = snapshot_path(self.log.path());

            self.write_snapshot(&snap_path, event_count as u64)
                .map_err(|e| RegistryError::General(e.to_string()))?;
        }

        Ok(())
    }

    pub fn write_snapshot(
        &self,
        path: impl AsRef<std::path::Path>,
        lsn: u64,
    ) -> std::io::Result<()> {
        let created_at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let snapshot = crate::snapshot::RegistrySnapshot::new(
            lsn,
            created_at_unix_ms,
            self.nodes.borrow().clone(),
            self.workloads.borrow().clone(),
        );

        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, &snapshot)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))?;

        Ok(())
    }

    pub fn load_snapshot(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<u64> {
        let file = std::fs::File::open(path)?;

        let snapshot: crate::snapshot::RegistrySnapshot = serde_json::from_reader(file)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))?;

        *self.nodes.borrow_mut() = snapshot.nodes;
        *self.workloads.borrow_mut() = snapshot.workloads;

        Ok(snapshot.metadata.lsn)
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        let registry = Self {
            log: EventLog::open(path)?,
            nodes: RefCell::new(HashMap::new()),
            workloads: RefCell::new(HashMap::new()),
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
        match event {
            RegistryEvent::NodeRegistered { record } => {
                self.nodes.borrow_mut().insert(record.node_id, record);
            }

            RegistryEvent::StatusUpdated {
                node_id,
                new_status,
            } => {
                if let Some(node) = self.nodes.borrow_mut().get_mut(&node_id) {
                    node.status = new_status;
                }
            }

            RegistryEvent::MetricsUpdated { node_id, metrics } => {
                if let Some(node) = self.nodes.borrow_mut().get_mut(&node_id) {
                    node.metrics = metrics;
                }
            }

            RegistryEvent::NodeTerminated { node_id } => {
                if let Some(node) = self.nodes.borrow_mut().get_mut(&node_id) {
                    node.status = OperationalStatus::Terminated;
                }
            }

            RegistryEvent::WorkloadScheduled { workload, node_id } => {
                {
                    let mut nodes = self.nodes.borrow_mut();
                    if let Some(node) = nodes.get_mut(&node_id) {
                        node.metrics.allocated_compute_cores +=
                            workload.requirements.required_compute_cores;
                        node.metrics.allocated_memory_bytes +=
                            workload.requirements.required_memory_bytes;
                    }
                }

                self.workloads
                    .borrow_mut()
                    .insert(workload.workload_id, workload);
            }

            RegistryEvent::WorkloadCompleted {
                workload_id,
                node_id,
            } => {
                let requirements = {
                    let mut workloads = self.workloads.borrow_mut();

                    if let Some(workload) = workloads.get_mut(&workload_id) {
                        workload.state = WorkloadState::Completed;
                        Some(workload.requirements.clone())
                    } else {
                        None
                    }
                };

                if let Some(requirements) = requirements {
                    let mut nodes = self.nodes.borrow_mut();

                    if let Some(node) = nodes.get_mut(&node_id) {
                        node.metrics.allocated_compute_cores = node
                            .metrics
                            .allocated_compute_cores
                            .saturating_sub(requirements.required_compute_cores);

                        node.metrics.allocated_memory_bytes = node
                            .metrics
                            .allocated_memory_bytes
                            .saturating_sub(requirements.required_memory_bytes);
                    }
                }
            }
        }
    }
}

impl Registry {
    pub fn get_workload(&self, workload_id: &Uuid) -> Option<Workload> {
        self.workloads.borrow().get(workload_id).cloned()
    }

    pub fn list_workloads(&self) -> Vec<Workload> {
        self.workloads.borrow().values().cloned().collect()
    }
}

impl RegistryEvent {
    pub fn to_active_event(&self) -> Result<active_memory::ActiveEvent, serde_json::Error> {
        let action = match self {
            RegistryEvent::NodeRegistered { .. } => "NODE_REGISTERED",
            RegistryEvent::StatusUpdated { .. } => "STATUS_UPDATED",
            RegistryEvent::MetricsUpdated { .. } => "METRICS_UPDATED",
            RegistryEvent::NodeTerminated { .. } => "NODE_TERMINATED",
            RegistryEvent::WorkloadScheduled { .. } => "WORKLOAD_SCHEDULED",
            RegistryEvent::WorkloadCompleted { .. } => "WORKLOAD_COMPLETED",
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
