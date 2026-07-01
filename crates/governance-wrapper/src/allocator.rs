use registry_service::{CapacityMetrics, NodeRecord, Registry};

#[derive(Debug, Clone)]
pub struct WorkloadProfile {
    pub required_compute_cores: u32,
    pub required_memory_bytes: u64,
}

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
        workload: &WorkloadProfile,
    ) -> Vec<PlacementCandidate> {
        let mut candidates = registry
            .list_nodes()
            .into_iter()
            .filter_map(|node| {
                let metrics: &CapacityMetrics = &node.metrics;

                let available_compute =
                    metrics.total_compute_cores.saturating_sub(metrics.allocated_compute_cores);

                let available_memory =
                    metrics.total_memory_bytes.saturating_sub(metrics.allocated_memory_bytes);

                if available_compute >= workload.required_compute_cores
                    && available_memory >= workload.required_memory_bytes
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
                .then(
                    b.available_memory_bytes
                        .cmp(&a.available_memory_bytes),
                )
        });

        candidates
    }
}
