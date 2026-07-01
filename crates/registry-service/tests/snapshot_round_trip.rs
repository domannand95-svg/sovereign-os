use registry_service::{CapacityMetrics, NodeRecord, OperationalStatus, Registry};
use uuid::Uuid;

#[test]
fn snapshot_round_trip_preserves_registry_state() {
    let ledger_path =
        std::env::temp_dir().join(format!("snapshot_ledger_{}.jsonl", Uuid::new_v4()));

    let snapshot_path =
        std::env::temp_dir().join(format!("snapshot_state_{}.snap", Uuid::new_v4()));

    let node_id = Uuid::new_v4();

    let registry = Registry::open(&ledger_path).unwrap();

    registry
        .register_node(NodeRecord {
            node_id,
            status: OperationalStatus::Active,
            capabilities: vec!["snapshot-test".to_string()],
            metrics: CapacityMetrics {
                total_compute_cores: 8,
                allocated_compute_cores: 2,
                total_memory_bytes: 8_589_934_592,
                allocated_memory_bytes: 1_073_741_824,
            },
        })
        .unwrap();

    registry.write_snapshot(&snapshot_path, 777).unwrap();

    let recovery_ledger =
        std::env::temp_dir().join(format!("snapshot_recovery_{}.jsonl", Uuid::new_v4()));

    let recovered = Registry::open(&recovery_ledger).unwrap();
    let restored_lsn = recovered.load_snapshot(&snapshot_path).unwrap();

    assert_eq!(restored_lsn, 777);

    let node = recovered.get_node(&node_id).unwrap();
    assert_eq!(node.node_id, node_id);
    assert_eq!(node.status, OperationalStatus::Active);
    assert_eq!(node.capabilities, vec!["snapshot-test".to_string()]);
    assert_eq!(node.metrics.allocated_compute_cores, 2);

    let _ = std::fs::remove_file(ledger_path);
    let _ = std::fs::remove_file(snapshot_path);
    let _ = std::fs::remove_file(recovery_ledger);
}
