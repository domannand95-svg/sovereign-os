//! Error definitions for the Base44 adapter.

#[derive(Debug, thiserror::Error)]
pub enum Base44AdapterError {
    #[error("Ingress validation failed: {0}")]
    IngressValidation(String),
    #[error("Payload too large: size {size} exceeds limit {limit}")]
    PayloadTooLarge { size: usize, limit: usize },
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("Digest mismatch: expected {expected}, got {got}")]
    DigestMismatch { expected: String, got: String },
    #[error("Execution API error: {0}")]
    ExecutionApi(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}
