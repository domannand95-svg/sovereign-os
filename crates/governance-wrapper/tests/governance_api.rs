use governance_wrapper::{GovernanceEngine, GovernanceError};
use registry_service::CapacityMetrics;
use uuid::Uuid;

fn zero_metrics() -> CapacityMetrics {
    CapacityMetrics {
        total_compute_cores: 0,
        allocated_compute_cores: 0,
        total_memory_bytes: 0,
        allocated_memory_bytes: 0,
    }
}

#[test]
fn governance_registers_valid_node() {
    let path = std::env::temp_dir()
        .join(format!("governance_valid_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&path).unwrap();

    governance
        .register_node(
            Uuid::new_v4(),
            vec!["validator".to_string()],
            zero_metrics(),
        )
        .unwrap();

    let _ = std::fs::remove_file(path);
}

#[test]
fn governance_rejects_empty_capabilities() {
    let path = std::env::temp_dir()
        .join(format!("governance_empty_capabilities_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&path).unwrap();

    let result = governance.register_node(
        Uuid::new_v4(),
        Vec::new(),
        zero_metrics(),
    );

    match result {
        Err(GovernanceError::PolicyViolation(_)) => {}
        other => panic!("expected PolicyViolation, got: {:?}", other),
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn governance_rejects_duplicate_node() {
    let path = std::env::temp_dir()
        .join(format!("governance_duplicate_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&path).unwrap();
    let node_id = Uuid::new_v4();

    governance
        .register_node(
            node_id,
            vec!["validator".to_string()],
            zero_metrics(),
        )
        .unwrap();

    let result = governance.register_node(
        node_id,
        vec!["validator".to_string()],
        zero_metrics(),
    );

    match result {
        Err(GovernanceError::DuplicateNode(id)) => assert_eq!(id, node_id),
        other => panic!("expected DuplicateNode, got: {:?}", other),
    }

    let _ = std::fs::remove_file(path);
}
