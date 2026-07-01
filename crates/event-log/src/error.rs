use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventLogError {
    #[error("memory error: {0}")]
    Memory(#[from] active_memory::ActiveMemoryError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid event: {0}")]
    InvalidEvent(String),
}
