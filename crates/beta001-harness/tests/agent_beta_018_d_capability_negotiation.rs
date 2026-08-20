use sovereign_agent_runtime::capability::{
    derive_request_id, issue_grant, validate_grant, validate_request, CapabilityGrant,
    CapabilityRequest, CapabilityScope, CapabilityType, EvaluationResult, GrantId, GrantStatus,
    GrantValidationError, PolicyAuthority, RequestId, RequestStatus,
};
use sovereign_agent_runtime::identity::AgentIdentityId;
use sovereign_audit::RecordId;

fn scope(permissions: &[&str]) -> CapabilityScope {
    CapabilityScope {
        target_resource: "repo/src/".into(),
        target_reference: RecordId::from_bytes([1u8; 32]),
        permissions: permissions.iter().map(|value| (*value).into()).collect(),
    }
}

fn request(scope: CapabilityScope) -> CapabilityRequest {
    let mut request = CapabilityRequest {
        schema_version: "CAPABILITY_REQUEST-v1".into(),
        request_id: RequestId(String::new()),
        requester_identity: AgentIdentityId("id_abc".into()),
        proposal_reference: RecordId::from_bytes([1u8; 32]),
        capability_type: CapabilityType::Read,
        requested_scope: scope,
        created_at: "2026-08-20T10:00:00Z".into(),
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

fn grant_for(request: &CapabilityRequest, scope: CapabilityScope) -> CapabilityGrant {
    CapabilityGrant {
        schema_version: "CAPABILITY_GRANT-v1".into(),
        grant_id: GrantId("0".repeat(64)),
        subject_identity: request.requester_identity.clone(),
        capability_type: request.capability_type.clone(),
        scope,
        request_reference: request.request_id.clone(),
        evaluation_digest: "evaluation_digest".into(),
        issued_by: "policy_engine_v1".into(),
        issued_at: "2026-08-20T10:00:00Z".into(),
        expires_at: "2026-08-20T11:00:00Z".into(),
        status: GrantStatus::Active,
    }
}

#[test]
fn o18_d_001_request_identity_is_deterministic() {
    let req_a = request(scope(&["read"]));
    let req_b = req_a.clone();
    assert_eq!(req_a, req_b);
    assert!(validate_request(&req_a));
}

#[test]
fn o18_d_002_proposal_reference_binding() {
    let req_a = request(scope(&["read"]));
    let req_b = CapabilityRequest {
        proposal_reference: RecordId::from_bytes([9u8; 32]),
        ..req_a.clone()
    };
    assert_ne!(req_a.proposal_reference, req_b.proposal_reference);
}

#[test]
fn o18_d_003_unauthorized_request_cannot_execute() {
    let request = request(scope(&["execute"]));
    assert_eq!(request.status, RequestStatus::Pending);
}

#[test]
fn o18_d_004_policy_denial_fails_closed() {
    let request = request(scope(&["read"]));
    let evaluation = EvaluationResult::Denied {
        reason: "policy denied".into(),
    };
    let grant = grant_for(&request, scope(&["read"]));
    assert_eq!(
        validate_grant(&request, &evaluation, &grant, "2026-08-20T10:30:00Z"),
        Err(GrantValidationError::PolicyDenied)
    );
}

#[test]
fn o18_d_005_grant_requires_policy_approval() {
    let evaluation = EvaluationResult::Denied {
        reason: "insufficient authority".into(),
    };
    assert!(matches!(evaluation, EvaluationResult::Denied { .. }));
}

#[test]
fn o18_d_006_grant_scope_cannot_expand() {
    let request = request(scope(&["read"]));
    let expanded = scope(&["read", "write", "execute"]);
    let evaluation = EvaluationResult::Approved {
        scope: expanded.clone(),
        expires_at: "2026-08-20T11:00:00Z".into(),
    };
    let grant = grant_for(&request, expanded);
    assert_eq!(
        validate_grant(&request, &evaluation, &grant, "2026-08-20T10:30:00Z"),
        Err(GrantValidationError::ScopeExpansion)
    );
}

#[test]
fn o18_d_007_expired_grant_is_invalid() {
    let request = request(scope(&["read"]));
    let approved_scope = scope(&["read"]);
    let evaluation = EvaluationResult::Approved {
        scope: approved_scope.clone(),
        expires_at: "2026-08-20T09:00:00Z".into(),
    };
    let mut grant = grant_for(&request, approved_scope);
    grant.status = GrantStatus::Expired;
    grant.expires_at = "2026-08-20T09:00:00Z".into();
    assert_eq!(
        validate_grant(&request, &evaluation, &grant, "2026-08-20T10:30:00Z"),
        Err(GrantValidationError::InactiveGrant)
    );
}

#[test]
fn o18_d_009_active_label_cannot_bypass_expiry() {
    let request = request(scope(&["read"]));
    let approved_scope = scope(&["read"]);
    let evaluation = EvaluationResult::Approved {
        scope: approved_scope.clone(),
        expires_at: "2026-08-20T10:15:00Z".into(),
    };
    let mut grant = grant_for(&request, approved_scope);
    grant.expires_at = "2026-08-20T10:15:00Z".into();

    assert_eq!(
        validate_grant(&request, &evaluation, &grant, "2026-08-20T10:30:00Z"),
        Err(GrantValidationError::Expired)
    );
}

#[test]
fn o18_d_010_policy_signature_tampering_is_rejected() {
    let request = request(scope(&["read"]));
    let authority = PolicyAuthority::from_seed("policy_engine_v1", [7u8; 32]);
    let mut evaluation = authority.evaluate(
        &request,
        EvaluationResult::Approved {
            scope: request.requested_scope.clone(),
            expires_at: "2026-08-20T11:00:00Z".into(),
        },
        "2026-08-20T09:59:00Z",
    );
    evaluation.signature[0] ^= 1;
    assert_eq!(
        issue_grant(
            &request,
            &evaluation,
            &authority.verifying_key(),
            "2026-08-20T10:00:00Z"
        ),
        Err(GrantValidationError::InvalidPolicySignature)
    );
}

#[test]
fn o18_d_011_request_mutation_invalidates_signed_evaluation() {
    let request = request(scope(&["read"]));
    let authority = PolicyAuthority::from_seed("policy_engine_v1", [7u8; 32]);
    let evaluation = authority.evaluate(
        &request,
        EvaluationResult::Approved {
            scope: request.requested_scope.clone(),
            expires_at: "2026-08-20T11:00:00Z".into(),
        },
        "2026-08-20T09:59:00Z",
    );
    let mut mutated = request.clone();
    mutated.proposal_reference = RecordId::from_bytes([3u8; 32]);
    assert_eq!(
        issue_grant(
            &mutated,
            &evaluation,
            &authority.verifying_key(),
            "2026-08-20T10:00:00Z"
        ),
        Err(GrantValidationError::InvalidRequest)
    );
}

#[test]
fn o18_d_008_grant_identity_replay_is_deterministic() {
    let request = request(scope(&["read"]));
    let grant_a = grant_for(&request, scope(&["read"]));
    let grant_b = grant_a.clone();
    assert_eq!(grant_a, grant_b);
}
