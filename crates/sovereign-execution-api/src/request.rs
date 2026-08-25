use crate::{AuthorizationReceiptRef, BoundaryParseError, CanonicalAction, ExecutionId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernedExecutionRequest {
    execution_id: ExecutionId,
    authorization_receipt: AuthorizationReceiptRef,
    action: CanonicalAction,
}

impl GovernedExecutionRequest {
    pub fn new(
        execution_id: ExecutionId,
        authorization_receipt: AuthorizationReceiptRef,
        action: CanonicalAction,
    ) -> Self {
        Self {
            execution_id,
            authorization_receipt,
            action,
        }
    }

    pub fn parse(
        execution_id: impl Into<String>,
        authorization_receipt_hex: &str,
        canonical_action: &[u8],
    ) -> Result<Self, BoundaryParseError> {
        Ok(Self::new(
            ExecutionId::parse(execution_id)?,
            AuthorizationReceiptRef::parse_hex(authorization_receipt_hex)?,
            CanonicalAction::parse(canonical_action)?,
        ))
    }

    pub fn execution_id(&self) -> &ExecutionId {
        &self.execution_id
    }
    pub fn authorization_receipt(&self) -> AuthorizationReceiptRef {
        self.authorization_receipt
    }
    pub fn action(&self) -> &CanonicalAction {
        &self.action
    }
}
