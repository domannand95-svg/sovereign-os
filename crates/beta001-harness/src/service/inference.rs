//! ADAM-011-E: External Provider Inference Gateway Contract
//!
//! Provides deterministic evidence capture, request/response hashing,
//! transport outcome coupling, and domain-separated evidence sealing for probabilistic model interactions.
//!
//! Invariant: Provider Intelligence != Authority (Δ Authority = 0)

use chrono::{DateTime, Utc};
use sha2::digest::Digest;
use sha2::Sha256;

use crate::service_contract::{
    InferenceGateway, SessionId, Sha256Digest, SchemaVersionV1, TransportStatus,
};

pub const INFERENCE_DOMAIN_TAG: &str = "SOVEREIGN_OS:INFERENCE_GATEWAY:v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceGatewayError {
    DigestDerivationFailed(String),
}

/// Strongly typed transport outcome representing external provider execution facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedTransportOutcome {
    Success { raw_model_output: String },
    Failed,
    Timeout,
    Disconnected,
}

pub struct InferenceGatewayAdapter;

impl InferenceGatewayAdapter {
    /// Records a provider transport interaction, computes cryptographic payload digests,
    /// enforces outcome-payload coupling, and produces a domain-sealed InferenceGateway envelope.
    pub fn record_transport(
        session_id: SessionId,
        provider_identity: impl Into<String>,
        adapter_identity: impl Into<String>,
        request_payload: &str,
        outcome: RecordedTransportOutcome,
        timestamp: DateTime<Utc>,
    ) -> Result<InferenceGateway, InferenceGatewayError> {
        let provider_id = provider_identity.into();
        let adapter_id = adapter_identity.into();

        // 1. Compute lowercase SHA-256 request digest (E011-002)
        let mut req_hasher = Sha256::new();
        req_hasher.update(request_payload.as_bytes());
        let req_digest_hex = format!("{:x}", req_hasher.finalize());
        let request_digest = Sha256Digest::new(req_digest_hex)
            .map_err(|e| InferenceGatewayError::DigestDerivationFailed(e.to_string()))?;

        // 2. Couple transport outcome with response payload and digest (E011-003)
        let (transport_status, response_digest, raw_model_output) = match outcome {
            RecordedTransportOutcome::Success { raw_model_output } => {
                let mut resp_hasher = Sha256::new();
                resp_hasher.update(raw_model_output.as_bytes());
                let resp_digest_hex = format!("{:x}", resp_hasher.finalize());
                let resp_digest = Sha256Digest::new(resp_digest_hex)
                    .map_err(|e| InferenceGatewayError::DigestDerivationFailed(e.to_string()))?;
                (TransportStatus::Success, Some(resp_digest), Some(raw_model_output))
            }
            RecordedTransportOutcome::Failed => (TransportStatus::Failed, None, None),
            RecordedTransportOutcome::Timeout => (TransportStatus::Timeout, None, None),
            RecordedTransportOutcome::Disconnected => (TransportStatus::Disconnected, None, None),
        };

        // 3. Compute domain-separated deterministic evidence seal digest (E011-004)
        let evidence_seal_digest = Self::derive_seal_digest(
            &session_id,
            &provider_id,
            &adapter_id,
            &request_digest,
            response_digest.as_ref(),
            transport_status,
            &timestamp,
        )?;

        // 4. Return canonical envelope (E011-001, E011-007: carries zero authority fields)
        Ok(InferenceGateway {
            schema_version: SchemaVersionV1,
            session_id,
            provider_identity: provider_id,
            adapter_identity: adapter_id,
            request_digest,
            response_digest,
            transport_status,
            raw_model_output,
            evidence_seal_digest,
            timestamp,
        })
    }

    /// Derives canonical domain-separated evidence seal digest using length-prefixed preimage encoding.
    fn derive_seal_digest(
        session_id: &SessionId,
        provider_id: &str,
        adapter_id: &str,
        request_digest: &Sha256Digest,
        response_digest: Option<&Sha256Digest>,
        transport_status: TransportStatus,
        timestamp: &DateTime<Utc>,
    ) -> Result<Sha256Digest, InferenceGatewayError> {
        let mut hasher = Sha256::new();

        // Length-prefixed domain tag
        hasher.update(format!("{}:", INFERENCE_DOMAIN_TAG.len()).as_bytes());
        hasher.update(INFERENCE_DOMAIN_TAG.as_bytes());

        // Length-prefixed session ID
        hasher.update(format!("{}:", session_id.as_str().len()).as_bytes());
        hasher.update(session_id.as_str().as_bytes());

        // Length-prefixed provider ID
        hasher.update(format!("{}:", provider_id.len()).as_bytes());
        hasher.update(provider_id.as_bytes());

        // Length-prefixed adapter ID
        hasher.update(format!("{}:", adapter_id.len()).as_bytes());
        hasher.update(adapter_id.as_bytes());

        // Length-prefixed request digest
        hasher.update(format!("{}:", request_digest.as_str().len()).as_bytes());
        hasher.update(request_digest.as_str().as_bytes());

        // Length-prefixed response digest or null literal
        let resp_str = response_digest.map(|d| d.as_str()).unwrap_or("null");
        hasher.update(format!("{}:", resp_str.len()).as_bytes());
        hasher.update(resp_str.as_bytes());

        // Length-prefixed transport status enum string
        let status_str = match transport_status {
            TransportStatus::Success => "SUCCESS",
            TransportStatus::Failed => "FAILED",
            TransportStatus::Timeout => "TIMEOUT",
            TransportStatus::Disconnected => "DISCONNECTED",
        };
        hasher.update(format!("{}:", status_str.len()).as_bytes());
        hasher.update(status_str.as_bytes());

        // Length-prefixed canonical RFC3339 UTC timestamp
        let ts_str = timestamp.to_rfc3339();
        hasher.update(format!("{}:", ts_str.len()).as_bytes());
        hasher.update(ts_str.as_bytes());

        let seal_hex = format!("{:x}", hasher.finalize());
        Sha256Digest::new(seal_hex)
            .map_err(|e| InferenceGatewayError::DigestDerivationFailed(e.to_string()))
    }
}