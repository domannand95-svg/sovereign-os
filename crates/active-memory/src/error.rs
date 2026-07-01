use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActiveMemoryError {
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("file I/O failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid event: {0}")]
    InvalidEvent(String),
}
