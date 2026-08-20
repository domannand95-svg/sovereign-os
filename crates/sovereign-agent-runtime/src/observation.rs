use crate::encoding::CanonicalHasher;
use crate::identity::AgentIdentityId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadDigest(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationType {
    RepositoryState,
    FileSystemState,
    EnvironmentState,
    RuntimeTelemetry,
    ExternalEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationRecord {
    pub schema_version: String,
    pub observation_id: ObservationId,
    pub observer_identity: AgentIdentityId,
    pub observation_type: ObservationType,
    pub payload_digest: PayloadDigest,
    pub observed_at: String,
    pub source_reference: String,
}

pub fn derive_observation_id(
    agent_id: &str,
    observation_type: &ObservationType,
    payload_digest: &PayloadDigest,
    timestamp: &str,
    source_reference: &str,
) -> ObservationId {
    let mut hasher = CanonicalHasher::new(b"SOVEREIGN_OBSERVATION_ID_V1");
    hasher.field(agent_id.as_bytes());

    match observation_type {
        ObservationType::RepositoryState => {
            hasher.field(b"RepositoryState");
        }
        ObservationType::FileSystemState => {
            hasher.field(b"FileSystemState");
        }
        ObservationType::EnvironmentState => {
            hasher.field(b"EnvironmentState");
        }
        ObservationType::RuntimeTelemetry => {
            hasher.field(b"RuntimeTelemetry");
        }
        ObservationType::ExternalEvidence => {
            hasher.field(b"ExternalEvidence");
        }
    }

    hasher.field(payload_digest.0.as_bytes());
    hasher.field(timestamp.as_bytes());
    hasher.field(source_reference.as_bytes());
    ObservationId(hasher.finish())
}

pub fn validate_observation_identity(record: &ObservationRecord) -> bool {
    record.schema_version == "OBSERVATION_RECORD-v1"
        && !record.source_reference.is_empty()
        && record.observation_id
            == derive_observation_id(
                &record.observer_identity.0,
                &record.observation_type,
                &record.payload_digest,
                &record.observed_at,
                &record.source_reference,
            )
}
