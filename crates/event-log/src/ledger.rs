use active_memory::{ActiveEvent, ActiveMemory};
use serde_json::json;
use std::path::PathBuf;

use crate::EventLogError;

pub struct EventLog {
    memory: ActiveMemory,
    path: PathBuf,
}

impl EventLog {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, EventLogError> {
        let path = path.into();
        Ok(Self {
            memory: ActiveMemory::open(path.clone())?,
            path,
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

    pub fn append_active_event(&self, event: &ActiveEvent) -> Result<(), EventLogError> {
        self.memory.record(event)?;
        Ok(())
    }

    pub fn replay(&self) -> Result<Vec<ActiveEvent>, EventLogError> {
        Ok(self.memory.history()?)
    }

    pub fn len(&self) -> Result<usize, EventLogError> {
        Ok(self.memory.history()?.len())
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}
