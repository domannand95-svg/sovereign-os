use active_memory::{ActiveEvent, StorageEngine};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn storage_engine_round_trip() {
    let mut path = std::env::temp_dir();

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    path.push(format!("active-memory-{}.jsonl", nanos));

    let engine = StorageEngine::new(&path).unwrap();

    let event = ActiveEvent::new(
        "storage_test",
        json!({
            "message": "storage engine verification"
        }),
    );

    engine.append_event(&event).unwrap();

    let events = engine.load_events().unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action, "storage_test");
    assert_eq!(
        events[0].payload["message"],
        "storage engine verification"
    );
}
