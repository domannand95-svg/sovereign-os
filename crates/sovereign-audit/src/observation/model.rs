use crate::identity::AgentIdentityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObservationId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PayloadDigest(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationType {
    RepositoryState,
    FileSystemState,
    EnvironmentState,
    ExternalEvidence,
    RuntimeTelemetry,
    SensorInput,
}

impl ObservationType {
    pub fn canonical_bytes(&self) -> &'static [u8] {
        match self {
            Self::RepositoryState => b"RepositoryState",
            Self::FileSystemState => b"FileSystemState",
            Self::EnvironmentState => b"EnvironmentState",
            Self::ExternalEvidence => b"ExternalEvidence",
            Self::RuntimeTelemetry => b"RuntimeTelemetry",
            Self::SensorInput => b"SensorInput",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRecord {
    pub schema_version: String,
    pub observation_id: ObservationId,
    pub observer_identity: AgentIdentityId,
    pub observation_type: ObservationType,
    pub payload_digest: PayloadDigest,
    pub observed_at: String,
    pub source_reference: String,
}
