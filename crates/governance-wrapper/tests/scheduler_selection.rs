use governance_wrapper::{AllocationRequest, GovernanceEngine, Scheduler};
use registry_service::{CapacityMetrics, OperationalStatus};
use uuid::Uuid;

#[test]
fn scheduler_selects_highest_ranked_viable_node() {
    let path = std::env::temp_dir()
        .join(format!("scheduler_selection_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&path).unwrap();

    let small_node = Uuid::new_v4();
    let large_node = Uuid::new_v4();

    governance
        .register_node(
            small_node,
            vec!["compute-tier-1".to_string()],
            CapacityMetrics {
                total_compute_cores: 8,
                allocated_compute_cores: 0,
                total_memory_bytes: 8_589_934_592,
                allocated_memory_bytes: 0,
            },
        )
        .unwrap();

    governance
        .update_status(small_node, OperationalStatus::Active)
        .unwrap();

    governance
        .register_node(
            large_node,
            vec!["compute-tier-1".to_string()],
            CapacityMetrics {
                total_compute_cores: 32,
                allocated_compute_cores: 0,
                total_memory_bytes: 68_719_476_736,
                allocated_memory_bytes: 0,
            },
        )
        .unwrap();

    governance
        .update_status(large_node, OperationalStatus::Active)
        .unwrap();

    let scheduler = Scheduler::new(&governance);

    let request = AllocationRequest {
        required_compute_cores: 4,
        required_memory_bytes: 4_294_967_296,
        required_capabilities: vec!["compute-tier-1".to_string()],
    };

    let selected = scheduler.schedule_workload(&request).unwrap();

    assert_eq!(selected, large_node);

    let _ = std::fs::remove_file(path);
}
