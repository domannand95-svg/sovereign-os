//! ADAM-011-B: Typed Service Contract Boundary
//!
//! Hydrates the five canonical ADAM-011-A service schemas into inert Rust
//! data-transfer objects.
//!
//! Invariants:
//! - Typed hydration != authorization.
//! - Typed hydration != capability creation.
//! - Typed hydration != execution.
//! - External inference != internal authority.
//! - Authority delta remains zero.

use chrono::{DateTime, Utc};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

macro_rules! prefixed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;

                if !value.starts_with($prefix) || value.len() <= $prefix.len() {
                    return Err(de::Error::custom(concat!(
                        "identifier must use namespace prefix ",
                        $prefix
                    )));
                }

                let suffix = &value[$prefix.len()..];

                if !suffix
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    return Err(de::Error::custom(
                        "identifier contains invalid namespace characters",
                    ));
                }

                Ok(Self(value))
            }
        }
    };
}

prefixed_id!(UserId, "usr_");
prefixed_id!(SessionId, "ses_");
prefixed_id!(ProposalId, "prop_");
prefixed_id!(ApprovalReceiptId, "apr_");
prefixed_id!(ExecutionId, "exe_");

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        if value.len() != 64
            || !value
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            return Err(de::Error::custom(
                "SHA-256 digest must be exactly 64 lowercase hexadecimal characters",
            ));
        }

        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaVersionV1;

impl Serialize for SchemaVersionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("v1.0.0")
    }
}

impl<'de> Deserialize<'de> for SchemaVersionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        if value != "v1.0.0" {
            return Err(de::Error::custom("schema_version must equal v1.0.0"));
        }

        Ok(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityDeltaZero;

impl Serialize for AuthorityDeltaZero {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(0)
    }
}

impl<'de> Deserialize<'de> for AuthorityDeltaZero {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = i64::deserialize(deserializer)?;

        if value != 0 {
            return Err(de::Error::custom("authority_delta must equal zero"));
        }

        Ok(Self)
    }
}

fn deserialize_nonempty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;

    if value.is_empty() {
        return Err(de::Error::custom("value must not be empty"));
    }

    Ok(value)
}

