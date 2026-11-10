//! Lifecycle boundaries of the in-memory capability registry and its replay.
//!
//! Contracts these tests hold the registry to:
//! - EXP-BETA-016 INVARIANT-068: expired authority cannot be revived by repetition.
//! - EXP-BETA-016 INVARIANT-069: revocation dominates previous approval.
//! - ADAM-010 T010-004: approval evidence is single-use.
//! - CAPABILITY_LIFECYCLE_EVENT-v1: a suspended state returns to active only through
//!   an explicit activation event. By analogy, a suspended identity is reactivated
//!   only through an explicit status change, never by registering it again.

use sovereign_agent_runtime::capability::{
    derive_request_id, CapabilityGrant, CapabilityRequest, CapabilityScope, CapabilityType,
    EvaluationResult, GrantId, GrantStatus, PolicyAuthority, RequestId, RequestStatus,
    SignedEvaluation,
};
use sovereign_agent_runtime::execution::{
    derive_attempt_id, AttemptId, CapabilityRegistry, ExecutionAttempt,
    ExecutionAuthorizationError, FixedClock,
};
use sovereign_agent_runtime::identity::{
    derive_identity_id, AgentClass, AgentIdentity, AgentIdentityStatus, Digest,
    IdentityValidationError, PolicyId, ReplayTimestamp, SchemaVersion,
};
use sovereign_agent_runtime::replay::{replay, ReplayError, RuntimeEvent};
use sovereign_audit::RecordId;

const T_ADMIT: &str = "2026-08-20T10:00:00Z";
const T_ADMIT_LATER: &str = "2026-08-20T10:00:01Z";
const T_ATTEMPT: &str = "2026-08-20T10:15:00Z";
const T_AFTER_EXPIRY: &str = "2026-08-20T11:00:00Z";

fn authority() -> PolicyAuthority {
    PolicyAuthority::from_seed("policy_engine_v1", [7u8; 32])
}

fn identity_with(status: AgentIdentityStatus) -> AgentIdentity {
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
        status,
    }
}

fn identity() -> AgentIdentity {
    identity_with(AgentIdentityStatus::Active)
}

