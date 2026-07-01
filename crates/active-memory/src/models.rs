use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub payload: Value,
}

impl ActiveEvent {
    pub fn new(action: impl Into<String>, payload: Value) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: action.into(),
            payload,
        }
    }
}
