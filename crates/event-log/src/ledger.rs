use active_memory::{ActiveEvent, ActiveMemory};
use serde_json::json;
use std::path::PathBuf;

use crate::EventLogError;

pub struct EventLog {
    memory: ActiveMemory,
}

impl EventLog {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, EventLogError> {
        Ok(Self {
            memory: ActiveMemory::open(path)?,
        })
    }

    pub fn record_transition(
        &self,
        action: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<(), EventLogError> {
        let event = ActiveEvent::new(
            action,
            json!({
                "source": source.into(),
                "target": target.into()
            }),
        );

        self.memory.record(&event)?;
        Ok(())
    }

    pub fn replay(&self) -> Result<Vec<ActiveEvent>, EventLogError> {
        Ok(self.memory.history()?)
    }
}
