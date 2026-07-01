use registry_service::{NodeRecord, Registry};
use uuid::Uuid;

#[test]
fn registry_registers_node_into_event_log() {
    let path = std::env::temp_dir()
        .join(format!("registry_service_{}.jsonl", Uuid::new_v4()));

    let registry = Registry::open(&path).unwrap();

    let node = NodeRecord {
        node_id: "node-001".to_string(),
        role: "validator".to_string(),
    };

    registry.register_node(node).unwrap();

    let history = registry.history().unwrap();

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].action, "NODE_REGISTERED");
    assert_eq!(history[0].payload["source"], "registry-service");
    assert_eq!(history[0].payload["target"], "node-001");

    let _ = std::fs::remove_file(path);
}
