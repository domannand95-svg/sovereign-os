use crate::{
    ExecutionApiError, ExecutionStatus, GovernedExecutionRequest, GovernedExecutionResponse,
};

/// Defines the dependency injection contract for the API layer to interact with the execution kernel.
/// This prevents the API from directly depending on filesystem or signing logic.
pub trait KernelInvoker {
    fn invoke_kernel(&self, receipt_id: &str, payload: &[u8]) -> Result<String, String>;
}

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
        // 1. Validation Boundary
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

        // 2. Adapter Invocation
        match self.kernel.invoke_kernel(
            &request.authorization_receipt_id,
            &request.operation_payload,
        ) {
            Ok(report_reference) => {
                // 3. Response Translation (Success)
                Ok(GovernedExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::AuthorizedAndExecuted,
                    report_reference: Some(report_reference),
                })
            }
            Err(e) => {
                // 3. Response Translation (Failure)
                Ok(GovernedExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::ExecutionFailed(e),
                    report_reference: None,
                })
            }
        }
    }
}
