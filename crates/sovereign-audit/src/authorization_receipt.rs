use crate::governance_admission::{AdmissionDecision, AdmissionOutcome};

/// Represents the trusted governance node issuing authority artifacts.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IssuerContext {
    pub issuer_reference: String,
    pub signing_key_reference: String,
}

/// Deterministic identity payload for receipt identity derivation.
/// Cryptographic derivation is introduced in the next commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAuthorizationReceiptIdentityPayloadV1 {
    pub admission_reference: String,
    pub subject_reference: String,
    pub issued_at: u64,
    pub nonce: String,
}

/// Deterministic signature payload definition.
/// Signing implementation is intentionally deferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignaturePayloadV1 {
    pub subject_reference: String,
    pub intent_reference: String,
    pub admission_reference: String,
    pub policy_reference: String,
    pub governance_context_reference: String,
    pub authorized_operation: String,
    pub authorized_target: String,
    pub authorized_scope: String,
    pub constraints: Vec<String>,
    pub issued_at: u64,
    pub expires_at: u64,
    pub issuer_reference: String,
    pub nonce: String,
    pub revocation_reference: String,
}

/// Passive authority artifact.
/// Contains no execution capability.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthorizationReceipt {
    pub receipt_reference: String,

    pub subject_reference: String,

    pub intent_reference: String,
    pub admission_reference: String,
    pub policy_reference: String,
    pub governance_context_reference: String,

    pub authorized_operation: String,
    pub authorized_target: String,
    pub authorized_scope: String,

    pub constraints: Vec<String>,

    pub issued_at: u64,
    pub expires_at: u64,

    pub revocation_reference: String,

    pub issuer_reference: String,
    pub nonce: String,

    pub signature: String,
}

impl AuthorizationReceipt {
    pub const MAX_LIFETIME: u64 = 3600;

    pub fn validate_admission(decision: &AdmissionDecision) -> Result<(), &'static str> {
        if decision.outcome != AdmissionOutcome::Permit {
            return Err("Cannot issue authority for non-Permit admission");
        }

        Ok(())
    }
}