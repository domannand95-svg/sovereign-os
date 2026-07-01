use governance_wrapper::GovernanceEngine;
use registry_service::{CapacityMetrics, OperationalStatus, Registry};
use uuid::Uuid;

#[test]
fn test_node_lifecycle_e2e_replay() {
    let ledger_path = std::env::temp_dir()
        .join(format!("sovereign_replay_{}.jsonl", Uuid::new_v4()));

    let node_id = Uuid::new_v4();
    let initial_capabilities = vec!["compute-tier-1".to_string(), "storage-local".to_string()];

    let initial_metrics = CapacityMetrics {
        total_compute_cores: 16,
        allocated_compute_cores: 0,
        total_memory_bytes: 68_719_476_736,
        allocated_memory_bytes: 0,
    };

    let updated_metrics = CapacityMetrics {
        total_compute_cores: 16,
        allocated_compute_cores: 8,
        total_memory_bytes: 68_719_476_736,
        allocated_memory_bytes: 34_359_738_368,
    };

    {
        let engine = GovernanceEngine::open(&ledger_path).unwrap();

        engine.register_node(node_id, initial_capabilities.clone(), initial_metrics).unwrap();
        engine.update_status(node_id, OperationalStatus::Active).unwrap();
        engine.update_metrics(node_id, updated_metrics.clone()).unwrap();
        engine.terminate_node(node_id).unwrap();
    }

    let replayed_registry = Registry::open(&ledger_path).unwrap();
    let record = replayed_registry.get_node(&node_id).unwrap();

    assert_eq!(record.node_id, node_id);
    assert_eq!(record.status, OperationalStatus::Terminated);
    assert_eq!(record.metrics, updated_metrics);
    assert_eq!(record.capabilities, initial_capabilities);

    let _ = std::fs::remove_file(ledger_path);
}
