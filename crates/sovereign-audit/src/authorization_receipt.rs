use crate::governance_admission::{AdmissionDecision, AdmissionOutcome};

/// Represents the trusted governance node issuing authority artifacts.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IssuerContext {
    pub issuer_reference: String,
    pub signing_key_reference: String,
}

/// Canonical fields used to derive receipt identity.
/// This is identity derivation only, not signature authenticity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAuthorizationReceiptIdentityPayloadV1 {
    pub admission_reference: String,
    pub subject_reference: String,
    pub issued_at: u64,
    pub nonce: String,
}

impl CanonicalAuthorizationReceiptIdentityPayloadV1 {
    const DOMAIN_SEPARATOR: &'static [u8] = b"SOV:AR:IDENT:V1";

    /// Produces the deterministic canonical byte representation.
    ///
    /// Layout:
    ///
    /// [domain separator]
    /// [admission_reference length u32 BE]
    /// [admission_reference bytes]
    /// [subject_reference length u32 BE]
    /// [subject_reference bytes]
    /// [issued_at u64 BE]
    /// [nonce length u32 BE]
    /// [nonce bytes]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();

        output.extend_from_slice(Self::DOMAIN_SEPARATOR);

        Self::encode_string(&mut output, &self.admission_reference);

        Self::encode_string(&mut output, &self.subject_reference);

        output.extend_from_slice(&self.issued_at.to_be_bytes());

        Self::encode_string(&mut output, &self.nonce);

        output
    }

    fn encode_string(output: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();

        output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());

        output.extend_from_slice(bytes);
    }
}

/// Deterministic receipt identity boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationReceiptIdentity {
    pub receipt_id: String,
}

impl AuthorizationReceiptIdentity {
    pub fn derive(payload: &CanonicalAuthorizationReceiptIdentityPayloadV1) -> Self {
        let canonical_bytes = payload.to_canonical_bytes();

        let digest = format!(
            "canonical::{:x?}",
            canonical_bytes
        );

        Self {
            receipt_id: digest,
        }
    }
}

/// Deferred signing payload.
/// Cryptographic signing is intentionally not implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignaturePayloadV1 {
    pub receipt_reference: String,
    pub intent_reference: String,
    pub authorized_scope: String,
    pub expires_at: u64,
    pub issuer_reference: String,
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

    pub fn generate(
        decision: &AdmissionDecision,
        subject: &str,
        intent_ref: &str,
        policy_ref: &str,
        context_ref: &str,
        operation: &str,
        target: &str,
        issued_at: u64,
        expires_at: u64,
        issuer_context: &IssuerContext,
        nonce: &str,
    ) -> Result<Self, &'static str> {
        if decision.outcome != AdmissionOutcome::Permit {
            return Err("Cannot issue authority for non-Permit admission");
        }

        if subject.trim().is_empty() {
            return Err("Subject reference cannot be empty");
        }

        if subject == issuer_context.issuer_reference {
            return Err("Subject cannot self-issue authority");
        }

        if issuer_context.issuer_reference.trim().is_empty() {
            return Err("Issuer reference cannot be empty");
        }

        if nonce.trim().is_empty() {
            return Err("Nonce cannot be empty");
        }

        let identity_payload = CanonicalAuthorizationReceiptIdentityPayloadV1 {
            admission_reference: decision.decision_reference.clone(),
            subject_reference: subject.to_string(),
            issued_at,
            nonce: nonce.to_string(),
        };

        let identity = AuthorizationReceiptIdentity::derive(&identity_payload);

        let receipt = Self {
            receipt_reference: identity.receipt_id,
            subject_reference: subject.to_string(),
            intent_reference: intent_ref.to_string(),
            admission_reference: decision.decision_reference.clone(),
            policy_reference: policy_ref.to_string(),
            governance_context_reference: context_ref.to_string(),
            authorized_operation: operation.to_string(),
            authorized_target: target.to_string(),
            authorized_scope: decision.authorized_scope.clone(),
            constraints: vec![],
            issued_at,
            expires_at,
            revocation_reference: "pending_revocation_registry".to_string(),
            issuer_reference: issuer_context.issuer_reference.clone(),
            nonce: nonce.to_string(),
            signature: "pending_signature".to_string(),
        };

        receipt.validate()?;

        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.receipt_reference.trim().is_empty() {
            return Err("Receipt reference missing");
        }

        if self.subject_reference.trim().is_empty() {
            return Err("Subject missing");
        }

        if self.intent_reference.trim().is_empty() {
            return Err("Intent reference missing");
        }

        if self.admission_reference.trim().is_empty() {
            return Err("Admission reference missing");
        }

        if self.policy_reference.trim().is_empty() {
            return Err("Policy reference missing");
        }

        if self.governance_context_reference.trim().is_empty() {
            return Err("Governance context missing");
        }

        if self.authorized_operation.trim().is_empty() {
            return Err("Authorized operation missing");
        }

        if self.authorized_target.trim().is_empty() {
            return Err("Authorized target missing");
        }

        if self.authorized_scope.trim().is_empty() {
            return Err("Authorized scope missing");
        }

        if self.issuer_reference.trim().is_empty() {
            return Err("Issuer missing");
        }

        if self.nonce.trim().is_empty() {
            return Err("Nonce missing");
        }

        if self.expires_at <= self.issued_at {
            return Err("Expiration must be strictly greater than issuance");
        }

        let lifetime = self
            .expires_at
            .checked_sub(self.issued_at)
            .ok_or("Invalid lifetime calculation")?;

        if lifetime > Self::MAX_LIFETIME {
            return Err("Authority lifetime exceeds maximum allowed bounds");
        }

        Ok(())
    }
}