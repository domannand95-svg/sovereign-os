use governance_wrapper::GovernanceEngine;
use registry_service::CapacityMetrics;
use uuid::Uuid;

#[test]
fn governance_to_storage_end_to_end_loop() {
    let ledger_path = std::env::temp_dir()
        .join(format!("sovereign_system_e2e_{}.jsonl", Uuid::new_v4()));

    let governance = GovernanceEngine::open(&ledger_path)
        .expect("failed to initialize governance engine");

    governance
        .register_node(Uuid::new_v4(), vec!["validator".to_string()], CapacityMetrics { total_compute_cores: 0, allocated_compute_cores: 0, total_memory_bytes: 0, allocated_memory_bytes: 0 })
        .expect("failed to register node through governance layer");

    assert!(ledger_path.exists(), "ledger file was not created");

    let raw = std::fs::read_to_string(&ledger_path)
        .expect("failed to read ledger file");

    assert!(
        raw.contains("NODE_REGISTERED"),
        "ledger does not contain node registration event"
    );

    assert!(
        raw.contains("NodeRegistered"),
        "ledger does not contain typed registry event"
    );

    assert!(
        raw.contains("validator"),
        "ledger does not contain registered capability"
    );

    let _ = std::fs::remove_file(ledger_path);
}
