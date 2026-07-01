use governance_wrapper::{AllocationRequest, GovernanceEngine, Scheduler, SchedulerError};
use registry_service::{CapacityMetrics, OperationalStatus, Registry};
use uuid::Uuid;

#[test]
fn test_stateful_scheduling_and_reclamation_lifecycle() {
    let ledger_path = std::env::temp_dir()
        .join(format!("sovereign_orchestration_{}.jsonl", Uuid::new_v4()));

    let node_id = Uuid::new_v4();

    let initial_metrics = CapacityMetrics {
        total_compute_cores: 8,
        allocated_compute_cores: 0,
        total_memory_bytes: 34_359_738_368,
        allocated_memory_bytes: 0,
    };

    let workload_a = AllocationRequest {
        required_compute_cores: 4,
        required_memory_bytes: 17_179_869_184,
        required_capabilities: vec!["compute-high-density".to_string()],
    };

    let workload_b = AllocationRequest {
        required_compute_cores: 6,
        required_memory_bytes: 4_294_967_296,
        required_capabilities: vec!["compute-high-density".to_string()],
    };

    {
        let governance = GovernanceEngine::open(&ledger_path).unwrap();

        governance
            .register_node(
                node_id,
                vec!["compute-high-density".to_string()],
                initial_metrics,
            )
            .unwrap();

        governance
            .update_status(node_id, OperationalStatus::Active)
            .unwrap();

        let scheduler = Scheduler::new(&governance);

        let allocated_node_id = scheduler.schedule_workload(&workload_a).unwrap();
        assert_eq!(allocated_node_id, node_id);

        let post_alloc_node = governance.registry().get_node(&node_id).unwrap();
        assert_eq!(post_alloc_node.metrics.allocated_compute_cores, 4);
        assert_eq!(post_alloc_node.metrics.allocated_memory_bytes, 17_179_869_184);

        let oversized_result = scheduler.schedule_workload(&workload_b);

        match oversized_result {
            Err(SchedulerError::NoResourceAvailability) => {}
            other => panic!("expected NoResourceAvailability, got: {:?}", other),
        }

        scheduler.release_resources(node_id, &workload_a).unwrap();

        let released_node = governance.registry().get_node(&node_id).unwrap();
        assert_eq!(released_node.metrics.allocated_compute_cores, 0);
        assert_eq!(released_node.metrics.allocated_memory_bytes, 0);
    }

    let replayed_registry = Registry::open(&ledger_path).unwrap();
    let target_record = replayed_registry.get_node(&node_id).unwrap();

    assert_eq!(target_record.status, OperationalStatus::Active);
    assert_eq!(target_record.metrics.allocated_compute_cores, 0);
    assert_eq!(target_record.metrics.allocated_memory_bytes, 0);

    let _ = std::fs::remove_file(ledger_path);
}
