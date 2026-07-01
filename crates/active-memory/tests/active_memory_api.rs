use active_memory::{ActiveEvent, ActiveMemory};
use serde_json::json;
use uuid::Uuid;

#[test]
fn active_memory_api_round_trip() {
    let path = std::env::temp_dir().join(format!("active_memory_api_{}.jsonl", Uuid::new_v4()));

    let memory = ActiveMemory::open(&path).unwrap();

    let event = ActiveEvent::new("API_TEST", json!({"status":"ok"}));

    memory.record(&event).unwrap();

    let history = memory.history().unwrap();
    assert_eq!(history.len(), 1);

    let latest = memory.latest().unwrap().unwrap();
    assert_eq!(latest.event_id, event.event_id);

    let _ = std::fs::remove_file(path);
}
