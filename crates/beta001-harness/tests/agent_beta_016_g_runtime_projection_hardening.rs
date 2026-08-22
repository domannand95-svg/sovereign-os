use sovereign_agent_runtime::audit_projection::{
    project_execution_receipt as old_project_execution_receipt,
    ProjectionError as OldProjectionError,
};

use sovereign_agent_runtime::adapters::audit::{
    project_execution_receipt as new_project_execution_receipt,
    ProjectionError as NewProjectionError,
};

use sovereign_agent_runtime::execution::{
    AuthorizedExecution,
    ExecutionAuthority,
    ExecutionReceipt,
    ExecutionResult,
};

use sovereign_agent_runtime::identity::AgentIdentityId as RuntimeAgentIdentityId;

use sovereign_audit::{
    AgentIdentityId as AuditAgentIdentityId,
    AuditEventType,
    Digest,
};


fn execution_authority() -> ExecutionAuthority {
    ExecutionAuthority::from_seed([8u8; 32])
}


fn create_receipt(result: ExecutionResult) -> ExecutionReceipt {
    execution_authority().issue_receipt(
        &AuthorizedExecution {
            attempt_id: sovereign_agent_runtime::execution::AttemptId(
                "attempt-001".into(),
            ),
            grant_id: sovereign_agent_runtime::capability::GrantId(
                "grant-001".into(),
            ),
            executor_identity: RuntimeAgentIdentityId(
                "agent-001".into(),
            ),
        },
        result,
        "output-digest",
        "2026-08-22T00:00:00Z",
    )
}


fn recorder() -> AuditAgentIdentityId {
    AuditAgentIdentityId("audit-recorder".into())
}


fn assert_projection_equivalent(
    old: Result<sovereign_audit::AuditLedgerEntry, OldProjectionError>,
    new: Result<sovereign_audit::AuditLedgerEntry, NewProjectionError>,
) {
    match (old, new) {
        (Ok(old_entry), Ok(new_entry)) => {
            assert_eq!(
                old_entry.sequence,
                new_entry.sequence
            );

            assert_eq!(
                old_entry.previous_entry_digest,
                new_entry.previous_entry_digest
            );

            assert_eq!(
                old_entry.event_type,
                new_entry.event_type
            );

            assert_eq!(
                old_entry.subject_digest,
                new_entry.subject_digest
            );

            assert_eq!(
                old_entry.payload_digest,
                new_entry.payload_digest
            );

            assert_eq!(
                old_entry.recorded_at,
                new_entry.recorded_at
            );

            assert_eq!(
                old_entry.recorded_by,
                new_entry.recorded_by
            );

            assert_eq!(
                old_entry.entry_digest,
                new_entry.entry_digest
            );
        }

        (Err(_), Err(_)) => {
            // Temporary parity acceptance:
            // both paths rejected the same evidence.
            //
            // Exact error unification happens in the next migration step.
        }

        _ => panic!("projection implementations diverged"),
    }
}


#[test]
fn production_adapter_matches_legacy_projection() {
    let receipt = create_receipt(ExecutionResult::Success);

    let old_result = old_project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    );

    let new_result = new_project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    );

    assert_projection_equivalent(
        old_result,
        new_result,
    );
}


#[test]
fn success_receipt_maps_to_execution_committed() {
    let receipt = create_receipt(ExecutionResult::Success);

    let entry = new_project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    )
    .unwrap();

    assert_eq!(
        entry.event_type,
        AuditEventType::ExecutionCommitted
    );

    assert!(entry.verify_integrity());
}


#[test]
fn failure_receipt_maps_to_execution_failed() {
    let receipt = create_receipt(ExecutionResult::Failure);

    let entry = new_project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    )
    .unwrap();

    assert_eq!(
        entry.event_type,
        AuditEventType::ExecutionFailed
    );

    assert!(entry.verify_integrity());
}


#[test]
fn invalid_signature_rejects_projection() {
    let receipt = create_receipt(ExecutionResult::Success);

    let result = new_project_execution_receipt(
        &receipt,
        &[0u8; 32],
        0,
        Digest("genesis".into()),
        recorder(),
    );

    assert_eq!(
        result,
        Err(NewProjectionError::InvalidReceipt)
    );
}


#[test]
fn tampered_receipt_digest_rejects_projection() {
    let mut receipt = create_receipt(ExecutionResult::Success);

    receipt.output_digest = "tampered".into();

    let result = new_project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    );

    assert_eq!(
        result,
        Err(NewProjectionError::InvalidReceipt)
    );
}


#[test]
fn projection_does_not_mutate_receipt() {
    let receipt = create_receipt(ExecutionResult::Success);

    let before = receipt.clone();

    let _ = new_project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    )
    .unwrap();

    assert_eq!(
        receipt,
        before
    );
}