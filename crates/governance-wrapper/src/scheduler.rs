use uuid::Uuid;

use crate::{AllocationRequest, GovernanceEngine, GovernanceError, ResourceAllocator};

#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("no viable compute nodes satisfy the allocation request")]
    NoResourceAvailability,

    #[error("resource release underflow for node: {0}")]
    ResourceReleaseUnderflow(Uuid),

    #[error(transparent)]
    Governance(#[from] GovernanceError),
}

pub struct Scheduler<'a> {
    governance: &'a mut GovernanceEngine,
}

impl<'a> Scheduler<'a> {
    pub fn new(governance: &'a mut GovernanceEngine) -> Self {
        Self { governance }
    }

    pub fn schedule_workload(
        &mut self,
        workload_id: Uuid,
        priority: u32,
        request: AllocationRequest,
    ) -> Result<Uuid, SchedulerError> {
        let candidates = ResourceAllocator::find_candidates(self.governance.registry(), &request);

        let best = candidates
            .first()
            .ok_or(SchedulerError::NoResourceAvailability)?;

        let node_id = best.node.node_id;

        let workload = registry_service::Workload {
            workload_id,
            priority,
            requirements: request,
            state: registry_service::WorkloadState::Running {
                assigned_node_id: node_id,
            },
        };

        let event = registry_service::RegistryEvent::WorkloadScheduled { workload, node_id };

        self.governance
            .registry_mut()
            .append_governed_event(event)
            .map_err(|e| GovernanceError::PolicyViolation(e.to_string()))?;

        Ok(node_id)
    }

    pub fn release_resources(
        &mut self,
        node_id: Uuid,
        request: &AllocationRequest,
    ) -> Result<(), SchedulerError> {
        let target_node = self
            .governance
            .registry()
            .get_node(&node_id)
            .ok_or(GovernanceError::NodeNotFound(node_id))?;

        let current_metrics = target_node.metrics.clone();

        if current_metrics.allocated_compute_cores < request.required_compute_cores
            || current_metrics.allocated_memory_bytes < request.required_memory_bytes
        {
            return Err(SchedulerError::ResourceReleaseUnderflow(node_id));
        }

        let updated_metrics = registry_service::CapacityMetrics {
            total_compute_cores: current_metrics.total_compute_cores,
            allocated_compute_cores: current_metrics.allocated_compute_cores
                - request.required_compute_cores,
            total_memory_bytes: current_metrics.total_memory_bytes,
            allocated_memory_bytes: current_metrics.allocated_memory_bytes
                - request.required_memory_bytes,
        };

        self.governance.update_metrics(node_id, updated_metrics)?;
        Ok(())
    }

    pub fn complete_workload(&mut self, workload_id: Uuid) -> Result<(), SchedulerError> {
        let workload = self
            .governance
            .registry()
            .get_workload(&workload_id)
            .ok_or_else(|| {
                GovernanceError::PolicyViolation(
                    "target workload not found in active cache".to_string(),
                )
            })?;

        let assigned_node_id = match workload.state {
            registry_service::WorkloadState::Running { assigned_node_id } => assigned_node_id,
            _ => {
                return Err(SchedulerError::Governance(
                    GovernanceError::PolicyViolation(
                        "cannot complete a workload that is not running".to_string(),
                    ),
                ));
            }
        };

        let event = registry_service::RegistryEvent::WorkloadCompleted {
            workload_id,
            node_id: assigned_node_id,
        };

        self.governance
            .registry_mut()
            .append_governed_event(event)
            .map_err(|e| GovernanceError::PolicyViolation(e.to_string()))?;

        Ok(())
    }
}
