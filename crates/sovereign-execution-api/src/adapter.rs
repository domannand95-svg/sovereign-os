use crate::{
    ExecutionApiError, ExecutionStatus, GovernedExecutionRequest, GovernedExecutionResponse,
    KernelInvoker,
};

/// The API translation layer.
/// Handles DTO validation, delegates to the kernel invoker, and maps responses.
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

        match self.kernel.invoke_kernel(crate::KernelExecutionRequest {
            authorization_receipt_id: request.authorization_receipt_id,
            operation_payload: request.operation_payload,
        }) {
            Ok(response) => Ok(GovernedExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::AuthorizedAndExecuted,
                report_reference: Some(response.report_reference),
            }),

            Err(error) => Ok(GovernedExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::ExecutionFailed(format!("{error:?}")),
                report_reference: None,
            }),
        }
    }
}