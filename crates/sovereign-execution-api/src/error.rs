use serde::{Deserialize, Serialize};

/// Standardized error boundaries for the Sovereign Execution API.
///
/// Error categories follow the governance precedence chain:
///
/// Validation
///     >
/// Authorization
///     >
/// Kernel Rejection
///     >
/// Execution Failure
///     >
/// Transport Failure
///
/// Detailed diagnostics remain available internally and must not
/// be exposed through ExecutionStatus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionApiError {
    /// Request structure, payload, digest, timestamp, or boundary validation failure.
    ValidationFailure(String),

    /// Authorization receipt invalid, expired, or authentication denied.
    AuthorizationFailure(String),

    /// Kernel explicitly rejected an otherwise valid execution request.
    KernelRejection(String),

    /// Runtime execution failure after authorization succeeded.
    ExecutionFailure(String),

    /// Serialization, encoding, or communication boundary failure.
    TransportFailure(String),
}

impl std::fmt::Display for ExecutionApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidationFailure(msg) => {
                write!(f, "Validation failure: {}", msg)
            }
            Self::AuthorizationFailure(msg) => {
                write!(f, "Authorization failure: {}", msg)
            }
            Self::KernelRejection(msg) => {
                write!(f, "Kernel rejection: {}", msg)
            }
            Self::ExecutionFailure(msg) => {
                write!(f, "Execution failure: {}", msg)
            }
            Self::TransportFailure(msg) => {
                write!(f, "Transport failure: {}", msg)
            }
        }
    }
}

impl std::error::Error for ExecutionApiError {}