fn deserialize_nonempty_digests<'de, D>(deserializer: D) -> Result<Vec<Sha256Digest>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<Sha256Digest>::deserialize(deserializer)?;

    if values.is_empty() {
        return Err(de::Error::custom(
            "source_evidence_references must contain at least one digest",
        ));
    }

    Ok(values)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposedOperation {
    EmitNotification,
    RequestReview,
    RequestApproval,
    RequestStateMutation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleState {
    Draft,
    Validated,
    RiskEvaluated,
    AwaitingApproval,
    Rejected,
    Quarantined,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlastRadius {
    Isolated,
    Subsystem,
    System,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequiredApprovalLevel {
    Peer,
    Operator,
    Governance,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskContext {
    #[serde(deserialize_with = "deserialize_nonempty_string")]
    pub context_id: String,
    pub risk_level: RiskLevel,
    pub blast_radius: BlastRadius,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalRequest {
    pub schema_version: SchemaVersionV1,
    pub user_id: UserId,
    pub session_id: SessionId,

    #[serde(deserialize_with = "deserialize_nonempty_string")]
    pub intent: String,

    pub proposed_operation: ProposedOperation,

    #[serde(deserialize_with = "deserialize_nonempty_digests")]
    pub source_evidence_references: Vec<Sha256Digest>,

    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalResponse {
    pub schema_version: SchemaVersionV1,
    pub proposal_id: ProposalId,
    pub session_id: SessionId,
    pub lifecycle_state: LifecycleState,
    pub risk_context: RiskContext,
    pub required_approval_level: RequiredApprovalLevel,
    pub authority_delta: AuthorityDeltaZero,
    pub provenance_digest: Sha256Digest,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRequest {
    pub schema_version: SchemaVersionV1,
    pub user_id: UserId,
    pub session_id: SessionId,
    pub proposal_id: ProposalId,
    pub approval_receipt_id: ApprovalReceiptId,

    #[serde(deserialize_with = "deserialize_nonempty_string")]
    pub caller_identity: String,

    #[serde(deserialize_with = "deserialize_nonempty_string")]
    pub signature: String,

    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimState {
    ExecutionClaimed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DispatchState {
    NotDispatched,
    Dispatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutcomeState {
    Pending,
    Succeeded,
    Failed,
    OutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutionResponse {
    pub schema_version: SchemaVersionV1,
    pub execution_id: ExecutionId,
    pub proposal_id: ProposalId,
    pub approval_receipt_id: ApprovalReceiptId,
    pub claim_state: ClaimState,
    pub dispatch_state: DispatchState,
    pub outcome_state: OutcomeState,
    pub authority_delta: AuthorityDeltaZero,
    pub evidence_package_digest: Sha256Digest,
    pub timestamp: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExecutionResponse {
    schema_version: SchemaVersionV1,
    execution_id: ExecutionId,
    proposal_id: ProposalId,
    approval_receipt_id: ApprovalReceiptId,
    claim_state: ClaimState,
    dispatch_state: DispatchState,
    outcome_state: OutcomeState,
    authority_delta: AuthorityDeltaZero,
    evidence_package_digest: Sha256Digest,
    timestamp: DateTime<Utc>,
}

impl<'de> Deserialize<'de> for ExecutionResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawExecutionResponse::deserialize(deserializer)?;

        if raw.dispatch_state == DispatchState::NotDispatched
            && raw.outcome_state != OutcomeState::Pending
        {
            return Err(de::Error::custom(
                "NOT_DISPATCHED execution must remain PENDING",
            ));
        }

        Ok(Self {
            schema_version: raw.schema_version,
            execution_id: raw.execution_id,
            proposal_id: raw.proposal_id,
            approval_receipt_id: raw.approval_receipt_id,
            claim_state: raw.claim_state,
            dispatch_state: raw.dispatch_state,
            outcome_state: raw.outcome_state,
            authority_delta: raw.authority_delta,
            evidence_package_digest: raw.evidence_package_digest,
            timestamp: raw.timestamp,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransportStatus {
    Success,
    Failed,
    Timeout,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InferenceGateway {
    pub schema_version: SchemaVersionV1,
    pub session_id: SessionId,
    pub provider_identity: String,
    pub adapter_identity: String,
    pub request_digest: Sha256Digest,
    pub response_digest: Option<Sha256Digest>,
    pub transport_status: TransportStatus,
    pub raw_model_output: Option<String>,
    pub evidence_seal_digest: Sha256Digest,
    pub timestamp: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawInferenceGateway {
    schema_version: SchemaVersionV1,
    session_id: SessionId,

    #[serde(deserialize_with = "deserialize_nonempty_string")]
    provider_identity: String,

    #[serde(deserialize_with = "deserialize_nonempty_string")]
    adapter_identity: String,

    request_digest: Sha256Digest,
    response_digest: Option<Sha256Digest>,
    transport_status: TransportStatus,
    raw_model_output: Option<String>,
    evidence_seal_digest: Sha256Digest,
    timestamp: DateTime<Utc>,
}

impl<'de> Deserialize<'de> for InferenceGateway {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawInferenceGateway::deserialize(deserializer)?;

        match raw.transport_status {
            TransportStatus::Success => {
                if raw.response_digest.is_none() || raw.raw_model_output.is_none() {
                    return Err(de::Error::custom(
                        "SUCCESS inference requires response_digest and raw_model_output",
                    ));
                }
            }
            TransportStatus::Failed | TransportStatus::Timeout | TransportStatus::Disconnected => {
                if raw.response_digest.is_some() || raw.raw_model_output.is_some() {
                    return Err(de::Error::custom(
                        "non-success inference must not carry response evidence",
                    ));
                }
            }
        }

        Ok(Self {
            schema_version: raw.schema_version,
            session_id: raw.session_id,
            provider_identity: raw.provider_identity,
            adapter_identity: raw.adapter_identity,
            request_digest: raw.request_digest,
            response_digest: raw.response_digest,
            transport_status: raw.transport_status,
            raw_model_output: raw.raw_model_output,
            evidence_seal_digest: raw.evidence_seal_digest,
            timestamp: raw.timestamp,
        })
    }
}
