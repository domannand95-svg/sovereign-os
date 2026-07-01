use registry_service::{
    CapacityMetrics,
    NodeRecord,
    OperationalStatus,
    Registry,
};
use std::path::PathBuf;
use uuid::Uuid;

use crate::GovernanceError;

pub struct GovernanceEngine {
    registry: Registry,
}

impl GovernanceEngine {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, GovernanceError> {
        Ok(Self {
            registry: Registry::open(path)?,
        })
    }

    pub fn register_node(
        &self,
        node_id: impl Into<String>,
        role: impl Into<String>,
    ) -> Result<(), GovernanceError> {
        let node_id = node_id.into();
        let role = role.into();

        if node_id.trim().is_empty() {
            return Err(GovernanceError::PolicyViolation(
                "node_id cannot be empty".to_string(),
            ));
        }

        if role.trim().is_empty() {
            return Err(GovernanceError::PolicyViolation(
                "role cannot be empty".to_string(),
            ));
        }

        let node = NodeRecord {
            node_id: Uuid::new_v4(),
            status: OperationalStatus::Initializing,
            capabilities: vec![role],
            metrics: CapacityMetrics {
                total_compute_cores: 0,
                allocated_compute_cores: 0,
                total_memory_bytes: 0,
                allocated_memory_bytes: 0,
            },
        };

        self.registry.register_node(node)?;

        Ok(())
    }
}
