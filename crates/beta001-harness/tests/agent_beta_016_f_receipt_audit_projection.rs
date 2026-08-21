use sovereign_agent_runtime::execution::{
    verify_receipt, AuthorizedExecution, ExecutionAuthority, ExecutionReceipt, ExecutionResult,
};
use sovereign_agent_runtime::identity::AgentIdentityId as RuntimeAgentIdentityId;

use sovereign_audit::{
    AgentIdentityId as AuditAgentIdentityId, AuditEventType, AuditLedgerChain, AuditLedgerEntry,
    Digest,
};

fn execution_authority() -> ExecutionAuthority {
    ExecutionAuthority::from_seed([8u8; 32])
}

fn create_receipt() -> ExecutionReceipt {
    let authority = execution_authority();

    authority.issue_receipt(
        &AuthorizedExecution {
            attempt_id: sovereign_agent_runtime::execution::AttemptId("attempt-001".into()),
            grant_id: sovereign_agent_runtime::capability::GrantId("grant-001".into()),
            executor_identity: RuntimeAgentIdentityId("agent-001".into()),
        },
        ExecutionResult::Success,
        "output-digest",
        "2026-08-22T00:00:00Z",
    )
}

fn project_receipt_to_entry(receipt: &ExecutionReceipt) -> AuditLedgerEntry {
    assert!(verify_receipt(
        receipt,
        &execution_authority().verifying_key()
    ));

    AuditLedgerEntry::new(
        0,
        Digest("genesis".into()),
        AuditEventType::ExecutionCommitted,
        Digest(receipt.receipt_id.0.clone()),
        Digest(receipt.output_digest.clone()),
        receipt.completed_at.clone(),
        AuditAgentIdentityId("audit-recorder".into()),
    )
}

#[test]
fn receipt_projects_to_audit_ledger_entry() {
    let receipt = create_receipt();

    let entry = project_receipt_to_entry(&receipt);

    assert_eq!(entry.event_type, AuditEventType::ExecutionCommitted);

    assert_eq!(entry.subject_digest, Digest(receipt.receipt_id.0.clone()));

    assert_eq!(entry.payload_digest, Digest(receipt.output_digest.clone()));

    assert!(entry.verify_integrity());
}

#[test]
fn tampered_receipt_is_rejected_before_projection() {
    let mut receipt = create_receipt();

    receipt.output_digest = "tampered".into();

    assert!(!verify_receipt(
        &receipt,
        &execution_authority().verifying_key()
    ));
}

#[test]
fn projection_does_not_mutate_receipt() {
    let receipt = create_receipt();
    let before = receipt.clone();

    let _entry = project_receipt_to_entry(&receipt);

    assert_eq!(receipt, before);
}

#[test]
fn projected_entry_can_enter_audit_chain() {
    let receipt = create_receipt();

    let entry = project_receipt_to_entry(&receipt);

    let mut chain = AuditLedgerChain::new();

    assert!(chain.append(entry).is_ok());
    assert!(chain.verify_chain().is_ok());
}
#[test]
fn invalid_sequence_is_rejected_by_chain() {
    let receipt = create_receipt();

    let entry = AuditLedgerEntry::new(
        1,
        Digest("genesis".into()),
        AuditEventType::ExecutionCommitted,
        Digest(receipt.receipt_id.0.clone()),
        Digest(receipt.output_digest.clone()),
        receipt.completed_at.clone(),
        AuditAgentIdentityId("audit-recorder".into()),
    );

    let mut chain = AuditLedgerChain::new();

    assert!(chain.append(entry).is_err());
}

#[test]
fn wrong_previous_digest_is_rejected_by_chain() {
    let receipt = create_receipt();

    let entry = AuditLedgerEntry::new(
        0,
        Digest("wrong-predecessor".into()),
        AuditEventType::ExecutionCommitted,
        Digest(receipt.receipt_id.0.clone()),
        Digest(receipt.output_digest.clone()),
        receipt.completed_at.clone(),
        AuditAgentIdentityId("audit-recorder".into()),
    );

    let mut chain = AuditLedgerChain::new();

    assert!(chain.append(entry).is_ok());

    let mut second = AuditLedgerEntry::new(
        1,
        Digest("wrong-predecessor".into()),
        AuditEventType::ExecutionCommitted,
        Digest("second".into()),
        Digest("payload".into()),
        "2026-08-22T00:01:00Z".into(),
        AuditAgentIdentityId("audit-recorder".into()),
    );

    assert!(second.verify_integrity());

    assert!(chain.append(second).is_err());
}

#[test]
fn different_sequence_changes_entry_digest() {
    let receipt = create_receipt();

    let first = AuditLedgerEntry::new(
        0,
        Digest("genesis".into()),
        AuditEventType::ExecutionCommitted,
        Digest(receipt.receipt_id.0.clone()),
        Digest(receipt.output_digest.clone()),
        receipt.completed_at.clone(),
        AuditAgentIdentityId("audit-recorder".into()),
    );

    let second = AuditLedgerEntry::new(
        1,
        Digest("genesis".into()),
        AuditEventType::ExecutionCommitted,
        Digest(receipt.receipt_id.0.clone()),
        Digest(receipt.output_digest.clone()),
        receipt.completed_at.clone(),
        AuditAgentIdentityId("audit-recorder".into()),
    );

    assert_ne!(first.entry_digest, second.entry_digest);
}
