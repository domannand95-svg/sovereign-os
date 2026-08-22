//! Runtime execution receipt projection boundary.
//!
//! Converts verified `ExecutionReceipt` evidence into generic
//! `sovereign-audit` `AuditLedgerEntry` records.
//!
//! This module:
//! - verifies receipt authenticity
//! - translates runtime evidence into audit evidence
//! - maps execution outcomes to audit events
//! - constructs immutable audit ledger entries
//!
//! This module does not:
//! - execute operations
//! - grant capabilities
//! - append ledger entries
//! - persist audit history
//! - mutate execution receipts

use sovereign_audit::{
    AgentIdentityId as AuditAgentIdentityId, AuditEventType, AuditLedgerEntry, Digest,
};

use crate::execution::{verify_receipt, ExecutionReceipt};

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidReceipt,
}

/// Projects a verified runtime execution receipt into an audit ledger entry.
///
/// The projection boundary:
///
/// `ExecutionReceipt`
///     -> receipt verification
///     -> evidence extraction
///     -> `AuditLedgerEntry`
///
/// This function performs no ledger mutation. The caller remains responsible
/// for explicitly appending the returned entry to an audit chain.
pub fn project_execution_receipt(
    receipt: &ExecutionReceipt,
    verifying_key: &[u8; 32],
    sequence: u64,
    previous_entry_digest: Digest,
    recorded_by: AuditAgentIdentityId,
) -> Result<AuditLedgerEntry, ProjectionError> {
    if !verify_receipt(receipt, verifying_key) {
        return Err(ProjectionError::InvalidReceipt);
    }

    let event_type = match receipt.result {
        crate::execution::ExecutionResult::Success => AuditEventType::ExecutionCommitted,
        crate::execution::ExecutionResult::Failure => AuditEventType::ExecutionFailed,
    };

    Ok(AuditLedgerEntry::new(
        sequence,
        previous_entry_digest,
        event_type,
        Digest(receipt.receipt_id.0.clone()),
        Digest(receipt.output_digest.clone()),
        receipt.completed_at.clone(),
        recorded_by,
    ))
}
