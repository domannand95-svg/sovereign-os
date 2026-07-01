use governance_wrapper::GovernanceEngine;
use registry_service::{CapacityMetrics, OperationalStatus, Registry};
use uuid::Uuid;

fn setup_populated_test_registry(ledger_path: &std::path::Path) -> (Uuid, Uuid) {
    let engine = GovernanceEngine::open(ledger_path).unwrap();

    let node_active_id = Uuid::new_v4();
    let node_dormant_id = Uuid::new_v4();

    let dummy_metrics = CapacityMetrics {
        total_compute_cores: 4,
        allocated_compute_cores: 0,
        total_memory_bytes: 4_294_967_296,
        allocated_memory_bytes: 0,
    };

    engine
        .register_node(
            node_active_id,
            vec!["validator".to_string(), "edge".to_string()],
            dummy_metrics.clone(),
        )
        .unwrap();

    engine
        .update_status(node_active_id, OperationalStatus::Active)
        .unwrap();

    engine
        .register_node(
            node_dormant_id,
            vec!["storage".to_string()],
            dummy_metrics,
        )
        .unwrap();

    engine
        .update_status(node_dormant_id, OperationalStatus::Active)
        .unwrap();

    engine
        .update_status(node_dormant_id, OperationalStatus::Dormant)
        .unwrap();

    (node_active_id, node_dormant_id)
}

#[test]
fn test_discovery_query_behaviors() {
    let ledger_path = std::env::temp_dir()
        .join(format!("sovereign_discovery_{}.jsonl", Uuid::new_v4()));

    let (active_id, dormant_id) = setup_populated_test_registry(&ledger_path);

    let registry = Registry::open(&ledger_path).unwrap();

    let all_nodes = registry.list_nodes();
    assert_eq!(all_nodes.len(), 2);
    assert!(all_nodes.iter().any(|node| node.node_id == active_id));
    assert!(all_nodes.iter().any(|node| node.node_id == dormant_id));

    let active_nodes = registry.nodes_by_status(OperationalStatus::Active);
    assert_eq!(active_nodes.len(), 1);
    assert_eq!(active_nodes[0].node_id, active_id);

    let validators = registry.nodes_with_capability("validator");
    assert_eq!(validators.len(), 1);
    assert_eq!(validators[0].node_id, active_id);

    let empty_status = registry.nodes_by_status(OperationalStatus::Terminated);
    assert!(empty_status.is_empty());

    let empty_caps = registry.nodes_with_capability("quantum-compute");
    assert!(empty_caps.is_empty());

    let _ = std::fs::remove_file(ledger_path);
}
