//! BETA-027 Execution Projection Adapter
//!
//! Converts execution receipts into audit ledger entries.
//!
//! Invariants:
//! Execution Evidence != Execution Authority
//! Receipt != Permission
//! Projection != Execution

use crate::{
    execution_receipt::{ExecutionReceipt, ExecutionReceiptStatus},
    identity::{AgentIdentityId, Digest},
    ledger::{AuditEventType, AuditLedgerEntry},
};

#[derive(Debug, PartialEq, Eq)]
pub enum ExecutionProjectionError {
    MissingReference,
}

pub struct ExecutionProjectionAdapter;

impl ExecutionProjectionAdapter {
    pub fn project(
        receipt: &ExecutionReceipt,
        sequence: u64,
        previous_entry_digest: Digest,
        recorded_at: String,
        recorded_by: AgentIdentityId,
    ) -> Result<AuditLedgerEntry, ExecutionProjectionError> {
        if receipt.execution_id.is_empty()
            || receipt.receipt_id.0.is_empty()
            || receipt.content_digest.0.is_empty()
        {
            return Err(ExecutionProjectionError::MissingReference);
        }

        let event_type = match receipt.status {
            ExecutionReceiptStatus::AuthorizedAndExecuted => AuditEventType::ExecutionCommitted,
            ExecutionReceiptStatus::AuthenticationFailed
            | ExecutionReceiptStatus::ExecutionFailed => AuditEventType::ExecutionFailed,
        };

        Ok(AuditLedgerEntry::new(
            sequence,
            previous_entry_digest,
            event_type,
            receipt.receipt_id.clone(),
            receipt.content_digest.clone(),
            recorded_at,
            recorded_by,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_receipt::{ErrorCategory, ExecutionReceipt};

    fn digest(value: &str) -> Digest {
        Digest(value.to_string())
    }

    fn identity(value: &str) -> AgentIdentityId {
        AgentIdentityId(value.to_string())
    }

    fn successful_receipt() -> ExecutionReceipt {
        let operation_hash =
            ExecutionReceipt::derive_operation_hash("FILE_WRITE", "/data/test.txt");

        ExecutionReceipt::new(
            "exec-001".to_string(),
            digest("authorization"),
            ExecutionReceiptStatus::AuthorizedAndExecuted,
            digest("content"),
            operation_hash,
            1700000000,
            None,
        )
    }

    fn failed_receipt() -> ExecutionReceipt {
        let operation_hash =
            ExecutionReceipt::derive_operation_hash("FILE_WRITE", "/data/test.txt");

        ExecutionReceipt::new(
            "exec-002".to_string(),
            digest("authorization"),
            ExecutionReceiptStatus::ExecutionFailed,
            digest("content"),
            operation_hash,
            1700000000,
            Some(ErrorCategory::ExecutionFailure),
        )
    }

    #[test]
    fn authorized_receipt_projects_to_execution_committed() {
        let receipt = successful_receipt();

        let entry = ExecutionProjectionAdapter::project(
            &receipt,
            0,
            digest("genesis"),
            "2026-08-23T00:00:00Z".to_string(),
            identity("agent-001"),
        )
        .expect("projection should succeed");

        assert_eq!(entry.event_type, AuditEventType::ExecutionCommitted);

        assert!(entry.verify_integrity());
    }

    #[test]
    fn failed_receipt_projects_to_execution_failed() {
        let receipt = failed_receipt();

        let entry = ExecutionProjectionAdapter::project(
            &receipt,
            0,
            digest("genesis"),
            "2026-08-23T00:00:00Z".to_string(),
            identity("agent-001"),
        )
        .expect("projection should succeed");

        assert_eq!(entry.event_type, AuditEventType::ExecutionFailed);

        assert!(entry.verify_integrity());
    }

    #[test]
    fn missing_execution_reference_is_rejected() {
        let mut receipt = successful_receipt();

        receipt.execution_id = String::new();

        let result = ExecutionProjectionAdapter::project(
            &receipt,
            0,
            digest("genesis"),
            "2026-08-23T00:00:00Z".to_string(),
            identity("agent-001"),
        );

        assert_eq!(result, Err(ExecutionProjectionError::MissingReference));
    }

    #[test]
    fn projection_preserves_authority_boundary() {
        let receipt = successful_receipt();

        let entry = ExecutionProjectionAdapter::project(
            &receipt,
            0,
            digest("genesis"),
            "2026-08-23T00:00:00Z".to_string(),
            identity("agent-001"),
        )
        .expect("projection should succeed");

        assert_eq!(entry.subject_digest, receipt.receipt_id);

        assert_eq!(entry.payload_digest, receipt.content_digest);
    }
}

#[cfg(test)]
mod ledger_integration_tests {
    use super::*;
    use crate::chain::AuditLedgerChain;
    use crate::execution_receipt::{ExecutionReceipt, ExecutionReceiptStatus};

    fn digest(value: &str) -> Digest {
        Digest(value.to_string())
    }

    fn identity(value: &str) -> AgentIdentityId {
        AgentIdentityId(value.to_string())
    }

    fn receipt() -> ExecutionReceipt {
        let operation_hash =
            ExecutionReceipt::derive_operation_hash("FILE_WRITE", "/data/test.txt");

        ExecutionReceipt::new(
            "exec-ledger-001".to_string(),
            digest("authorization"),
            ExecutionReceiptStatus::AuthorizedAndExecuted,
            digest("payload"),
            operation_hash,
            1700000000,
            None,
        )
    }

    #[test]
    fn projected_execution_receipt_appends_to_audit_chain() {
        let receipt = receipt();

        let entry = ExecutionProjectionAdapter::project(
            &receipt,
            0,
            digest("genesis"),
            "2026-08-23T00:00:00Z".to_string(),
            identity("agent-001"),
        )
        .expect("projection succeeds");

        let mut chain = AuditLedgerChain::new();

        chain.append(entry).expect("ledger append succeeds");

        assert_eq!(chain.len(), 1);
        assert!(chain.verify_chain().is_ok());
    }

    #[test]
    fn receipt_identity_is_preserved_in_ledger_subject() {
        let receipt = receipt();

        let entry = ExecutionProjectionAdapter::project(
            &receipt,
            0,
            digest("genesis"),
            "2026-08-23T00:00:00Z".to_string(),
            identity("agent-001"),
        )
        .expect("projection succeeds");

        assert_eq!(entry.subject_digest, receipt.receipt_id);

        assert_eq!(entry.payload_digest, receipt.content_digest);
    }
}
