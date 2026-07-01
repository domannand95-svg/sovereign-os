use governance_wrapper::{GovernanceEngine, GovernanceError};
use uuid::Uuid;

#[test]
fn governance_registers_valid_node() {
    let path = std::env::temp_dir()
        .join(format!("governance_wrapper_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&path).unwrap();

    governance
        .register_node("node-001", "validator")
        .unwrap();

    let _ = std::fs::remove_file(path);
}

#[test]
fn governance_rejects_empty_node_id() {
    let path = std::env::temp_dir()
        .join(format!("governance_wrapper_empty_node_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&path).unwrap();

    let result = governance.register_node("", "validator");

    match result {
        Err(GovernanceError::PolicyViolation(_)) => {}
        other => panic!("Expected policy violation, got: {:?}", other),
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn governance_rejects_empty_role() {
    let path = std::env::temp_dir()
        .join(format!("governance_wrapper_empty_role_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&path).unwrap();

    let result = governance.register_node("node-001", "");

    match result {
        Err(GovernanceError::PolicyViolation(_)) => {}
        other => panic!("Expected policy violation, got: {:?}", other),
    }

    let _ = std::fs::remove_file(path);
}
