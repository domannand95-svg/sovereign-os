use crate::governance_admission::{AdmissionDecision, AdmissionOutcome};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};

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
///
/// Identity derivation is based only on canonical bytes.
/// This provides deterministic identity, not authentication.
///
/// Digest != Signature.
/// Identity != Authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationReceiptIdentity {
    pub digest: [u8; 32],
}

impl AuthorizationReceiptIdentity {
    pub fn derive(payload: &CanonicalAuthorizationReceiptIdentityPayloadV1) -> Self {
        let canonical_bytes = payload.to_canonical_bytes();

        let digest = blake3::hash(&canonical_bytes);

        Self {
            digest: *digest.as_bytes(),
        }
    }
}
/// Canonical payload used for cryptographic receipt authentication.
///
/// This payload is separate from identity derivation.
/// It binds the complete authorization context to the issuer signature.
///
/// SignaturePayloadV1:
///     identity reference
///     +
///     authority context
///
/// Signature authenticity is implemented in a later boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignaturePayloadV1 {
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

    pub issuer_reference: String,
    pub nonce: String,

    pub revocation_reference: String,
}

impl SignaturePayloadV1 {
    const DOMAIN_SEPARATOR: &'static [u8] = b"SOV:AR:SIG:V1";

    /// Deterministic canonical signing payload encoding.
    ///
    /// This produces signing input only.
    /// It does not sign or verify.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();

        output.extend_from_slice(Self::DOMAIN_SEPARATOR);

        Self::encode_string(&mut output, &self.receipt_reference);

        Self::encode_string(&mut output, &self.subject_reference);

        Self::encode_string(&mut output, &self.intent_reference);
        Self::encode_string(&mut output, &self.admission_reference);
        Self::encode_string(&mut output, &self.policy_reference);
        Self::encode_string(&mut output, &self.governance_context_reference);

        Self::encode_string(&mut output, &self.authorized_operation);
        Self::encode_string(&mut output, &self.authorized_target);
        Self::encode_string(&mut output, &self.authorized_scope);

        output.extend_from_slice(&(self.constraints.len() as u32).to_be_bytes());

        for constraint in &self.constraints {
            Self::encode_string(&mut output, constraint);
        }

        output.extend_from_slice(&self.issued_at.to_be_bytes());
        output.extend_from_slice(&self.expires_at.to_be_bytes());

        Self::encode_string(&mut output, &self.issuer_reference);
        Self::encode_string(&mut output, &self.nonce);
        Self::encode_string(&mut output, &self.revocation_reference);

        output
    }

    fn encode_string(output: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();

        output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());

        output.extend_from_slice(bytes);
    }
}
impl SignaturePayloadV1 {
    /// Signs the canonical signature payload bytes.
    ///
    /// This operation:
    /// - consumes canonical payload representation
    /// - performs Ed25519 signing
    /// - returns the resulting signature
    ///
    /// This operation does not:
    /// - verify signatures
    /// - resolve issuers
    /// - mutate receipts
    /// - grant authority
    pub fn sign(&self, signing_key: &SigningKey) -> ed25519_dalek::Signature {
        let canonical_bytes = self.to_canonical_bytes();
        signing_key.sign(&canonical_bytes)
    }

    pub fn verify(
        &self,
        verifying_key: &VerifyingKey,
        signature: &ed25519_dalek::Signature,
    ) -> Result<(), ed25519_dalek::SignatureError> {
        let canonical_bytes = self.to_canonical_bytes();
        verifying_key.verify(&canonical_bytes, signature)
    }
}
/// Result of cryptographic receipt authentication.
///
/// This represents signature validity only.
/// It does not represent trust, authorization, or capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptAuthenticationResult {
    Valid,
    Invalid,
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
    /// Authenticates the receipt payload against a provided verifying key and signature.
    ///
    /// This operation:
    /// - verifies cryptographic validity of the signature over canonical bytes
    /// - returns authentication state only
    ///
    /// This operation does not:
    /// - resolve issuers
    /// - evaluate trust anchors
    /// - grant authority
    pub fn authenticate(
        &self,
        payload: &SignaturePayloadV1,
        verifying_key: &VerifyingKey,
        signature: &ed25519_dalek::Signature,
    ) -> ReceiptAuthenticationResult {
        match payload.verify(verifying_key, signature) {
            Ok(()) => ReceiptAuthenticationResult::Valid,
            Err(_) => ReceiptAuthenticationResult::Invalid,
        }
    }

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
            receipt_reference: hex::encode(identity.digest),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signature_payload() -> SignaturePayloadV1 {
        SignaturePayloadV1 {
            receipt_reference: "receipt-001".into(),
            subject_reference: "subject-001".into(),
            intent_reference: "intent-001".into(),
            admission_reference: "admission-001".into(),
            policy_reference: "policy-001".into(),
            governance_context_reference: "context-001".into(),
            authorized_operation: "operation-001".into(),
            authorized_target: "target-001".into(),
            authorized_scope: "scope-001".into(),
            constraints: vec!["constraint-001".into()],
            issued_at: 1000,
            expires_at: 2000,
            issuer_reference: "issuer-001".into(),
            nonce: "nonce-001".into(),
            revocation_reference: "revocation-001".into(),
        }
    }

    #[test]
    fn signature_payload_signing_is_deterministic() {
        let payload = test_signature_payload();
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);

        let first = payload.sign(&signing_key);
        let second = payload.sign(&signing_key);

        assert_eq!(first, second);
    }

    #[test]
    fn signature_payload_verification_succeeds_with_matching_key() {
        let payload = test_signature_payload();
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);

        let verifying_key = signing_key.verifying_key();

        let signature = payload.sign(&signing_key);

        assert!(payload.verify(&verifying_key, &signature).is_ok());
    }

    #[test]
    fn signature_payload_verification_fails_on_payload_mutation() {
        let mut payload = test_signature_payload();
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);

        let verifying_key = signing_key.verifying_key();

        let signature = payload.sign(&signing_key);

        payload.authorized_scope = "modified-scope".into();

        assert!(payload.verify(&verifying_key, &signature).is_err());
    }

    #[test]
    fn signature_payload_verification_fails_with_wrong_key() {
        let payload = test_signature_payload();

        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let wrong_key = SigningKey::from_bytes(&[8u8; 32]);

        let signature = payload.sign(&signing_key);

        assert!(payload
            .verify(&wrong_key.verifying_key(), &signature)
            .is_err());
    }
}
