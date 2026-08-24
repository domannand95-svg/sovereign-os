//! Service Contract Boundary DTOs and Strong Domain Types (ADAM-011-B)
//!
//! Provides Draft 2020-12 compliant, strongly-typed DTOs for the Sovereign OS Service API.
//! Invariant: Ingestion != Authorization != Execution (Δ Authority = 0)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceContractError {
    InvalidIdentifierPrefix {
        expected: &'static str,
        found: String,
    },
    InvalidSha256Digest(String),
    InvalidAuthorityDelta(i64),
    InvalidSchemaVersion(String),
    ValidationError(String),
}

impl fmt::Display for ServiceContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifierPrefix { expected, found } => {
                write!(
                    f,
                    "Invalid identifier prefix: expected '{}', found '{}'",
                    expected, found
                )
            }
            Self::InvalidSha256Digest(s) => write!(f, "Invalid SHA-256 digest: '{}'", s),
            Self::InvalidAuthorityDelta(d) => write!(f, "Authority delta must be 0, found {}", d),
            Self::InvalidSchemaVersion(v) => write!(f, "Unsupported schema version: '{}'", v),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ServiceContractError {}

macro_rules! define_id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ServiceContractError> {
                let s = value.into();
                if s.starts_with($prefix) && s.len() > $prefix.len() {
                    Ok(Self(s))
                } else {
                    Err(ServiceContractError::InvalidIdentifierPrefix {
                        expected: $prefix,
                        found: s,
                    })
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = ServiceContractError;
            fn try_from(s: String) -> Result<Self, Self::Error> {
                Self::new(s)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = ServiceContractError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s.to_string())
            }
        }
    };
}

define_id_type!(UserId, "usr_");
define_id_type!(SessionId, "ses_");
define_id_type!(ProposalId, "prop_");
define_id_type!(ApprovalReceiptId, "apr_");
define_id_type!(ExecutionId, "exe_");

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Digest(String);

impl Sha256Digest {
    pub fn new(value: impl Into<String>) -> Result<Self, ServiceContractError> {
        let s = value.into();
        if s.len() == 64
            && s.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            Ok(Self(s))
        } else {
            Err(ServiceContractError::InvalidSha256Digest(s))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Sha256Digest {
    type Error = ServiceContractError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<Sha256Digest> for String {
    fn from(digest: Sha256Digest) -> Self {
        digest.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Sha256Digest {
    type Err = ServiceContractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct AuthorityDeltaZero;

impl AuthorityDeltaZero {
    pub fn value(&self) -> i64 {
        0
    }
}

impl TryFrom<i64> for AuthorityDeltaZero {
    type Error = ServiceContractError;
    fn try_from(val: i64) -> Result<Self, Self::Error> {
        if val == 0 {
            Ok(Self)
        } else {
            Err(ServiceContractError::InvalidAuthorityDelta(val))
        }
    }
}

impl From<AuthorityDeltaZero> for i64 {
    fn from(_: AuthorityDeltaZero) -> Self {
        0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SchemaVersionV1;

impl TryFrom<String> for SchemaVersionV1 {
    type Error = ServiceContractError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s == "v1.0.0" {
            Ok(Self)
        } else {
            Err(ServiceContractError::InvalidSchemaVersion(s))
        }
    }
}

impl From<SchemaVersionV1> for String {
    fn from(_: SchemaVersionV1) -> Self {
        "v1.0.0".to_string()
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransportStatus {
    Success,
    Failed,
    Timeout,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskContext {
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
    pub intent: String,
    pub proposed_operation: ProposedOperation,
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
    pub caller_identity: String,
    pub signature: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawExecutionResponse", into = "RawExecutionResponse")]
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExecutionResponse {
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

impl From<ExecutionResponse> for RawExecutionResponse {
    fn from(res: ExecutionResponse) -> Self {
        Self {
            schema_version: res.schema_version,
            execution_id: res.execution_id,
            proposal_id: res.proposal_id,
            approval_receipt_id: res.approval_receipt_id,
            claim_state: res.claim_state,
            dispatch_state: res.dispatch_state,
            outcome_state: res.outcome_state,
            authority_delta: res.authority_delta,
            evidence_package_digest: res.evidence_package_digest,
            timestamp: res.timestamp,
        }
    }
}

impl TryFrom<RawExecutionResponse> for ExecutionResponse {
    type Error = ServiceContractError;
    fn try_from(raw: RawExecutionResponse) -> Result<Self, Self::Error> {
        if raw.dispatch_state == DispatchState::NotDispatched
            && raw.outcome_state != OutcomeState::Pending
        {
            return Err(ServiceContractError::ValidationError(
                "NotDispatched execution response must have outcome_state Pending".to_string(),
            ));
        }
        if (raw.outcome_state == OutcomeState::Succeeded
            || raw.outcome_state == OutcomeState::Failed
            || raw.outcome_state == OutcomeState::OutcomeUnknown)
            && raw.dispatch_state != DispatchState::Dispatched
        {
            return Err(ServiceContractError::ValidationError(
                "Non-pending outcome requires Dispatched state".to_string(),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawInferenceGateway", into = "RawInferenceGateway")]
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

pub type InferenceGatewayEnvelope = InferenceGateway;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawInferenceGateway {
    pub schema_version: SchemaVersionV1,
    pub session_id: SessionId,
    pub provider_identity: String,
    pub adapter_identity: String,
    pub request_digest: Sha256Digest,
    #[serde(default)]
    pub response_digest: Option<Sha256Digest>,
    pub transport_status: TransportStatus,
    #[serde(default)]
    pub raw_model_output: Option<String>,
    pub evidence_seal_digest: Sha256Digest,
    pub timestamp: DateTime<Utc>,
}

impl From<InferenceGateway> for RawInferenceGateway {
    fn from(g: InferenceGateway) -> Self {
        Self {
            schema_version: g.schema_version,
            session_id: g.session_id,
            provider_identity: g.provider_identity,
            adapter_identity: g.adapter_identity,
            request_digest: g.request_digest,
            response_digest: g.response_digest,
            transport_status: g.transport_status,
            raw_model_output: g.raw_model_output,
            evidence_seal_digest: g.evidence_seal_digest,
            timestamp: g.timestamp,
        }
    }
}

impl TryFrom<RawInferenceGateway> for InferenceGateway {
    type Error = ServiceContractError;
    fn try_from(raw: RawInferenceGateway) -> Result<Self, Self::Error> {
        match raw.transport_status {
            TransportStatus::Success => {
                if raw.response_digest.is_none() || raw.raw_model_output.is_none() {
                    return Err(ServiceContractError::ValidationError(
                        "TransportStatus::Success requires response_digest and raw_model_output"
                            .to_string(),
                    ));
                }
            }
            TransportStatus::Failed | TransportStatus::Timeout | TransportStatus::Disconnected => {
                if raw.response_digest.is_some() || raw.raw_model_output.is_some() {
                    return Err(ServiceContractError::ValidationError(
                        "Non-success TransportStatus must have null response_digest and raw_model_output".to_string(),
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
