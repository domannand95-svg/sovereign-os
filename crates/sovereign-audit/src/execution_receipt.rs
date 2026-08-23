//! Execution Receipt — immutable audit evidence record.
//!
//! Invariant:
//! Execution Evidence ≠ Execution Authority

use crate::identity::Digest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionReceipt {
    pub receipt_id: Digest,
    pub execution_id: String,
    pub authorization_receipt_id: Digest,
    pub status: ExecutionReceiptStatus,
    pub content_digest: Digest,
    pub operation_hash: Digest,
    pub executed_at: u64,
    pub error_category: Option<ErrorCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionReceiptStatus {
    AuthorizedAndExecuted,
    AuthenticationFailed,
    ExecutionFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCategory {
    ValidationFailure,
    AuthorizationFailure,
    KernelRejection,
    ExecutionFailure,
    TransportFailure,
}

impl ExecutionReceipt {
    pub fn new(
        execution_id: String,
        authorization_receipt_id: Digest,
        status: ExecutionReceiptStatus,
        content_digest: Digest,
        operation_hash: Digest,
        executed_at: u64,
        error_category: Option<ErrorCategory>,
    ) -> Self {
        let receipt_id = Self::derive_id(
            &execution_id,
            &authorization_receipt_id,
            &status,
            &content_digest,
            &operation_hash,
            executed_at,
            &error_category,
        );

        Self {
            receipt_id,
            execution_id,
            authorization_receipt_id,
            status,
            content_digest,
            operation_hash,
            executed_at,
            error_category,
        }
    }

    pub fn derive_operation_hash(
        operation_type: &str,
        operation_target: &str,
    ) -> Digest {
        let mut hasher = blake3::Hasher::new();

        hasher.update(b"SOV:EXECUTION_OPERATION:V1");
        hasher.update(operation_type.as_bytes());
        hasher.update(operation_target.as_bytes());

        Digest(hex::encode(hasher.finalize().as_bytes()))
    }

    pub fn verify_integrity(&self) -> bool {
        self.receipt_id
            == Self::derive_id(
                &self.execution_id,
                &self.authorization_receipt_id,
                &self.status,
                &self.content_digest,
                &self.operation_hash,
                self.executed_at,
                &self.error_category,
            )
    }

    fn derive_id(
        execution_id: &str,
        authorization_receipt_id: &Digest,
        status: &ExecutionReceiptStatus,
        content_digest: &Digest,
        operation_hash: &Digest,
        executed_at: u64,
        error_category: &Option<ErrorCategory>,
    ) -> Digest {
        let mut hasher = blake3::Hasher::new();

        hasher.update(b"SOV:EXECUTION_RECEIPT:V1");
        hasher.update(execution_id.as_bytes());
        hasher.update(authorization_receipt_id.0.as_bytes());
        hasher.update(content_digest.0.as_bytes());
        hasher.update(operation_hash.0.as_bytes());
        hasher.update(&executed_at.to_be_bytes());

        match status {
            ExecutionReceiptStatus::AuthorizedAndExecuted => {
                hasher.update(b"AuthorizedAndExecuted");
            }
            ExecutionReceiptStatus::AuthenticationFailed => {
                hasher.update(b"AuthenticationFailed");
            }
            ExecutionReceiptStatus::ExecutionFailed => {
                hasher.update(b"ExecutionFailed");
            }
        }

        if let Some(error) = error_category {
            match error {
                ErrorCategory::ValidationFailure => {
                    hasher.update(b"ValidationFailure");
                }
                ErrorCategory::AuthorizationFailure => {
                    hasher.update(b"AuthorizationFailure");
                }
                ErrorCategory::KernelRejection => {
                    hasher.update(b"KernelRejection");
                }
                ErrorCategory::ExecutionFailure => {
                    hasher.update(b"ExecutionFailure");
                }
                ErrorCategory::TransportFailure => {
                    hasher.update(b"TransportFailure");
                }
            }
        }

        Digest(hex::encode(hasher.finalize().as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_identity_is_deterministic() {
        let operation_hash =
            ExecutionReceipt::derive_operation_hash(
                "FILE_WRITE",
                "/data/test.txt",
            );

        let receipt_a = ExecutionReceipt::new(
            "exec-001".into(),
            Digest("auth".repeat(16)),
            ExecutionReceiptStatus::AuthorizedAndExecuted,
            Digest("payload".repeat(8)),
            operation_hash.clone(),
            1000,
            None,
        );

        let receipt_b = ExecutionReceipt::new(
            "exec-001".into(),
            Digest("auth".repeat(16)),
            ExecutionReceiptStatus::AuthorizedAndExecuted,
            Digest("payload".repeat(8)),
            operation_hash,
            1000,
            None,
        );

        assert_eq!(receipt_a.receipt_id, receipt_b.receipt_id);
        assert!(receipt_a.verify_integrity());
    }

    #[test]
    fn receipt_detects_mutation() {
        let mut receipt = ExecutionReceipt::new(
            "exec-002".into(),
            Digest("auth".repeat(16)),
            ExecutionReceiptStatus::ExecutionFailed,
            Digest("payload".repeat(8)),
            Digest("operation".repeat(8)),
            1000,
            Some(ErrorCategory::KernelRejection),
        );

        receipt.executed_at = 2000;

        assert!(!receipt.verify_integrity());
    }
}