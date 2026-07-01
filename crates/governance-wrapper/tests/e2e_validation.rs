use governance_wrapper::GovernanceEngine;
use uuid::Uuid;

#[test]
fn governance_to_storage_end_to_end_loop() {
    let ledger_path = std::env::temp_dir()
        .join(format!("sovereign_system_e2e_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&ledger_path)
        .expect("failed to initialize governance engine");

    governance
        .register_node("node-e2e-001", "validator")
        .expect("failed to register node through governance layer");

    assert!(ledger_path.exists(), "ledger file was not created");

    let raw = std::fs::read_to_string(&ledger_path)
        .expect("failed to read ledger file");

    assert!(
        raw.contains("NODE_REGISTERED"),
        "ledger does not contain node registration event"
    );

    assert!(
        raw.contains("registry-service"),
        "ledger does not contain registry source"
    );

    let _ = std::fs::remove_file(ledger_path);
}
