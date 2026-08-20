use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentIdentityId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Digest(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolicyId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReplayTimestamp(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentClass {
    HumanOperator,
    LocalModel,
    ExternalModel,
    SystemService,
    EvaluationHarness,
}

impl AgentClass {
    pub fn canonical_bytes(&self) -> &'static [u8] {
        match self {
            Self::HumanOperator => b"HumanOperator",
            Self::LocalModel => b"LocalModel",
            Self::ExternalModel => b"ExternalModel",
            Self::SystemService => b"SystemService",
            Self::EvaluationHarness => b"EvaluationHarness",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentIdentityStatus {
    Pending,
    Active,
    Suspended,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub schema_version: SchemaVersion,
    pub identity_id: AgentIdentityId,
    pub public_key_digest: Digest,
    pub agent_class: AgentClass,
    pub created_at: ReplayTimestamp,
    pub governing_policy: PolicyId,
    pub status: AgentIdentityStatus,
}
