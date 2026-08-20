use sovereign_agent_runtime::capability::{
    derive_request_id, CapabilityRequest, CapabilityScope, CapabilityType, EvaluationResult,
    GrantStatus, PolicyAuthority, RequestId, RequestStatus,
};
use sovereign_agent_runtime::execution::{
    derive_attempt_id, verify_receipt, AuthorizedExecution, CapabilityRegistry, ExecutionAttempt,
    ExecutionAuthority, ExecutionAuthorizationError, ExecutionReceipt, ExecutionResult, FixedClock,
};
use sovereign_agent_runtime::identity::{
    derive_identity_id, AgentClass, AgentIdentity, AgentIdentityStatus, Digest, PolicyId,
    ReplayTimestamp, SchemaVersion,
};
use sovereign_agent_runtime::replay::{replay, ReplayError, RuntimeEvent};
use sovereign_audit::RecordId;

fn active_identity() -> AgentIdentity {
    let key = Digest("11".repeat(32));
    let policy = PolicyId("policy_engine_v1".into());
    let class = AgentClass::LocalModel;
    AgentIdentity {
        schema_version: SchemaVersion("AGENT_IDENTITY-v1".into()),
        identity_id: derive_identity_id(&key, &class, &policy),
        public_key_digest: key,
        agent_class: class,
        created_at: ReplayTimestamp("2026-08-20T09:00:00Z".into()),
        governing_policy: policy,
        status: AgentIdentityStatus::Active,
    }
}

fn request() -> CapabilityRequest {
    let mut request = CapabilityRequest {
        schema_version: "CAPABILITY_REQUEST-v1".into(),
        request_id: RequestId(String::new()),
        requester_identity: active_identity().identity_id,
        proposal_reference: RecordId::from_bytes([9u8; 32]),
        capability_type: CapabilityType::Write,
        requested_scope: CapabilityScope {
            target_resource: "repo/src/lib.rs".into(),
            target_reference: RecordId::from_bytes([1u8; 32]),
            permissions: vec!["write".into()],
        },
        created_at: "2026-08-20T09:55:00Z".into(),
        status: RequestStatus::Pending,
    };
    request.request_id = derive_request_id(
        &request.requester_identity,
        &request.proposal_reference,
        &request.capability_type,
        &request.requested_scope,
        &request.created_at,
    );
    request
}

fn admitted_registry() -> (
    CapabilityRegistry,
    sovereign_agent_runtime::capability::GrantId,
) {
    let authority = PolicyAuthority::from_seed("policy_engine_v1", [7u8; 32]);
    let request = request();
    let evaluation = authority.evaluate(
        &request,
        EvaluationResult::Approved {
            scope: request.requested_scope.clone(),
            expires_at: "2026-08-20T11:00:00Z".into(),
        },
        "2026-08-20T09:59:00Z",
    );
    let mut registry = CapabilityRegistry::new("policy_engine_v1", authority.verifying_key());
    registry.register_identity(active_identity()).unwrap();
    let grant_id = registry
        .admit(&request, &evaluation, "2026-08-20T10:00:00Z")
        .unwrap();
    (registry, grant_id)
}

fn attempt(grant_id: sovereign_agent_runtime::capability::GrantId) -> ExecutionAttempt {
    let mut attempt = ExecutionAttempt {
        schema_version: "EXECUTION_ATTEMPT-v1".into(),
        attempt_id: sovereign_agent_runtime::execution::AttemptId(String::new()),
        grant_reference: grant_id,
        executor_identity: active_identity().identity_id,
        operation_type: "write".into(),
        target_resource: "repo/src/lib.rs".into(),
        target_reference: RecordId::from_bytes([1u8; 32]),
        created_at: "2026-08-20T10:15:00Z".into(),
    };
    reseal_attempt(&mut attempt);
    attempt
}

fn reseal_attempt(attempt: &mut ExecutionAttempt) {
    attempt.attempt_id = derive_attempt_id(
        &attempt.grant_reference,
        &attempt.executor_identity,
        &attempt.operation_type,
        &attempt.target_resource,
        &attempt.target_reference,
        &attempt.created_at,
    );
}

fn receipt_for(attempt: &ExecutionAttempt) -> ExecutionReceipt {
    execution_authority().issue_receipt(
        &AuthorizedExecution {
            attempt_id: attempt.attempt_id.clone(),
            grant_id: attempt.grant_reference.clone(),
            executor_identity: attempt.executor_identity.clone(),
        },
        ExecutionResult::Success,
        "output_digest",
        "2026-08-20T10:16:00Z",
    )
}

