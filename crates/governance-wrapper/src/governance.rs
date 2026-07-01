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
        node_id: Uuid,
        capabilities: Vec<String>,
        metrics: CapacityMetrics,
    ) -> Result<(), GovernanceError> {
        if self.registry.get_node(&node_id).is_some() {
            return Err(GovernanceError::DuplicateNode(node_id));
        }

        if capabilities.is_empty() {
            return Err(GovernanceError::PolicyViolation(
                "capabilities cannot be empty".to_string(),
            ));
        }

        Self::validate_capacity(&metrics)?;

        let node = NodeRecord {
            node_id,
            status: OperationalStatus::Initializing,
            capabilities,
            metrics,
        };

        self.registry.register_node(node)?;

        Ok(())
    }

    pub fn update_status(
        &self,
        node_id: Uuid,
        new_status: OperationalStatus,
    ) -> Result<(), GovernanceError> {
        self.validate_node_exists(&node_id)?;

        let current_record = self
            .registry
            .get_node(&node_id)
            .ok_or(GovernanceError::NodeNotFound(node_id))?;

        self.validate_transition(current_record.status, new_status)?;

        self.registry.append_registry_event(
            registry_service::RegistryEvent::StatusUpdated {
                node_id,
                new_status,
            },
        )?;

        Ok(())
    }

    pub fn update_metrics(
        &self,
        node_id: Uuid,
        new_metrics: CapacityMetrics,
    ) -> Result<(), GovernanceError> {
        self.validate_node_exists(&node_id)?;
        Self::validate_capacity(&new_metrics)?;

        self.registry.append_registry_event(
            registry_service::RegistryEvent::MetricsUpdated {
                node_id,
                metrics: new_metrics,
            },
        )?;

        Ok(())
    }

    fn validate_node_exists(&self, node_id: &Uuid) -> Result<(), GovernanceError> {
        if self.registry.get_node(node_id).is_none() {
            return Err(GovernanceError::NodeNotFound(*node_id));
        }

        Ok(())
    }

    fn validate_transition(
        &self,
        current: OperationalStatus,
        next: OperationalStatus,
    ) -> Result<(), GovernanceError> {
        use OperationalStatus::*;

        match (current, next) {
            (Initializing, Active) => Ok(()),
            (Active, Dormant) => Ok(()),
            (Dormant, Active) => Ok(()),
            (Active, Terminated) => Ok(()),

            (Terminated, _) => Err(GovernanceError::IllegalTransition(
                "terminated nodes cannot transition".to_string(),
            )),

            _ => Err(GovernanceError::IllegalTransition(
                format!("illegal transition: {:?} -> {:?}", current, next),
            )),
        }
    }


    fn validate_capacity(metrics: &CapacityMetrics) -> Result<(), GovernanceError> {
        if metrics.allocated_compute_cores > metrics.total_compute_cores {
            return Err(GovernanceError::InvalidCapacity(
                "allocated compute cores exceed total compute cores".to_string(),
            ));
        }

        if metrics.allocated_memory_bytes > metrics.total_memory_bytes {
            return Err(GovernanceError::InvalidCapacity(
                "allocated memory bytes exceed total memory bytes".to_string(),
            ));
        }

        Ok(())
    }
}
