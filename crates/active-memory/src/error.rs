use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActiveMemoryError {
    #[error("I/O error encountered: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid event state: {0}")]
    InvalidEvent(String),
}
