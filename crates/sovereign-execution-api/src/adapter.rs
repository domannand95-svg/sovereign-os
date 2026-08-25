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
        let execution_id = request.execution_id().clone();
        let kernel_request = crate::KernelExecutionRequest::new(
            request.authorization_receipt(),
            request.action().clone(),
        );
        match self.kernel.invoke_kernel(kernel_request) {
            Ok(response) => Ok(GovernedExecutionResponse {
                execution_id: execution_id.clone(),
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
