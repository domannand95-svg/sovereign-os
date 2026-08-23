use serde::{Deserialize, Serialize};

/// Typed request crossing from the execution API boundary
/// into the governed execution kernel contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KernelExecutionRequest {
    pub authorization_receipt_id: String,
    pub operation_payload: Vec<u8>,
}

/// Typed response returned from the governed execution kernel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KernelExecutionResponse {
    pub report_reference: String,
}

/// Typed failures crossing the kernel boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KernelExecutionError {
    AuthenticationFailure,
    OperationRejected(String),
    IntegrityFailure,
    ExecutionFailure(String),
}

/// Dependency injection boundary between the API layer
/// and the execution kernel.
///
/// This trait does not grant authority.
/// It only transports already-governed execution requests.
pub trait KernelInvoker {
    fn invoke_kernel(
        &self,
        request: KernelExecutionRequest,
    ) -> Result<KernelExecutionResponse, KernelExecutionError>;
}