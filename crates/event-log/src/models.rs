use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub entry_id: Uuid,
    pub event_type: String,
    pub payload: Value,
}

impl LogEntry {
    pub fn new(event_type: impl Into<String>, payload: Value) -> Self {
        Self {
            entry_id: Uuid::new_v4(),
            event_type: event_type.into(),
            payload,
        }
    }
}
