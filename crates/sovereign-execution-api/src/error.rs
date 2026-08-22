use serde::{Deserialize, Serialize};

/// Standardized error boundaries for the Sovereign Execution API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionApiError {
    SerializationFault(String),
    InvalidReceipt(String),
    KernelFault(String),
}

impl std::fmt::Display for ExecutionApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationFault(msg) => write!(f, "Serialization fault: {}", msg),
            Self::InvalidReceipt(msg) => write!(f, "Invalid authorization receipt: {}", msg),
            Self::KernelFault(msg) => write!(f, "Kernel fault: {}", msg),
        }
    }
}

impl std::error::Error for ExecutionApiError {}
