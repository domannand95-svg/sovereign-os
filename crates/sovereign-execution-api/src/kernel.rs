use crate::{AuthorizationReceiptRef, CanonicalAction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KernelExecutionRequest {
    authorization_receipt: AuthorizationReceiptRef,
    action: CanonicalAction,
}

impl KernelExecutionRequest {
    pub(crate) fn new(
        authorization_receipt: AuthorizationReceiptRef,
        action: CanonicalAction,
    ) -> Self {
        Self {
            authorization_receipt,
            action,
        }
    }
    pub fn authorization_receipt(&self) -> AuthorizationReceiptRef {
        self.authorization_receipt
    }
    pub fn action(&self) -> &CanonicalAction {
        &self.action
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KernelExecutionResponse {
    pub report_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KernelExecutionError {
    AuthenticationFailure,
    OperationRejected(String),
    IntegrityFailure,
    ExecutionFailure(String),
}

/// This trait transports validated requests; it grants no authority.
pub trait KernelInvoker {
    fn invoke_kernel(
        &self,
        request: KernelExecutionRequest,
    ) -> Result<KernelExecutionResponse, KernelExecutionError>;
}
