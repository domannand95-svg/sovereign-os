use governance_wrapper::{AllocationRequest, ResourceAllocator};
use registry_service::{CapacityMetrics, NodeRecord, OperationalStatus};
use uuid::Uuid;

fn node(
    status: OperationalStatus,
    capabilities: Vec<&str>,
    total_cores: u32,
    allocated_cores: u32,
    total_memory: u64,
    allocated_memory: u64,
) -> NodeRecord {
    NodeRecord {
        node_id: Uuid::new_v4(),
        status,
        capabilities: capabilities.into_iter().map(String::from).collect(),
        metrics: CapacityMetrics {
            total_compute_cores: total_cores,
            allocated_compute_cores: allocated_cores,
            total_memory_bytes: total_memory,
            allocated_memory_bytes: allocated_memory,
        },
    }
}

#[test]
fn allocator_filters_and_ranks_candidates() {
    let underpowered = node(
        OperationalStatus::Active,
        vec!["compute-tier-1"],
        4,
        2,
        4_294_967_296,
        0,
    );

    let efficient = node(
        OperationalStatus::Active,
        vec!["compute-tier-1"],
        16,
        4,
        34_359_738_368,
        0,
    );

    let high_perf = node(
        OperationalStatus::Active,
        vec!["compute-tier-1"],
        64,
        4,
        137_438_953_472,
        0,
    );

    let terminated = node(
        OperationalStatus::Terminated,
        vec!["compute-tier-1"],
        32,
        0,
        68_719_476_736,
        0,
    );

    let high_perf_id = high_perf.node_id;
    let efficient_id = efficient.node_id;

    let pool = vec![underpowered, efficient, high_perf, terminated];

    let request = AllocationRequest {
        required_compute_cores: 8,
        required_memory_bytes: 8_589_934_592,
        required_capabilities: vec!["compute-tier-1".to_string()],
    };

    let candidates = ResourceAllocator::find_candidates_from_nodes(pool.clone(), &request);

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].node.node_id, high_perf_id);
    assert_eq!(candidates[1].node.node_id, efficient_id);

    let impossible = AllocationRequest {
        required_compute_cores: 128,
        required_memory_bytes: 1024,
        required_capabilities: vec!["compute-tier-1".to_string()],
    };

    let empty = ResourceAllocator::find_candidates_from_nodes(pool, &impossible);
    assert!(empty.is_empty());
}
