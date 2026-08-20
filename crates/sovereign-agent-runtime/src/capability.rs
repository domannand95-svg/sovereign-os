//! Capability negotiation models for the governed agent runtime.
//!
//! A request expresses intent. Only a policy-approved, scope-preserving grant
//! can authorize a later execution attempt.

use crate::encoding::CanonicalHasher;
use crate::identity::AgentIdentityId;
use chrono::{DateTime, FixedOffset};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sovereign_audit::RecordId;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GrantId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityType {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityScope {
    pub target_resource: String,
    pub target_reference: RecordId,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestStatus {
    Pending,
    Approved,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantStatus {
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRequest {
    pub schema_version: String,
    pub request_id: RequestId,
    pub requester_identity: AgentIdentityId,
    pub proposal_reference: RecordId,
    pub capability_type: CapabilityType,
    pub requested_scope: CapabilityScope,
    pub created_at: String,
    pub status: RequestStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGrant {
    pub schema_version: String,
    pub grant_id: GrantId,
    pub subject_identity: AgentIdentityId,
    pub capability_type: CapabilityType,
    pub scope: CapabilityScope,
    pub request_reference: RequestId,
    pub evaluation_digest: String,
    pub issued_by: String,
    pub issued_at: String,
    pub expires_at: String,
    pub status: GrantStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationResult {
    Approved {
        scope: CapabilityScope,
        expires_at: String,
    },
    Denied {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedEvaluation {
    pub request_reference: RequestId,
    pub policy_id: String,
    pub result: EvaluationResult,
    pub evaluated_at: String,
    pub signature: [u8; 64],
}

pub struct PolicyAuthority {
    policy_id: String,
    signing_key: SigningKey,
}

impl PolicyAuthority {
    pub fn from_seed(policy_id: impl Into<String>, seed: [u8; 32]) -> Self {
        Self {
            policy_id: policy_id.into(),
            signing_key: SigningKey::from_bytes(&seed),
        }
    }

    pub fn verifying_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn evaluate(
        &self,
        request: &CapabilityRequest,
        result: EvaluationResult,
        evaluated_at: impl Into<String>,
    ) -> SignedEvaluation {
        let evaluated_at = evaluated_at.into();
        let digest =
            evaluation_digest(&request.request_id, &self.policy_id, &result, &evaluated_at);
        SignedEvaluation {
            request_reference: request.request_id.clone(),
            policy_id: self.policy_id.clone(),
            result,
            evaluated_at,
            signature: self.signing_key.sign(digest.as_bytes()).to_bytes(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantValidationError {
    InvalidRequest,
    PolicyDenied,
    InactiveGrant,
    IdentityMismatch,
    CapabilityMismatch,
    EvaluationMismatch,
    ScopeExpansion,
    InvalidTimestamp,
    InvalidLifetime,
    Expired,
    InvalidSchema,
    InvalidIdentifier,
    InvalidPolicySignature,
    PolicyMismatch,
}

pub fn derive_request_id(
    requester: &AgentIdentityId,
    proposal: &RecordId,
    capability_type: &CapabilityType,
    scope: &CapabilityScope,
    created_at: &str,
) -> RequestId {
    let mut hasher = CanonicalHasher::new(b"SOVEREIGN_CAPABILITY_REQUEST_ID_V1");
    hasher.field(requester.0.as_bytes());
    hasher.field(proposal.as_bytes());
    hasher.field(capability_tag(capability_type));
    scope_fields(&mut hasher, scope);
    hasher.field(created_at.as_bytes());
    RequestId(hasher.finish())
}

pub fn derive_grant_id(
    request: &CapabilityRequest,
    evaluation: &SignedEvaluation,
    issued_at: &str,
) -> GrantId {
    let mut hasher = CanonicalHasher::new(b"SOVEREIGN_CAPABILITY_GRANT_ID_V1");
    hasher.field(request.request_id.0.as_bytes());
    hasher.field(evaluation_digest_for(evaluation).as_bytes());
    hasher.field(issued_at.as_bytes());
    GrantId(hasher.finish())
}

pub fn verify_evaluation(
    request: &CapabilityRequest,
    evaluation: &SignedEvaluation,
    verifying_key: &[u8; 32],
) -> Result<(), GrantValidationError> {
    if evaluation.request_reference != request.request_id {
        return Err(GrantValidationError::EvaluationMismatch);
    }
    let key = VerifyingKey::from_bytes(verifying_key)
        .map_err(|_| GrantValidationError::InvalidPolicySignature)?;
    let signature = Signature::from_bytes(&evaluation.signature);
    key.verify(evaluation_digest_for(evaluation).as_bytes(), &signature)
        .map_err(|_| GrantValidationError::InvalidPolicySignature)
}

pub fn issue_grant(
    request: &CapabilityRequest,
    evaluation: &SignedEvaluation,
    verifying_key: &[u8; 32],
    issued_at: &str,
) -> Result<CapabilityGrant, GrantValidationError> {
    if !validate_request(request) {
        return Err(GrantValidationError::InvalidRequest);
    }
    verify_evaluation(request, evaluation, verifying_key)?;
    let EvaluationResult::Approved { scope, expires_at } = &evaluation.result else {
        return Err(GrantValidationError::PolicyDenied);
    };
    let requested_at = parse_timestamp(&request.created_at)?;
    let evaluated_at = parse_timestamp(&evaluation.evaluated_at)?;
    let issued = parse_timestamp(issued_at)?;
    let expires = parse_timestamp(expires_at)?;
    if requested_at > evaluated_at || evaluated_at > issued || issued >= expires {
        return Err(GrantValidationError::InvalidLifetime);
    }
    let grant = CapabilityGrant {
        schema_version: "CAPABILITY_GRANT-v1".into(),
        grant_id: derive_grant_id(request, evaluation, issued_at),
        subject_identity: request.requester_identity.clone(),
        capability_type: request.capability_type.clone(),
        scope: scope.clone(),
        request_reference: request.request_id.clone(),
        evaluation_digest: evaluation_digest_for(evaluation),
        issued_by: evaluation.policy_id.clone(),
        issued_at: issued_at.into(),
        expires_at: expires_at.clone(),
        status: GrantStatus::Active,
    };
    validate_grant(request, &evaluation.result, &grant, issued_at)?;
    Ok(grant)
}

pub fn validate_request(request: &CapabilityRequest) -> bool {
    let permissions: HashSet<_> = request.requested_scope.permissions.iter().collect();
    request.schema_version == "CAPABILITY_REQUEST-v1"
        && request.status == RequestStatus::Pending
        && !request.request_id.0.is_empty()
        && !request.requester_identity.0.is_empty()
        && !request.requested_scope.target_resource.is_empty()
        && !request.requested_scope.permissions.is_empty()
        && permissions.len() == request.requested_scope.permissions.len()
        && request
            .requested_scope
            .permissions
            .iter()
            .all(|permission| {
                permission == std::str::from_utf8(capability_tag(&request.capability_type)).unwrap()
            })
        && parse_timestamp(&request.created_at).is_ok()
        && request.request_id
            == derive_request_id(
                &request.requester_identity,
                &request.proposal_reference,
                &request.capability_type,
                &request.requested_scope,
                &request.created_at,
            )
}

pub fn validate_grant(
    request: &CapabilityRequest,
    evaluation: &EvaluationResult,
    grant: &CapabilityGrant,
    now: &str,
) -> Result<(), GrantValidationError> {
    if !validate_request(request) {
        return Err(GrantValidationError::InvalidRequest);
    }

    let EvaluationResult::Approved { scope, expires_at } = evaluation else {
        return Err(GrantValidationError::PolicyDenied);
    };

    if grant.status != GrantStatus::Active {
        return Err(GrantValidationError::InactiveGrant);
    }
    if grant.schema_version != "CAPABILITY_GRANT-v1"
        || grant.request_reference != request.request_id
        || grant.grant_id.0.len() != 64
    {
        return Err(GrantValidationError::InvalidIdentifier);
    }
    if grant.subject_identity != request.requester_identity {
        return Err(GrantValidationError::IdentityMismatch);
    }
    if grant.capability_type != request.capability_type {
        return Err(GrantValidationError::CapabilityMismatch);
    }
    if &grant.scope != scope || &grant.expires_at != expires_at {
        return Err(GrantValidationError::EvaluationMismatch);
    }
    if grant.scope.target_resource != request.requested_scope.target_resource
        || !grant
            .scope
            .permissions
            .iter()
            .all(|permission| request.requested_scope.permissions.contains(permission))
    {
        return Err(GrantValidationError::ScopeExpansion);
    }

    let issued_at = parse_timestamp(&grant.issued_at)?;
    let expires_at = parse_timestamp(&grant.expires_at)?;
    let now = parse_timestamp(now)?;
    if issued_at >= expires_at {
        return Err(GrantValidationError::InvalidLifetime);
    }
    if now < issued_at || now >= expires_at {
        return Err(GrantValidationError::Expired);
    }

    Ok(())
}

fn evaluation_digest_for(evaluation: &SignedEvaluation) -> String {
    evaluation_digest(
        &evaluation.request_reference,
        &evaluation.policy_id,
        &evaluation.result,
        &evaluation.evaluated_at,
    )
}

fn evaluation_digest(
    request_id: &RequestId,
    policy_id: &str,
    result: &EvaluationResult,
    evaluated_at: &str,
) -> String {
    let mut hasher = CanonicalHasher::new(b"SOVEREIGN_POLICY_EVALUATION_V1");
    hasher.field(request_id.0.as_bytes());
    hasher.field(policy_id.as_bytes());
    match result {
        EvaluationResult::Approved { scope, expires_at } => {
            hasher.field(b"Approved");
            scope_fields(&mut hasher, scope);
            hasher.field(expires_at.as_bytes());
        }
        EvaluationResult::Denied { reason } => {
            hasher.field(b"Denied");
            hasher.field(reason.as_bytes());
        }
    }
    hasher.field(evaluated_at.as_bytes());
    hasher.finish()
}

fn capability_tag(capability_type: &CapabilityType) -> &'static [u8] {
    match capability_type {
        CapabilityType::Read => b"read",
        CapabilityType::Write => b"write",
        CapabilityType::Execute => b"execute",
    }
}

fn scope_fields(hasher: &mut CanonicalHasher, scope: &CapabilityScope) {
    hasher.field(scope.target_resource.as_bytes());
    hasher.field(scope.target_reference.as_bytes());
    hasher.field(&(scope.permissions.len() as u32).to_be_bytes());
    for permission in &scope.permissions {
        hasher.field(permission.as_bytes());
    }
}

pub(crate) fn parse_timestamp(
    timestamp: &str,
) -> Result<DateTime<FixedOffset>, GrantValidationError> {
    DateTime::parse_from_rfc3339(timestamp).map_err(|_| GrantValidationError::InvalidTimestamp)
}
