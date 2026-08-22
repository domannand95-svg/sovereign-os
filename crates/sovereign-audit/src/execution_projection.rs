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
            ExecutionReceiptStatus::AuthorizedAndExecuted => {
                AuditEventType::ExecutionCommitted
            }
            ExecutionReceiptStatus::AuthenticationFailed
            | ExecutionReceiptStatus::ExecutionFailed => {
                AuditEventType::ExecutionFailed
            }
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