//! BETA-001-T018-A Agent identity model.
//!
//! Identity establishes attribution only.
//! Identity does not grant authority, capability, or execution rights.

use crate::encoding::CanonicalHasher;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digest(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentIdentityId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayTimestamp(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentClass {
    LocalModel,
    ExternalModel,
    HumanOperator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentIdentityStatus {
    Pending,
    Active,
    Suspended,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    pub schema_version: SchemaVersion,
    pub identity_id: AgentIdentityId,
    pub public_key_digest: Digest,
    pub agent_class: AgentClass,
    pub created_at: ReplayTimestamp,
    pub governing_policy: PolicyId,
    pub status: AgentIdentityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityValidationError {
    InvalidSchema,
    InvalidPublicKeyDigest,
    InvalidIdentity,
    UnknownIdentity,
    /// The identity is already registered. Status changes go through
    /// `set_status`; registration never replaces an existing record.
    AlreadyRegistered,
    /// The identity is revoked. Revocation is final: access needs a new identity
    /// and a fresh approval. Suspension, by contrast, is reversible.
    IdentityRevoked,
}

#[derive(Default)]
pub struct IdentityRegistry {
    identities: HashMap<AgentIdentityId, AgentIdentity>,
}

impl IdentityRegistry {
    pub fn register(&mut self, identity: AgentIdentity) -> Result<(), IdentityValidationError> {
        validate_identity(&identity)?;
        if self.identities.contains_key(&identity.identity_id) {
            return Err(IdentityValidationError::AlreadyRegistered);
        }
        self.identities
            .insert(identity.identity_id.clone(), identity);
        Ok(())
    }

    pub fn set_status(
        &mut self,
        identity_id: &AgentIdentityId,
        status: AgentIdentityStatus,
    ) -> Result<(), IdentityValidationError> {
        let identity = self
            .identities
            .get_mut(identity_id)
            .ok_or(IdentityValidationError::UnknownIdentity)?;
        if identity.status == AgentIdentityStatus::Revoked {
            return Err(IdentityValidationError::IdentityRevoked);
        }
        identity.status = status;
        Ok(())
    }

    pub fn is_active(&self, identity_id: &AgentIdentityId) -> bool {
        self.identities
            .get(identity_id)
            .is_some_and(|identity| identity.status == AgentIdentityStatus::Active)
    }
}

pub fn validate_identity(identity: &AgentIdentity) -> Result<(), IdentityValidationError> {
    if identity.schema_version.0 != "AGENT_IDENTITY-v1" {
        return Err(IdentityValidationError::InvalidSchema);
    }
    let key = hex::decode(&identity.public_key_digest.0)
        .map_err(|_| IdentityValidationError::InvalidPublicKeyDigest)?;
    if key.len() != 32 {
        return Err(IdentityValidationError::InvalidPublicKeyDigest);
    }
    if identity.identity_id
        != derive_identity_id(
            &identity.public_key_digest,
            &identity.agent_class,
            &identity.governing_policy,
        )
    {
        return Err(IdentityValidationError::InvalidIdentity);
    }
    Ok(())
}

pub fn derive_identity_id(key: &Digest, class: &AgentClass, policy: &PolicyId) -> AgentIdentityId {
    let mut hasher = CanonicalHasher::new(b"SOVEREIGN_AGENT_IDENTITY_V1");
    hasher.field(key.0.as_bytes());

    match class {
        AgentClass::LocalModel => {
            hasher.field(b"LocalModel");
        }
        AgentClass::ExternalModel => {
            hasher.field(b"ExternalModel");
        }
        AgentClass::HumanOperator => {
            hasher.field(b"HumanOperator");
        }
    }

    hasher.field(policy.0.as_bytes());
    AgentIdentityId(hasher.finish())
}
