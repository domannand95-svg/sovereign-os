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

        Ok(best.node.node_id)
    }
}
