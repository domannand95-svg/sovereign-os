//! ADAM-011-D: Untrusted Client Trust Boundary & Sanitization
//!
//! Provides deterministic ingress normalization, size gating, temporal skew verification,
//! and authenticated session context binding.
//!
//! Invariant: Sanitization != Trust (Δ Authority = 0)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::digest::Digest;
use sha2::Sha256;
use unicode_normalization::UnicodeNormalization;

use crate::service_contract::{ProposalRequest, SessionId, Sha256Digest, UserId};

pub const MAX_INTENT_BYTES: usize = 65_536;
pub const MAX_EVIDENCE_REFERENCES: usize = 32;
pub const MAX_ALLOWED_CLOCK_SKEW_SECONDS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientSanitizationError {
    SessionIdentityMismatch {
        claimed_user: String,
        authenticated_user: String,
        claimed_session: String,
        authenticated_session: String,
    },
    TimestampSkewExceeded {
        client_time: DateTime<Utc>,
        server_time: DateTime<Utc>,
        skew_seconds: i64,
    },
    IntentPayloadExceeded {
        size_bytes: usize,
        max_bytes: usize,
    },
    EvidenceReferencesExceeded {
        count: usize,
        max_count: usize,
    },
    ProhibitedCharactersDetected,
}

/// Authenticated context originating strictly from trusted gateway authentication mechanisms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticatedClientContext {
    pub authenticated_user_id: UserId,
    pub authenticated_session_id: SessionId,
}

/// Sanitized proposal envelope containing immutable raw evidence and unprivileged normalized intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedProposalRequest {
    pub original_request: ProposalRequest,
    pub raw_intent_digest: Sha256Digest,
    pub normalized_intent: String,
    pub normalized_intent_digest: Sha256Digest,
    pub server_intake_timestamp: DateTime<Utc>,
}

pub struct ClientSanitizer;

impl ClientSanitizer {
    /// Sanitizes an untrusted proposal request against trusted server context and temporal bounds.
    pub fn sanitize_proposal_request(
        req: ProposalRequest,
        auth_ctx: &AuthenticatedClientContext,
        server_intake_time: DateTime<Utc>,
    ) -> Result<SanitizedProposalRequest, ClientSanitizationError> {
        // 1. Authenticated Session & Identity Context Binding (D011-005)
        if req.user_id != auth_ctx.authenticated_user_id
            || req.session_id != auth_ctx.authenticated_session_id
        {
            return Err(ClientSanitizationError::SessionIdentityMismatch {
                claimed_user: req.user_id.as_str().to_string(),
                authenticated_user: auth_ctx.authenticated_user_id.as_str().to_string(),
                claimed_session: req.session_id.as_str().to_string(),
                authenticated_session: auth_ctx.authenticated_session_id.as_str().to_string(),
            });
        }

        // 2. Payload Exhaustion Gating (D011-003)
        let raw_intent_bytes = req.intent.len();
        if raw_intent_bytes > MAX_INTENT_BYTES {
            return Err(ClientSanitizationError::IntentPayloadExceeded {
                size_bytes: raw_intent_bytes,
                max_bytes: MAX_INTENT_BYTES,
            });
        }

        let evidence_count = req.source_evidence_references.len();
        if evidence_count > MAX_EVIDENCE_REFERENCES {
            return Err(ClientSanitizationError::EvidenceReferencesExceeded {
                count: evidence_count,
                max_count: MAX_EVIDENCE_REFERENCES,
            });
        }

        // 3. Injected Server Clock Skew Check (D011-002)
        let skew = (req.timestamp - server_intake_time).num_seconds().abs();
        if skew > MAX_ALLOWED_CLOCK_SKEW_SECONDS {
            return Err(ClientSanitizationError::TimestampSkewExceeded {
                client_time: req.timestamp,
                server_time: server_intake_time,
                skew_seconds: skew,
            });
        }

        // 4. Compute Raw Intent Digest (Preserves raw client evidence) (D011-001)
        let mut hasher_raw = Sha256::new();
        hasher_raw.update(req.intent.as_bytes());
        let raw_digest_hex = format!("{:x}", hasher_raw.finalize());
        let raw_intent_digest = Sha256Digest::new(raw_digest_hex)
            .map_err(|_| ClientSanitizationError::ProhibitedCharactersDetected)?;

        // 5. Deterministic Unicode NFKC Normalization & Control Character Filtering (D011-001)
        let filtered: String = req
            .intent
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
            .collect();
        let normalized_intent: String = filtered.nfkc().collect();

        // 6. Compute Normalized Intent Digest (D011-001)
        let mut hasher_norm = Sha256::new();
        hasher_norm.update(normalized_intent.as_bytes());
        let norm_digest_hex = format!("{:x}", hasher_norm.finalize());
        let normalized_intent_digest = Sha256Digest::new(norm_digest_hex)
            .map_err(|_| ClientSanitizationError::ProhibitedCharactersDetected)?;

        Ok(SanitizedProposalRequest {
            original_request: req,
            raw_intent_digest,
            normalized_intent,
            normalized_intent_digest,
            server_intake_timestamp: server_intake_time,
        })
    }
}
