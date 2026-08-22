use crate::{
    ExecutionAttempt, ExecutionError, ExecutionOutcome, ExecutionReport, FileCreationOperation,
    GovernedExecutor,
};

use sovereign_audit::authorization_receipt::{AuthorizationReceipt, ReceiptAuthenticationResult};

#[derive(Debug, Clone, Copy)]
pub struct FileCreationAdapter;

impl GovernedExecutor for FileCreationAdapter {
    fn execute(
        &self,
        receipt: &AuthorizationReceipt,
        authentication: ReceiptAuthenticationResult,
        governed_operation: &FileCreationOperation,
        requested_operation: &FileCreationOperation,
        content: &[u8],
    ) -> Result<ExecutionReport, ExecutionError> {
        match authentication {
            ReceiptAuthenticationResult::Invalid => Err(ExecutionError::Unauthenticated),

            ReceiptAuthenticationResult::Valid => {
                if !governed_operation.matches(requested_operation) {
                    return Err(ExecutionError::OperationMismatch);
                }

                if !requested_operation.verify_content(content) {
                    return Err(ExecutionError::ContentIntegrityMismatch);
                }

                std::fs::write(&requested_operation.path, content)
                    .map_err(|_| ExecutionError::FilesystemFailure)?;

                let outcome = ExecutionOutcome::Created;

                Ok(ExecutionReport {
                    attempt: ExecutionAttempt {
                        execution_id: "exec-local-001".into(),
                        receipt_reference: receipt.receipt_reference.clone(),
                        operation_reference: requested_operation.path.clone(),
                        outcome: outcome.clone(),
                        timestamp: 0,
                    },
                    outcome,
                })
            }
        }
    }
}
