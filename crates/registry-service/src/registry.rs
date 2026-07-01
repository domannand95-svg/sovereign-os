use event_log::EventLog;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::RegistryError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeRecord {
    pub node_id: String,
    pub role: String,
}

pub struct Registry {
    log: EventLog,
}

impl Registry {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        Ok(Self {
            log: EventLog::open(path)?,
        })
    }

    pub fn register_node(&self, node: NodeRecord) -> Result<(), RegistryError> {
        self.log.record_transition(
            "NODE_REGISTERED",
            "registry-service",
            node.node_id,
        )?;

        Ok(())
    }

    pub fn history(&self) -> Result<Vec<active_memory::ActiveEvent>, RegistryError> {
        Ok(self.log.replay()?)
    }
}
