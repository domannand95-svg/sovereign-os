use uuid::Uuid;

use crate::{
    AllocationRequest,
    GovernanceEngine,
    GovernanceError,
    ResourceAllocator,
};

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
    governance: &'a GovernanceEngine,
}

impl<'a> Scheduler<'a> {
    pub fn new(governance: &'a GovernanceEngine) -> Self {
        Self { governance }
    }

    /// Returns the highest-ranked node for a workload request.
    ///
    /// This initial implementation is read-only. A later sprint will
    /// commit the allocation back through GovernanceEngine.
    pub fn schedule_workload(
        &self,
        request: &AllocationRequest,
    ) -> Result<Uuid, SchedulerError> {
        let candidates =
            ResourceAllocator::find_candidates(self.governance.registry(), request);

        let best = candidates
            .first()
            .ok_or(SchedulerError::NoResourceAvailability)?;

        let node_id = best.node.node_id;

        let current = best.node.metrics.clone();

        let updated = registry_service::CapacityMetrics {
            total_compute_cores: current.total_compute_cores,
            allocated_compute_cores: current.allocated_compute_cores
                + request.required_compute_cores,
            total_memory_bytes: current.total_memory_bytes,
            allocated_memory_bytes: current.allocated_memory_bytes
                + request.required_memory_bytes,
        };

        self.governance
            .update_metrics(node_id, updated)?;

        Ok(node_id)
    }

    pub fn release_resources(
        &self,
        node_id: Uuid,
        request: &AllocationRequest,
    ) -> Result<(), SchedulerError> {
        let target_node = self
            .governance
            .registry()
            .get_node(&node_id)
            .ok_or(GovernanceError::NodeNotFound(node_id))?;

        let current_metrics = target_node.metrics;

        if current_metrics.allocated_compute_cores < request.required_compute_cores
            || current_metrics.allocated_memory_bytes < request.required_memory_bytes
        {
            return Err(SchedulerError::ResourceReleaseUnderflow(node_id));
        }

        let updated_metrics = registry_service::CapacityMetrics {
            total_compute_cores: current_metrics.total_compute_cores,
            allocated_compute_cores: current_metrics
                .allocated_compute_cores
                .saturating_sub(request.required_compute_cores),
            total_memory_bytes: current_metrics.total_memory_bytes,
            allocated_memory_bytes: current_metrics
                .allocated_memory_bytes
                .saturating_sub(request.required_memory_bytes),
        };

        self.governance.update_metrics(node_id, updated_metrics)?;

        Ok(())
    }
}
