pub mod file_adapter;
pub use file_adapter::FileCreationAdapter;

use sovereign_audit::authorization_receipt::{AuthorizationReceipt, ReceiptAuthenticationResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCreationOperation {
    pub path: String,
    pub content_hash: [u8; 32],
}

impl FileCreationOperation {
    pub fn matches(&self, other: &FileCreationOperation) -> bool {
        self == other
    }

    pub fn verify_content(&self, content: &[u8]) -> bool {
        let digest = blake3::hash(content);
        digest.as_bytes() == &self.content_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionResult {
    Created,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Created,
    Rejected(ExecutionError),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionAttempt {
    pub execution_id: String,
    pub receipt_reference: String,
    pub operation_reference: String,
    pub outcome: ExecutionOutcome,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReport {
    pub attempt: ExecutionAttempt,
    pub outcome: ExecutionOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    Unauthenticated,
    OperationMismatch,
    ContentIntegrityMismatch,
    FilesystemFailure,
    ExecutionRejected,
}

pub trait GovernedExecutor {
    fn execute(
        &self,
        receipt: &AuthorizationReceipt,
        authentication: ReceiptAuthenticationResult,
        governed_operation: &FileCreationOperation,
        requested_operation: &FileCreationOperation,
        content: &[u8],
    ) -> Result<ExecutionReport, ExecutionError>;
}