fn execution_authority() -> ExecutionAuthority {
    ExecutionAuthority::from_seed([8u8; 32])
}

fn execution_verifying_key() -> [u8; 32] {
    execution_authority().verifying_key()
}

#[test]
fn o18_e_001_policy_issued_grant_authorizes_once() {
    let (mut registry, grant_id) = admitted_registry();
    let attempt = attempt(grant_id);
    assert!(registry
        .authorize_and_consume(&attempt, &FixedClock("2026-08-20T10:15:00Z".into()))
        .is_ok());
}

#[test]
fn o18_e_002_execution_cannot_expand_scope() {
    let (mut registry, grant_id) = admitted_registry();
    let mut attempt = attempt(grant_id);
    attempt.target_resource = "repo/src/secret.rs".into();
    reseal_attempt(&mut attempt);
    assert_eq!(
        registry.authorize_and_consume(&attempt, &FixedClock("2026-08-20T10:15:00Z".into())),
        Err(ExecutionAuthorizationError::ScopeMismatch)
    );
}

#[test]
fn o18_e_003_target_reference_substitution_is_denied() {
    let (mut registry, grant_id) = admitted_registry();
    let mut attempt = attempt(grant_id);
    attempt.target_reference = RecordId::from_bytes([2u8; 32]);
    reseal_attempt(&mut attempt);
    assert_eq!(
        registry.authorize_and_consume(&attempt, &FixedClock("2026-08-20T10:15:00Z".into())),
        Err(ExecutionAuthorizationError::ScopeMismatch)
    );
}

#[test]
fn o18_e_004_forged_grant_reference_is_unknown() {
    let (mut registry, _) = admitted_registry();
    let forged = sovereign_agent_runtime::capability::GrantId("f".repeat(64));
    let attempt = attempt(forged);
    assert_eq!(
        registry.authorize_and_consume(&attempt, &FixedClock("2026-08-20T10:15:00Z".into())),
        Err(ExecutionAuthorizationError::UnknownGrant)
    );
}

#[test]
fn o18_e_005_expired_grant_fails_closed_against_trusted_clock() {
    let (mut registry, grant_id) = admitted_registry();
    assert_eq!(
        registry.authorize_and_consume(
            &attempt(grant_id),
            &FixedClock("2026-08-20T11:00:00Z".into())
        ),
        Err(ExecutionAuthorizationError::OutsideGrantLifetime)
    );
}

#[test]
fn o18_e_006_revocation_and_replay_are_enforced_by_registry_state() {
    let (mut registry, grant_id) = admitted_registry();
    let revoked_attempt = attempt(grant_id.clone());
    registry.revoke(&grant_id).unwrap();
    assert_eq!(
        registry
            .authorize_and_consume(&revoked_attempt, &FixedClock("2026-08-20T10:15:00Z".into())),
        Err(ExecutionAuthorizationError::InactiveGrant)
    );

    let (mut registry, grant_id) = admitted_registry();
    let attempt = attempt(grant_id);
    let clock = FixedClock("2026-08-20T10:15:00Z".into());
    registry.authorize_and_consume(&attempt, &clock).unwrap();
    assert_eq!(
        registry.authorize_and_consume(&attempt, &clock),
        Err(ExecutionAuthorizationError::AlreadyConsumed)
    );
}

#[test]
fn o18_e_007_receipt_identity_is_verified() {
    let (mut registry, grant_id) = admitted_registry();
    let attempt = attempt(grant_id);
    let authorized = registry
        .authorize_and_consume(&attempt, &FixedClock("2026-08-20T10:15:00Z".into()))
        .unwrap();
    let receipt = execution_authority().issue_receipt(
        &authorized,
        ExecutionResult::Success,
        "output_digest",
        "2026-08-20T10:16:00Z",
    );
    assert!(verify_receipt(&receipt, &execution_verifying_key()));
}

#[test]
fn o18_e_008_receipt_mutation_is_detected() {
    let (mut registry, grant_id) = admitted_registry();
    let attempt = attempt(grant_id);
    let authorized = registry
        .authorize_and_consume(&attempt, &FixedClock("2026-08-20T10:15:00Z".into()))
        .unwrap();
    let mut receipt = execution_authority().issue_receipt(
        &authorized,
        ExecutionResult::Success,
        "original",
        "2026-08-20T10:16:00Z",
    );
    receipt.output_digest = "tampered".into();
    assert!(!verify_receipt(&receipt, &execution_verifying_key()));
}

