use serde::{Deserialize, Serialize};

/// Defines the boundary input for a governed execution request.
/// Ensures the execution kernel only receives validated, serializable parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernedExecutionRequest {
    pub execution_id: String,
    pub authorization_receipt_id: String,
    pub operation_payload: Vec<u8>,
}

impl GovernedExecutionRequest {
    pub fn new(
        execution_id: String,
        authorization_receipt_id: String,
        operation_payload: Vec<u8>,
    ) -> Self {
        Self {
            execution_id,
            authorization_receipt_id,
            operation_payload,
        }
    }
}