fn request_created_at(created_at: &str) -> CapabilityRequest {
    let mut request = CapabilityRequest {
        schema_version: "CAPABILITY_REQUEST-v1".into(),
        request_id: RequestId(String::new()),
        requester_identity: identity().identity_id,
        proposal_reference: RecordId::from_bytes([9u8; 32]),
        capability_type: CapabilityType::Write,
        requested_scope: CapabilityScope {
            target_resource: "repo/src/lib.rs".into(),
            target_reference: RecordId::from_bytes([1u8; 32]),
            permissions: vec!["write".into()],
        },
        created_at: created_at.into(),
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

fn request() -> CapabilityRequest {
    request_created_at("2026-08-20T09:55:00Z")
}

fn approval_at(request: &CapabilityRequest, evaluated_at: &str) -> SignedEvaluation {
    authority().evaluate(
        request,
        EvaluationResult::Approved {
            scope: request.requested_scope.clone(),
            expires_at: T_AFTER_EXPIRY.into(),
        },
        evaluated_at,
    )
}

fn approval(request: &CapabilityRequest) -> SignedEvaluation {
    approval_at(request, "2026-08-20T09:59:00Z")
}

fn registry() -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new("policy_engine_v1", authority().verifying_key());
    registry.register_identity(identity()).unwrap();
    registry
}

fn admitted() -> (CapabilityRegistry, GrantId) {
    let mut registry = registry();
    let request = request();
    let grant_id = registry
        .admit(&request, &approval(&request), T_ADMIT)
        .unwrap();
    (registry, grant_id)
}

fn attempt(grant_id: &GrantId) -> ExecutionAttempt {
    let mut attempt = ExecutionAttempt {
        schema_version: "EXECUTION_ATTEMPT-v1".into(),
        attempt_id: AttemptId(String::new()),
        grant_reference: grant_id.clone(),
        executor_identity: identity().identity_id,
        operation_type: "write".into(),
        target_resource: "repo/src/lib.rs".into(),
        target_reference: RecordId::from_bytes([1u8; 32]),
        created_at: T_ATTEMPT.into(),
    };
    attempt.attempt_id = derive_attempt_id(
        &attempt.grant_reference,
        &attempt.executor_identity,
        &attempt.operation_type,
        &attempt.target_resource,
        &attempt.target_reference,
        &attempt.created_at,
    );
    attempt
}

fn clock() -> FixedClock {
    FixedClock(T_ATTEMPT.into())
}

const DUPLICATE: Result<GrantId, ExecutionAuthorizationError> =
    Err(ExecutionAuthorizationError::DuplicateAdmission);

// --- Identity registration ----------------------------------------------------

#[test]
fn o18_f_001_re_registration_cannot_reactivate_a_suspended_identity() {
    let (mut registry, grant_id) = admitted();
    registry
        .set_identity_status(&identity().identity_id, AgentIdentityStatus::Suspended)
        .unwrap();

    assert_eq!(
        registry.register_identity(identity()),
        Err(ExecutionAuthorizationError::Identity(
            IdentityValidationError::AlreadyRegistered
        ))
    );
    assert_eq!(
        registry.authorize_and_consume(&attempt(&grant_id), &clock()),
        Err(ExecutionAuthorizationError::IdentityInactive)
    );
}

#[test]
fn o18_f_002_re_registration_cannot_reactivate_a_revoked_identity() {
    let (mut registry, grant_id) = admitted();
    registry
        .set_identity_status(&identity().identity_id, AgentIdentityStatus::Revoked)
        .unwrap();

    assert_eq!(
        registry.register_identity(identity()),
        Err(ExecutionAuthorizationError::Identity(
            IdentityValidationError::AlreadyRegistered
        ))
    );
    assert_eq!(
        registry.authorize_and_consume(&attempt(&grant_id), &clock()),
        Err(ExecutionAuthorizationError::IdentityInactive)
    );
}

#[test]
fn o18_f_003_re_registration_cannot_change_an_active_identity_either() {
    let (mut registry, grant_id) = admitted();

    // Registering the same identity again as suspended must not silently suspend it.
    assert_eq!(
        registry.register_identity(identity_with(AgentIdentityStatus::Suspended)),
        Err(ExecutionAuthorizationError::Identity(
            IdentityValidationError::AlreadyRegistered
        ))
    );
    assert!(registry
        .authorize_and_consume(&attempt(&grant_id), &clock())
        .is_ok());
}

#[test]
fn o18_f_004_explicit_status_change_is_the_intended_way_to_reactivate() {
    // Intended boundary, not a defect: suspension is reversible through an explicit
    // status change, matching the ACTIVATED transition of the lifecycle schema.
    let (mut registry, grant_id) = admitted();
    let id = identity().identity_id;
    registry
        .set_identity_status(&id, AgentIdentityStatus::Suspended)
        .unwrap();
    assert_eq!(
        registry.authorize_and_consume(&attempt(&grant_id), &clock()),
        Err(ExecutionAuthorizationError::IdentityInactive)
    );

    registry
        .set_identity_status(&id, AgentIdentityStatus::Active)
        .unwrap();
    assert!(registry
        .authorize_and_consume(&attempt(&grant_id), &clock())
        .is_ok());
}

// --- Grant admission ----------------------------------------------------------

#[test]
fn o18_f_005_re_admission_cannot_revive_a_revoked_grant() {
    let (mut registry, grant_id) = admitted();
    let request = request();
    registry.revoke(&grant_id).unwrap();

    assert_eq!(
        registry.admit(&request, &approval(&request), T_ADMIT),
        DUPLICATE
    );
    assert_eq!(
        registry.grant(&grant_id).unwrap().status,
        GrantStatus::Revoked
    );
    assert_eq!(
        registry.authorize_and_consume(&attempt(&grant_id), &clock()),
        Err(ExecutionAuthorizationError::InactiveGrant)
    );
}

#[test]
fn o18_f_006_one_approved_request_yields_one_grant_whatever_the_issue_time() {
    let (mut registry, first) = admitted();
    let request = request();

    assert_eq!(
        registry.admit(&request, &approval(&request), T_ADMIT_LATER),
        DUPLICATE
    );
    // A fresh signature over the same request does not make it a new approval.
    assert_eq!(
        registry.admit(
            &request,
            &approval_at(&request, "2026-08-20T09:59:30Z"),
            T_ADMIT_LATER
        ),
        DUPLICATE
    );

    // The single grant is usable once, and nothing else was minted.
    assert!(registry
        .authorize_and_consume(&attempt(&first), &clock())
        .is_ok());
    assert_eq!(
        registry.authorize_and_consume(&attempt(&first), &clock()),
        Err(ExecutionAuthorizationError::AlreadyConsumed)
    );
}

#[test]
fn o18_f_007_re_admission_cannot_refresh_a_consumed_or_expired_grant() {
    let (mut registry, grant_id) = admitted();
    let request = request();
    registry
        .authorize_and_consume(&attempt(&grant_id), &clock())
        .unwrap();
    for issued_at in [T_ADMIT, T_ADMIT_LATER] {
        assert_eq!(
            registry.admit(&request, &approval(&request), issued_at),
            DUPLICATE
        );
    }
    assert_eq!(
        registry.authorize_and_consume(&attempt(&grant_id), &clock()),
        Err(ExecutionAuthorizationError::AlreadyConsumed)
    );

    // Unconsumed but past its lifetime: repetition does not revive it.
    let (mut registry, grant_id) = admitted();
    assert_eq!(
        registry.admit(&request, &approval(&request), T_ADMIT_LATER),
        DUPLICATE
    );
    assert_eq!(
        registry.authorize_and_consume(&attempt(&grant_id), &FixedClock(T_AFTER_EXPIRY.into())),
        Err(ExecutionAuthorizationError::OutsideGrantLifetime)
    );
}

#[test]
fn o18_f_008_a_genuinely_new_request_is_still_admitted() {
    // Intended boundary: a new request (different creation time, so a different
    // request identifier) with its own approval is a separate grant.
    let (mut registry, first) = admitted();
    let renewed = request_created_at("2026-08-20T09:56:00Z");
    assert_ne!(renewed.request_id, request().request_id);

    let second = registry
        .admit(&renewed, &approval(&renewed), T_ADMIT_LATER)
        .unwrap();
    assert_ne!(first, second);
}

#[test]
fn o18_f_009_a_refused_admission_does_not_use_up_the_request() {
    let mut registry = registry();
    let request = request();

    let mut forged = approval(&request);
    forged.signature[0] ^= 0x01;
    assert!(matches!(
        registry.admit(&request, &forged, T_ADMIT),
        Err(ExecutionAuthorizationError::GrantAdmission(_))
    ));

    assert!(registry
        .admit(&request, &approval(&request), T_ADMIT)
        .is_ok());
}

// --- Replay -------------------------------------------------------------------

fn grant_issued_at(issued_at: &str) -> CapabilityGrant {
    let request = request();
    let mut registry = registry();
    let grant_id = registry
        .admit(&request, &approval(&request), issued_at)
        .unwrap();
    registry.grant(&grant_id).unwrap().clone()
}

fn admitted_event(grant: CapabilityGrant, recorded_at: &str) -> RuntimeEvent {
    RuntimeEvent::GrantAdmitted {
        grant,
        recorded_at: recorded_at.into(),
    }
}

#[test]
fn o18_f_010_replay_rejects_a_second_grant_for_the_same_request() {
    let first = grant_issued_at(T_ADMIT);
    let second = grant_issued_at(T_ADMIT_LATER);
    assert_ne!(first.grant_id, second.grant_id);
    assert_eq!(first.request_reference, second.request_reference);

    let events = [
        admitted_event(first, T_ADMIT),
        admitted_event(second, T_ADMIT_LATER),
    ];
    assert_eq!(
        replay(&events, &[0u8; 32]),
        Err(ReplayError::DuplicateGrant)
    );
}

#[test]
fn o18_f_011_replay_rejects_re_admission_after_revocation() {
    let grant = grant_issued_at(T_ADMIT);
    let events = [
        admitted_event(grant.clone(), T_ADMIT),
        RuntimeEvent::GrantRevoked {
            grant_id: grant.grant_id.clone(),
            recorded_at: T_ADMIT_LATER.into(),
        },
        admitted_event(grant, T_ADMIT_LATER),
    ];
    assert_eq!(
        replay(&events, &[0u8; 32]),
        Err(ReplayError::DuplicateGrant)
    );
}

#[test]
fn o18_f_012_replay_still_accepts_grants_for_distinct_requests() {
    let first = grant_issued_at(T_ADMIT);
    let renewed = request_created_at("2026-08-20T09:56:00Z");
    let mut registry = registry();
    let second_id = registry
        .admit(&renewed, &approval(&renewed), T_ADMIT_LATER)
        .unwrap();
    let second = registry.grant(&second_id).unwrap().clone();

    let events = [
        admitted_event(first, T_ADMIT),
        admitted_event(second, T_ADMIT_LATER),
    ];
    let state = replay(&events, &[0u8; 32]).unwrap();
    assert_eq!(state.grants.len(), 2);
}
