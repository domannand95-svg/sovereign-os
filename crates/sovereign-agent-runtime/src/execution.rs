//! Governed execution attempt and receipt models.

use crate::capability::{
    issue_grant, parse_timestamp, CapabilityGrant, CapabilityRequest, CapabilityType, GrantId,
    GrantStatus, GrantValidationError, SignedEvaluation,
};
use crate::encoding::CanonicalHasher;
use crate::identity::{AgentIdentity, AgentIdentityId, IdentityRegistry, IdentityValidationError};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sovereign_audit::RecordId;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttemptId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionResult {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionAttempt {
    pub schema_version: String,
    pub attempt_id: AttemptId,
    pub grant_reference: GrantId,
    pub executor_identity: AgentIdentityId,
    pub operation_type: String,
    pub target_resource: String,
    pub target_reference: RecordId,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionAuthorizationError {
    InactiveGrant,
    GrantMismatch,
    IdentityMismatch,
    CapabilityMismatch,
    ScopeMismatch,
    InvalidTimestamp,
    OutsideGrantLifetime,
    InvalidAttemptIdentity,
    AlreadyConsumed,
    UnknownGrant,
    GrantAdmission(GrantValidationError),
    Identity(IdentityValidationError),
    IdentityInactive,
}

pub trait TrustedClock {
    fn now(&self) -> String;
}

pub struct SystemClock;

impl TrustedClock for SystemClock {
    fn now(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}

#[derive(Debug, Clone)]
pub struct FixedClock(pub String);

impl TrustedClock for FixedClock {
    fn now(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedExecution {
    pub attempt_id: AttemptId,
    pub grant_id: GrantId,
    pub executor_identity: AgentIdentityId,
}

pub struct CapabilityRegistry {
    policy_id: String,
    policy_verifying_key: [u8; 32],
    grants: HashMap<GrantId, CapabilityGrant>,
    consumed_grants: HashSet<GrantId>,
    consumed_attempts: HashSet<AttemptId>,
    identities: IdentityRegistry,
}

impl CapabilityRegistry {
    pub fn new(policy_id: impl Into<String>, policy_verifying_key: [u8; 32]) -> Self {
        Self {
            policy_id: policy_id.into(),
            policy_verifying_key,
            grants: HashMap::new(),
            consumed_grants: HashSet::new(),
            consumed_attempts: HashSet::new(),
            identities: IdentityRegistry::default(),
        }
    }

    pub fn register_identity(
        &mut self,
        identity: AgentIdentity,
    ) -> Result<(), ExecutionAuthorizationError> {
        self.identities
            .register(identity)
            .map_err(ExecutionAuthorizationError::Identity)
    }

    pub fn set_identity_status(
        &mut self,
        identity_id: &AgentIdentityId,
        status: crate::identity::AgentIdentityStatus,
    ) -> Result<(), ExecutionAuthorizationError> {
        self.identities
            .set_status(identity_id, status)
            .map_err(ExecutionAuthorizationError::Identity)
    }

    pub fn admit(
        &mut self,
        request: &CapabilityRequest,
        evaluation: &SignedEvaluation,
        issued_at: &str,
    ) -> Result<GrantId, ExecutionAuthorizationError> {
        if !self.identities.is_active(&request.requester_identity) {
            return Err(ExecutionAuthorizationError::IdentityInactive);
        }
        if evaluation.policy_id != self.policy_id {
            return Err(ExecutionAuthorizationError::GrantAdmission(
                GrantValidationError::PolicyMismatch,
            ));
        }
        let grant = issue_grant(request, evaluation, &self.policy_verifying_key, issued_at)
            .map_err(ExecutionAuthorizationError::GrantAdmission)?;
        let grant_id = grant.grant_id.clone();
        self.grants.insert(grant_id.clone(), grant);
        Ok(grant_id)
    }

    pub fn revoke(&mut self, grant_id: &GrantId) -> Result<(), ExecutionAuthorizationError> {
        let grant = self
            .grants
            .get_mut(grant_id)
            .ok_or(ExecutionAuthorizationError::UnknownGrant)?;
        grant.status = GrantStatus::Revoked;
        Ok(())
    }

    pub fn authorize_and_consume(
        &mut self,
        attempt: &ExecutionAttempt,
        clock: &impl TrustedClock,
    ) -> Result<AuthorizedExecution, ExecutionAuthorizationError> {
        if !self.identities.is_active(&attempt.executor_identity) {
            return Err(ExecutionAuthorizationError::IdentityInactive);
        }
        let grant = self
            .grants
            .get(&attempt.grant_reference)
            .ok_or(ExecutionAuthorizationError::UnknownGrant)?;
        if self.consumed_grants.contains(&grant.grant_id)
            || self.consumed_attempts.contains(&attempt.attempt_id)
        {
            return Err(ExecutionAuthorizationError::AlreadyConsumed);
        }
        authorize_execution(grant, attempt, &clock.now())?;
        self.consumed_grants.insert(grant.grant_id.clone());
        self.consumed_attempts.insert(attempt.attempt_id.clone());
        Ok(AuthorizedExecution {
            attempt_id: attempt.attempt_id.clone(),
            grant_id: grant.grant_id.clone(),
            executor_identity: attempt.executor_identity.clone(),
        })
    }

    pub fn grant(&self, grant_id: &GrantId) -> Option<&CapabilityGrant> {
        self.grants.get(grant_id)
    }
}

pub fn derive_attempt_id(
    grant_id: &GrantId,
    executor: &AgentIdentityId,
    operation_type: &str,
    target_resource: &str,
    target_reference: &RecordId,
    created_at: &str,
) -> AttemptId {
    let mut hasher = CanonicalHasher::new(b"SOVEREIGN_EXECUTION_ATTEMPT_ID_V1");
    hasher.field(grant_id.0.as_bytes());
    hasher.field(executor.0.as_bytes());
    hasher.field(operation_type.as_bytes());
    hasher.field(target_resource.as_bytes());
    hasher.field(target_reference.as_bytes());
    hasher.field(created_at.as_bytes());
    AttemptId(hasher.finish())
}

pub(crate) fn authorize_execution(
    grant: &CapabilityGrant,
    attempt: &ExecutionAttempt,
    now: &str,
) -> Result<(), ExecutionAuthorizationError> {
    if grant.status != GrantStatus::Active {
        return Err(ExecutionAuthorizationError::InactiveGrant);
    }
    if grant.grant_id != attempt.grant_reference {
        return Err(ExecutionAuthorizationError::GrantMismatch);
    }
    if grant.subject_identity != attempt.executor_identity {
        return Err(ExecutionAuthorizationError::IdentityMismatch);
    }
    if attempt.attempt_id
        != derive_attempt_id(
            &attempt.grant_reference,
            &attempt.executor_identity,
            &attempt.operation_type,
            &attempt.target_resource,
            &attempt.target_reference,
            &attempt.created_at,
        )
    {
        return Err(ExecutionAuthorizationError::InvalidAttemptIdentity);
    }

    let required_permission = match grant.capability_type {
        CapabilityType::Read => "read",
        CapabilityType::Write => "write",
        CapabilityType::Execute => "execute",
    };
    if attempt.operation_type != required_permission {
        return Err(ExecutionAuthorizationError::CapabilityMismatch);
    }
    if attempt.target_resource != grant.scope.target_resource
        || attempt.target_reference != grant.scope.target_reference
        || !grant
            .scope
            .permissions
            .iter()
            .any(|value| value == required_permission)
    {
        return Err(ExecutionAuthorizationError::ScopeMismatch);
    }

    let issued_at = parse_timestamp(&grant.issued_at)
        .map_err(|_| ExecutionAuthorizationError::InvalidTimestamp)?;
    let expires_at = parse_timestamp(&grant.expires_at)
        .map_err(|_| ExecutionAuthorizationError::InvalidTimestamp)?;
    let attempted_at = parse_timestamp(&attempt.created_at)
        .map_err(|_| ExecutionAuthorizationError::InvalidTimestamp)?;
    let now = parse_timestamp(now).map_err(|_| ExecutionAuthorizationError::InvalidTimestamp)?;

    if issued_at >= expires_at
        || attempted_at < issued_at
        || attempted_at >= expires_at
        || now < attempted_at
        || now >= expires_at
    {
        return Err(ExecutionAuthorizationError::OutsideGrantLifetime);
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReceipt {
    pub schema_version: String,
    pub receipt_id: ReceiptId,
    pub attempt_reference: AttemptId,
    pub grant_reference: GrantId,
    pub executor_identity: AgentIdentityId,
    pub result: ExecutionResult,
    pub output_digest: String,
    pub completed_at: String,
    pub receipt_digest: String,
    pub signature: [u8; 64],
}

impl ExecutionReceipt {
    pub fn derive_receipt_digest(&self) -> String {
        let mut hasher = CanonicalHasher::new(b"SOVEREIGN_EXECUTION_RECEIPT_DIGEST_V1");

        hasher.field(self.schema_version.as_bytes());
        hasher.field(self.receipt_id.0.as_bytes());
        hasher.field(self.attempt_reference.0.as_bytes());
        hasher.field(self.grant_reference.0.as_bytes());
        hasher.field(self.executor_identity.0.as_bytes());

        hasher.field(match self.result {
            ExecutionResult::Success => b"Success",
            ExecutionResult::Failure => b"Failure",
        });

        hasher.field(self.output_digest.as_bytes());
        hasher.field(self.completed_at.as_bytes());

        hasher.finish()
    }

    pub fn verify_receipt_integrity(&self) -> bool {
        self.receipt_digest == self.derive_receipt_digest()
    }
}

pub struct ExecutionAuthority {
    signing_key: SigningKey,
}

impl ExecutionAuthority {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(&seed),
        }
    }

    pub fn verifying_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn issue_receipt(
        &self,
        authorized: &AuthorizedExecution,
        result: ExecutionResult,
        output_digest: impl Into<String>,
        completed_at: impl Into<String>,
    ) -> ExecutionReceipt {
        let output_digest = output_digest.into();
        let completed_at = completed_at.into();
        let receipt_id = derive_receipt_id(
            &authorized.attempt_id,
            &authorized.grant_id,
            &authorized.executor_identity,
            &result,
            &output_digest,
            &completed_at,
        );
        let mut receipt = ExecutionReceipt {
            schema_version: "EXECUTION_RECEIPT-v1".into(),
            receipt_id,
            attempt_reference: authorized.attempt_id.clone(),
            grant_reference: authorized.grant_id.clone(),
            executor_identity: authorized.executor_identity.clone(),
            result,
            output_digest,
            completed_at,
            receipt_digest: String::new(),
            signature: [0u8; 64],
        };
        receipt.receipt_digest = receipt.derive_receipt_digest();

        receipt.signature = self
            .signing_key
            .sign(receipt_signature_digest(&receipt).as_bytes())
            .to_bytes();
        receipt
    }
}

pub fn derive_receipt_id(
    attempt_id: &AttemptId,
    grant_id: &GrantId,
    executor: &AgentIdentityId,
    result: &ExecutionResult,
    output_digest: &str,
    timestamp: &str,
) -> ReceiptId {
    let mut hasher = CanonicalHasher::new(b"SOVEREIGN_EXECUTION_RECEIPT_ID_V1");
    hasher.field(attempt_id.0.as_bytes());
    hasher.field(grant_id.0.as_bytes());
    hasher.field(executor.0.as_bytes());
    hasher.field(match result {
        ExecutionResult::Success => b"Success",
        ExecutionResult::Failure => b"Failure",
    });
    hasher.field(output_digest.as_bytes());
    hasher.field(timestamp.as_bytes());
    ReceiptId(hasher.finish())
}

pub fn validate_receipt_identity(receipt: &ExecutionReceipt) -> bool {
    receipt.receipt_id
        == derive_receipt_id(
            &receipt.attempt_reference,
            &receipt.grant_reference,
            &receipt.executor_identity,
            &receipt.result,
            &receipt.output_digest,
            &receipt.completed_at,
        )
}

pub fn verify_receipt(receipt: &ExecutionReceipt, verifying_key: &[u8; 32]) -> bool {
    if !validate_receipt_identity(receipt) || receipt.schema_version != "EXECUTION_RECEIPT-v1" {
        return false;
    }
    let Ok(key) = VerifyingKey::from_bytes(verifying_key) else {
        return false;
    };
    key.verify(
        receipt_signature_digest(receipt).as_bytes(),
        &Signature::from_bytes(&receipt.signature),
    )
    .is_ok()
}

fn receipt_signature_digest(receipt: &ExecutionReceipt) -> String {
    let mut hasher = CanonicalHasher::new(b"SOVEREIGN_EXECUTION_RECEIPT_SIGNATURE_V1");
    hasher.field(receipt.schema_version.as_bytes());
    hasher.field(receipt.receipt_id.0.as_bytes());
    hasher.field(receipt.attempt_reference.0.as_bytes());
    hasher.field(receipt.grant_reference.0.as_bytes());
    hasher.field(receipt.executor_identity.0.as_bytes());
    hasher.field(match receipt.result {
        ExecutionResult::Success => b"Success",
        ExecutionResult::Failure => b"Failure",
    });
    hasher.field(receipt.output_digest.as_bytes());
    hasher.field(receipt.completed_at.as_bytes());
    hasher.finish()
}