#[test]
fn o18_e_009_admitted_grant_is_active_but_not_publicly_forgeable_into_registry() {
    let (registry, grant_id) = admitted_registry();
    assert_eq!(
        registry.grant(&grant_id).unwrap().status,
        GrantStatus::Active
    );
}

#[test]
fn o18_e_010_replay_reconstructs_identical_state() {
    let (registry, grant_id) = admitted_registry();
    let grant = registry.grant(&grant_id).unwrap().clone();
    let attempt = attempt(grant_id);
    let events = vec![
        RuntimeEvent::GrantAdmitted {
            grant,
            recorded_at: "2026-08-20T10:00:00Z".into(),
        },
        RuntimeEvent::ExecutionAuthorized {
            attempt: attempt.clone(),
            recorded_at: "2026-08-20T10:15:00Z".into(),
        },
        RuntimeEvent::ReceiptRecorded {
            receipt: receipt_for(&attempt),
            recorded_at: "2026-08-20T10:16:00Z".into(),
        },
    ];
    assert_eq!(
        replay(&events, &execution_verifying_key()),
        replay(&events, &execution_verifying_key())
    );
    assert!(replay(&events, &execution_verifying_key())
        .unwrap()
        .consumed_grants
        .contains(&attempt.grant_reference));
}

#[test]
fn o18_e_011_replay_rejects_revocation_before_execution() {
    let (registry, grant_id) = admitted_registry();
    let grant = registry.grant(&grant_id).unwrap().clone();
    let attempt = attempt(grant_id.clone());
    let events = vec![
        RuntimeEvent::GrantAdmitted {
            grant,
            recorded_at: "2026-08-20T10:00:00Z".into(),
        },
        RuntimeEvent::GrantRevoked {
            grant_id,
            recorded_at: "2026-08-20T10:10:00Z".into(),
        },
        RuntimeEvent::ExecutionAuthorized {
            attempt,
            recorded_at: "2026-08-20T10:15:00Z".into(),
        },
    ];
    assert_eq!(
        replay(&events, &execution_verifying_key()),
        Err(ReplayError::InvalidExecution)
    );
}

#[test]
fn o18_e_012_replay_rejects_duplicate_execution() {
    let (registry, grant_id) = admitted_registry();
    let grant = registry.grant(&grant_id).unwrap().clone();
    let attempt = attempt(grant_id);
    let events = vec![
        RuntimeEvent::GrantAdmitted {
            grant,
            recorded_at: "2026-08-20T10:00:00Z".into(),
        },
        RuntimeEvent::ExecutionAuthorized {
            attempt: attempt.clone(),
            recorded_at: "2026-08-20T10:15:00Z".into(),
        },
        RuntimeEvent::ExecutionAuthorized {
            attempt,
            recorded_at: "2026-08-20T10:15:01Z".into(),
        },
    ];
    assert_eq!(
        replay(&events, &execution_verifying_key()),
        Err(ReplayError::DuplicateExecution)
    );
}

#[test]
fn o18_e_013_replay_rejects_tampered_receipt() {
    let (registry, grant_id) = admitted_registry();
    let grant = registry.grant(&grant_id).unwrap().clone();
    let attempt = attempt(grant_id);
    let mut receipt = receipt_for(&attempt);
    receipt.output_digest = "tampered".into();
    let events = vec![
        RuntimeEvent::GrantAdmitted {
            grant,
            recorded_at: "2026-08-20T10:00:00Z".into(),
        },
        RuntimeEvent::ExecutionAuthorized {
            attempt,
            recorded_at: "2026-08-20T10:15:00Z".into(),
        },
        RuntimeEvent::ReceiptRecorded {
            receipt,
            recorded_at: "2026-08-20T10:16:00Z".into(),
        },
    ];
    assert_eq!(
        replay(&events, &execution_verifying_key()),
        Err(ReplayError::InvalidReceipt)
    );
}

#[test]
fn o18_e_014_identity_revocation_invalidates_existing_grant() {
    let (mut registry, grant_id) = admitted_registry();
    let identity_id = active_identity().identity_id;
    registry
        .set_identity_status(&identity_id, AgentIdentityStatus::Revoked)
        .unwrap();
    assert_eq!(
        registry.authorize_and_consume(
            &attempt(grant_id),
            &FixedClock("2026-08-20T10:15:00Z".into())
        ),
        Err(ExecutionAuthorizationError::IdentityInactive)
    );
}
