use event_log::EventLog;
use uuid::Uuid;

#[test]
fn event_log_round_trip() {
    let path = std::env::temp_dir().join(format!("event_log_{}.jsonl", Uuid::new_v4()));

    let log = EventLog::open(&path).unwrap();

    log.record_transition("NODE_REGISTERED", "bootstrap", "node-001")
        .unwrap();

    let history = log.replay().unwrap();

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].action, "NODE_REGISTERED");
    assert_eq!(history[0].payload["source"], "bootstrap");
    assert_eq!(history[0].payload["target"], "node-001");

    let _ = std::fs::remove_file(path);
}
