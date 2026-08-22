use serde::{Deserialize, Serialize};

/// Defines the boundary output from the execution kernel.
/// Encapsulates the execution report without leaking internal kernel state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernedExecutionResponse {
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub report_reference: Option<String>,
}

/// Execution outcome status — sanitized for external consumers.
///
/// This enum intentionally omits error details to prevent leaking
/// internal kernel state. Detailed failure information belongs in
/// the audit layer, not the external execution API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Operation authorized and successfully executed.
    AuthorizedAndExecuted,

    /// Receipt authentication failed.
    AuthenticationFailed,

    /// Execution failed after successful authentication.
    /// Detailed diagnostics remain inside audit boundaries.
    ExecutionFailed,
}
