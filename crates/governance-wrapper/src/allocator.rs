use registry_service::AllocationRequest;
use registry_service::{NodeRecord, OperationalStatus, Registry};

#[derive(Debug, Clone)]
pub struct PlacementCandidate {
    pub node: NodeRecord,
    pub available_compute_cores: u32,
    pub available_memory_bytes: u64,
}

pub struct ResourceAllocator;

impl ResourceAllocator {
    pub fn find_candidates(
        registry: &Registry,
        request: &AllocationRequest,
    ) -> Vec<PlacementCandidate> {
        Self::find_candidates_from_nodes(registry.list_nodes(), request)
    }

    pub fn find_candidates_from_nodes(
        nodes: Vec<NodeRecord>,
        request: &AllocationRequest,
    ) -> Vec<PlacementCandidate> {
        let mut candidates = nodes
            .into_iter()
            .filter(|node| node.status == OperationalStatus::Active)
            .filter(|node| {
                request
                    .required_capabilities
                    .iter()
                    .all(|required| node.capabilities.contains(required))
            })
            .filter_map(|node| {
                let available_compute = node
                    .metrics
                    .total_compute_cores
                    .saturating_sub(node.metrics.allocated_compute_cores);

                let available_memory = node
                    .metrics
                    .total_memory_bytes
                    .saturating_sub(node.metrics.allocated_memory_bytes);

                if available_compute >= request.required_compute_cores
                    && available_memory >= request.required_memory_bytes
                {
                    Some(PlacementCandidate {
                        node,
                        available_compute_cores: available_compute,
                        available_memory_bytes: available_memory,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        candidates.sort_by(|a, b| {
            b.available_compute_cores
                .cmp(&a.available_compute_cores)
                .then(b.available_memory_bytes.cmp(&a.available_memory_bytes))
        });

        candidates
    }
}
