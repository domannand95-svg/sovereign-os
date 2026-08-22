pub mod file_adapter;
pub use file_adapter::FileCreationAdapter;

use sovereign_audit::authorization_receipt::{
    AuthorizationReceipt,
    ReceiptAuthenticationResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCreationOperation {
    pub path: String,
    pub content_hash: [u8; 32],
}
impl FileCreationOperation {
    pub fn matches(&self, other: &FileCreationOperation) -> bool {
        self == other
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionResult {
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    Unauthenticated,
    OperationMismatch,
    ExecutionRejected,
}

pub trait GovernedExecutor {
    fn execute(
        &self,
        receipt: &AuthorizationReceipt,
        authentication: ReceiptAuthenticationResult,
        governed_operation: &FileCreationOperation,
        requested_operation: &FileCreationOperation,
    ) -> Result<ExecutionResult, ExecutionError>;
}