use active_memory::{ActiveEvent, ActiveMemoryError, StorageEngine};
use serde_json::json;
use std::io::Write;

#[test]
fn test_persistence_lifecycle_loop() {
    let test_file_path = std::env::temp_dir()
        .join(format!("sovereign_test_lifecycle_{}.jsonl", uuid::Uuid::new_v4()));

    let engine = StorageEngine::new(&test_file_path).expect("Failed to initialize storage engine");

    let event_1 = ActiveEvent::new("NODE_init", json!({ "node_id": "test-node-01", "status": "active" }));
    let event_2 = ActiveEvent::new("METRICS_snapshot", json!({ "metrics": { "cpu_utilization": 12.5 } }));

    engine.append_event(&event_1).expect("Failed to append event 1");
    engine.append_event(&event_2).expect("Failed to append event 2");

    let recovered_events = engine.load_events().expect("Failed to load events from disk");

    assert_eq!(recovered_events.len(), 2);
    assert_eq!(recovered_events[0].event_id, event_1.event_id);
    assert_eq!(recovered_events[1].event_id, event_2.event_id);

    let _ = std::fs::remove_file(test_file_path);
}

#[test]
fn test_empty_log_returns_empty_vector() {
    let test_file_path = std::env::temp_dir()
        .join(format!("sovereign_test_empty_{}.jsonl", uuid::Uuid::new_v4()));

    let engine = StorageEngine::new(&test_file_path).expect("Failed to initialize empty storage engine");
    let recovered_events = engine.load_events().expect("Loading an empty file must succeed");

    assert!(recovered_events.is_empty());

    let _ = std::fs::remove_file(test_file_path);
}

#[test]
fn test_corrupted_log_line_surfaces_serialization_error() {
    let test_file_path = std::env::temp_dir()
        .join(format!("sovereign_test_corrupt_{}.jsonl", uuid::Uuid::new_v4()));

    let engine = StorageEngine::new(&test_file_path).expect("Failed to initialize engine");

    let event = ActiveEvent::new("VALID_event", json!({ "status": "ok" }));
    engine.append_event(&event).expect("Failed to write valid event");

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&test_file_path)
        .expect("Failed to open target for raw injection");

    writeln!(file, "{{ INVALID CORRUPTED JSON STRING NODE }}")
        .expect("Failed to write corruption line");

    match engine.load_events() {
        Err(ActiveMemoryError::Serialization(_)) => {}
        other => panic!("Expected ActiveMemoryError::Serialization, got: {:?}", other),
    }

    let _ = std::fs::remove_file(test_file_path);
}

#[test]
fn test_deep_nested_directory_auto_creation() {
    let root = std::env::temp_dir()
        .join(format!("gusa_nested_layer_a_{}", uuid::Uuid::new_v4()));

    let nested_file_path = root
        .join("layer_b")
        .join("layer_c")
        .join("storage_node.jsonl");

    let engine = StorageEngine::new(&nested_file_path)
        .expect("StorageEngine failed to create nested directory tree");

    let test_event = ActiveEvent::new("NESTED_write", json!({ "dir_created": true }));
    engine.append_event(&test_event).expect("Failed to write nested event");

    let _ = std::fs::remove_dir_all(root);
}
