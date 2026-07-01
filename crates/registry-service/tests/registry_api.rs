use registry_service::{
    CapacityMetrics,
    NodeRecord,
    OperationalStatus,
    Registry,
};
use uuid::Uuid;

#[test]
fn registry_registers_node_into_event_log() {
    let path = std::env::temp_dir()
        .join(format!("registry_service_{}.jsonl", Uuid::new_v4()));

    let registry = Registry::open(&path).unwrap();

    let node_id = Uuid::new_v4();

    let node = NodeRecord {
        node_id,
        status: OperationalStatus::Initializing,
        capabilities: vec!["validator".to_string()],
        metrics: CapacityMetrics {
            total_compute_cores: 4,
            allocated_compute_cores: 0,
            total_memory_bytes: 8_589_934_592,
            allocated_memory_bytes: 0,
        },
    };

    registry.register_node(node).unwrap();

    let history = registry.history().unwrap();

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].action, "NODE_REGISTERED");
    assert_eq!(
        history[0].payload["NodeRegistered"]["record"]["node_id"],
        node_id.to_string()
    );
    assert_eq!(
        history[0].payload["NodeRegistered"]["record"]["capabilities"][0],
        "validator"
    );

    assert_eq!(registry.active_node_count(), 1);
    assert!(registry.get_node(&node_id).is_some());

    let _ = std::fs::remove_file(path);
}
