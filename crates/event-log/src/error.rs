use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventLogError {
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid log entry: {0}")]
    InvalidEntry(String),
}
