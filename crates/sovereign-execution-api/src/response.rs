use serde::{Deserialize, Serialize};

/// Defines the boundary output from the execution kernel.
/// Encapsulates the execution report without leaking internal kernel state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernedExecutionResponse {
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub report_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    AuthorizedAndExecuted,
    AuthenticationFailed,
    ExecutionFailed(String),
}
