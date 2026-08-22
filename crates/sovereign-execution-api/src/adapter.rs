use crate::{
    ExecutionApiError,
    ExecutionStatus,
    GovernedExecutionRequest,
    GovernedExecutionResponse,
    KernelExecutionRequest,
    KernelInvoker,
};

/// API translation layer.
///
/// Converts external execution requests into typed kernel requests,
/// invokes the governed kernel boundary, and sanitizes responses.
pub struct ExecutionApiFacade<K: KernelInvoker> {
    kernel: K,
}

impl<K: KernelInvoker> ExecutionApiFacade<K> {
    pub fn new(kernel: K) -> Self {
        Self { kernel }
    }

    pub fn execute(
        &self,
        request: GovernedExecutionRequest,
    ) -> Result<GovernedExecutionResponse, ExecutionApiError> {
        if request.authorization_receipt_id.trim().is_empty() {
            return Err(ExecutionApiError::InvalidReceipt(
                "Authorization receipt ID is missing".into(),
            ));
        }

        if request.operation_payload.is_empty() {
            return Err(ExecutionApiError::SerializationFault(
                "Operation payload cannot be empty".into(),
            ));
        }

        let execution_id = request.execution_id;

        match self.kernel.invoke_kernel(KernelExecutionRequest {
            authorization_receipt_id: request.authorization_receipt_id,
            operation_payload: request.operation_payload,
        }) {
            Ok(response) => Ok(GovernedExecutionResponse {
                execution_id,
                status: ExecutionStatus::AuthorizedAndExecuted,
                report_reference: Some(response.report_reference),
            }),

            Err(_error) => Ok(GovernedExecutionResponse {
                execution_id,
                status: ExecutionStatus::ExecutionFailed,
                report_reference: None,
            }),
        }
    }
}