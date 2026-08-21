use sovereign_audit::{
    AgentIdentityId as AuditAgentIdentityId, AuditEventType, AuditLedgerEntry, Digest,
};

use crate::execution::{verify_receipt, ExecutionReceipt};

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidReceipt,
}

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
