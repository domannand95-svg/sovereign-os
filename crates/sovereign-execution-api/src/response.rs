use crate::ExecutionId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernedExecutionResponse {
    pub execution_id: ExecutionId,
    pub status: ExecutionStatus,
    pub report_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    AuthorizedAndExecuted,
    AuthenticationFailed,
    ExecutionFailed,
}
