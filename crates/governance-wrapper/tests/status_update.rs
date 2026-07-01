use governance_wrapper::{GovernanceEngine, GovernanceError};
use registry_service::CapacityMetrics;
use registry_service::OperationalStatus;
use uuid::Uuid;

#[test]
fn governance_allows_valid_status_transition() {
    let path = std::env::temp_dir().join(format!("governance_status_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&path).unwrap();

    governance
        .register_node(
            Uuid::new_v4(),
            vec!["validator".to_string()],
            CapacityMetrics {
                total_compute_cores: 0,
                allocated_compute_cores: 0,
                total_memory_bytes: 0,
                allocated_memory_bytes: 0,
            },
        )
        .unwrap();

    let history = std::fs::read_to_string(&path).unwrap();
    assert!(history.contains("NODE_REGISTERED"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn governance_rejects_unknown_node_status_update() {
    let path = std::env::temp_dir().join(format!("governance_unknown_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&path).unwrap();
    let missing_node = Uuid::new_v4();

    let result = governance.update_status(missing_node, OperationalStatus::Active);

    match result {
        Err(GovernanceError::NodeNotFound(id)) => assert_eq!(id, missing_node),
        other => panic!("expected NodeNotFound, got: {:?}", other),
    }

    let _ = std::fs::remove_file(path);
}
