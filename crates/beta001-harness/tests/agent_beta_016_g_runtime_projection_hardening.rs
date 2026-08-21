use sovereign_agent_runtime::audit_projection::{project_execution_receipt, ProjectionError};
use sovereign_agent_runtime::execution::{
    verify_receipt, AuthorizedExecution, ExecutionAuthority, ExecutionReceipt, ExecutionResult,
};
use sovereign_agent_runtime::identity::AgentIdentityId as RuntimeAgentIdentityId;

use sovereign_audit::{AgentIdentityId as AuditAgentIdentityId, AuditEventType, Digest};

fn execution_authority() -> ExecutionAuthority {
    ExecutionAuthority::from_seed([8u8; 32])
}

fn create_receipt(result: ExecutionResult) -> ExecutionReceipt {
    execution_authority().issue_receipt(
        &AuthorizedExecution {
            attempt_id: sovereign_agent_runtime::execution::AttemptId("attempt-001".into()),
            grant_id: sovereign_agent_runtime::capability::GrantId("grant-001".into()),
            executor_identity: RuntimeAgentIdentityId("agent-001".into()),
        },
        result,
        "output-digest",
        "2026-08-22T00:00:00Z",
    )
}

fn recorder() -> AuditAgentIdentityId {
    AuditAgentIdentityId("audit-recorder".into())
}

#[test]
fn success_receipt_maps_to_execution_committed() {
    let receipt = create_receipt(ExecutionResult::Success);

    let entry = project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    )
    .unwrap();

    assert_eq!(entry.event_type, AuditEventType::ExecutionCommitted);

    assert!(entry.verify_integrity());
}

#[test]
fn failure_receipt_maps_to_execution_failed() {
    let receipt = create_receipt(ExecutionResult::Failure);

    let entry = project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    )
    .unwrap();

    assert_eq!(entry.event_type, AuditEventType::ExecutionFailed);

    assert!(entry.verify_integrity());
}

#[test]
fn invalid_signature_rejects_projection() {
    let receipt = create_receipt(ExecutionResult::Success);

    let result = project_execution_receipt(
        &receipt,
        &[0u8; 32],
        0,
        Digest("genesis".into()),
        recorder(),
    );

    assert_eq!(result, Err(ProjectionError::InvalidReceipt));
}

#[test]
fn tampered_receipt_digest_rejects_projection() {
    let mut receipt = create_receipt(ExecutionResult::Success);

    receipt.output_digest = "tampered".into();

    let result = project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    );

    assert_eq!(result, Err(ProjectionError::InvalidReceipt));
}

#[test]
fn projection_does_not_mutate_receipt() {
    let receipt = create_receipt(ExecutionResult::Success);
    let before = receipt.clone();

    let _ = project_execution_receipt(
        &receipt,
        &execution_authority().verifying_key(),
        0,
        Digest("genesis".into()),
        recorder(),
    )
    .unwrap();

    assert_eq!(receipt, before);
}
