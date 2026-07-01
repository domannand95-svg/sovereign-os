use crate::{ActiveEvent, ActiveMemoryError, StorageEngine};

pub struct ActiveMemory {
    storage: StorageEngine,
}

impl ActiveMemory {
    pub fn open(path: impl Into<std::path::PathBuf>) -> Result<Self, ActiveMemoryError> {
        Ok(Self {
            storage: StorageEngine::new(path)?,
        })
    }

    pub fn record(&self, event: &ActiveEvent) -> Result<(), ActiveMemoryError> {
        self.storage.append_event(event)
    }

    pub fn history(&self) -> Result<Vec<ActiveEvent>, ActiveMemoryError> {
        self.storage.load_events()
    }

    pub fn latest(&self) -> Result<Option<ActiveEvent>, ActiveMemoryError> {
        Ok(self.history()?.pop())
    }
}
