// ============================================================================
// AGENT-BETA-016-A: Ephemeral Capability Token Schema & Minting Boundary
// ============================================================================
// Authority Expansion Target: ZERO (Strict Ephemeral Lease Only)
// Invariant: Sealed Dossier -> Ephemeral Capability Token -> Zero Ambient Authority
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalLevel {
    Peer,
    Operator,
    Governance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRecord {
    pub proposal_id: ProposalId,
    pub approver_identity: String,
    pub granted_approval_level: ApprovalLevel,
    pub signature_hash: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityScope {
    pub target_resource: String,
    pub operation_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedGovernanceDossier {
    pub proposal_id: ProposalId,
    pub proposal_hash: String,
    pub approval_record: ApprovalRecord,
    pub capability_scope: CapabilityScope,
    pub sealed_timestamp: u64,
    pub authority_expansion: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EphemeralCapabilityToken {
    pub token_id: String,
    pub proposal_id: ProposalId,
    pub capability_scope: CapabilityScope,
    pub issued_at: u64,
    pub expires_at: u64,
    pub nonce: u64,
    pub cryptographic_proof: String,
    pub single_use: bool,
}

impl EphemeralCapabilityToken {
    pub fn is_valid(&self, current_timestamp: u64) -> bool {
        current_timestamp >= self.issued_at && current_timestamp <= self.expires_at
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MintingError {
    AuthorityExpansionViolation,
    InvalidDossierTimestamp,
    EmptyOperationalScope,
}

pub struct CapabilityTokenMinter;

impl CapabilityTokenMinter {
    pub const MAX_TOKEN_TTL_SECONDS: u64 = 60;
    pub const DOMAIN_TAG: &'static str = "SOVEREIGN_CAPABILITY_TOKEN_V1";

    pub fn mint_token(
        dossier: &SealedGovernanceDossier,
        current_timestamp: u64,
        nonce: u64,
    ) -> Result<EphemeralCapabilityToken, MintingError> {
        if dossier.authority_expansion != 0 {
            return Err(MintingError::AuthorityExpansionViolation);
        }

        if current_timestamp < dossier.sealed_timestamp {
            return Err(MintingError::InvalidDossierTimestamp);
        }

        if dossier.capability_scope.target_resource.is_empty()
            || dossier.capability_scope.operation_type.is_empty()
        {
            return Err(MintingError::EmptyOperationalScope);
        }

        let expires_at = current_timestamp + Self::MAX_TOKEN_TTL_SECONDS;

        let mut hasher = blake3::Hasher::new();
        hasher.update(Self::DOMAIN_TAG.as_bytes());
        hasher.update(dossier.proposal_id.0.as_bytes());
        hasher.update(dossier.proposal_hash.as_bytes());
        hasher.update(dossier.capability_scope.target_resource.as_bytes());
        hasher.update(dossier.capability_scope.operation_type.as_bytes());
        hasher.update(&current_timestamp.to_be_bytes());
        hasher.update(&expires_at.to_be_bytes());
        hasher.update(&nonce.to_be_bytes());

        let hex_digest = hasher.finalize().to_hex().to_string();

        let proof = format!("blake3:{}", hex_digest);
        let token_id = format!("cap_tok_{}", &hex_digest[..16]);

        Ok(EphemeralCapabilityToken {
            token_id,
            proposal_id: dossier.proposal_id.clone(),
            capability_scope: dossier.capability_scope.clone(),
            issued_at: current_timestamp,
            expires_at,
            nonce,
            cryptographic_proof: proof,
            single_use: true,
        })
    }
}

// ============================================================================
// CAPABILITY VALIDATION SUITE (CAP-01..04)
// ============================================================================

#[cfg(test)]
mod capability_token_tests {
    use super::*;

    fn get_valid_dossier() -> SealedGovernanceDossier {
        SealedGovernanceDossier {
            proposal_id: ProposalId("PROP-EXEC-001".into()),
            proposal_hash: "blake3:dossier_hash_123".into(),
            approval_record: ApprovalRecord {
                proposal_id: ProposalId("PROP-EXEC-001".into()),
                approver_identity: "governance_chair".into(),
                granted_approval_level: ApprovalLevel::Governance,
                signature_hash: "SIG-GOV-999".into(),
                timestamp: 1710000000,
            },
            capability_scope: CapabilityScope {
                target_resource: "urn:internal:entity:x".into(),
                operation_type: "QUARANTINE".into(),
            },
            sealed_timestamp: 1710000000,
            authority_expansion: 0,
        }
    }

    #[test]
    fn cap_01_valid_token_minting() {
        let dossier = get_valid_dossier();
        let token = CapabilityTokenMinter::mint_token(&dossier, 1710000010, 42).unwrap();

        assert_eq!(token.proposal_id.0, "PROP-EXEC-001");
        assert_eq!(token.capability_scope.target_resource, "urn:internal:entity:x");
        assert_eq!(token.capability_scope.operation_type, "QUARANTINE");
        assert_eq!(token.expires_at - token.issued_at, 60);
        assert_eq!(token.nonce, 42);
        assert!(token.single_use);
        assert!(token.token_id.starts_with("cap_tok_"));
        assert!(token.cryptographic_proof.starts_with("blake3:"));
        assert_eq!(token.cryptographic_proof.len(), 71); // "blake3:" (7) + 64 hex chars
        assert!(token.is_valid(1710000010));
        assert!(token.is_valid(1710000070));
        assert!(!token.is_valid(1710000071));
    }

    #[test]
    fn cap_02_authority_expansion_rejection() {
        let mut dossier = get_valid_dossier();
        dossier.authority_expansion = 1;

        let result = CapabilityTokenMinter::mint_token(&dossier, 1710000010, 42);
        assert_eq!(result, Err(MintingError::AuthorityExpansionViolation));
    }

    #[test]
    fn cap_03_temporal_inversion_rejection() {
        let dossier = get_valid_dossier();
        let current_time_before_sealed = 1709999990;

        let result = CapabilityTokenMinter::mint_token(&dossier, current_time_before_sealed, 42);
        assert_eq!(result, Err(MintingError::InvalidDossierTimestamp));
    }

    #[test]
    fn cap_04_empty_scope_rejection() {
        let mut dossier = get_valid_dossier();
        dossier.capability_scope.operation_type = "".into();

        let result = CapabilityTokenMinter::mint_token(&dossier, 1710000010, 42);
        assert_eq!(result, Err(MintingError::EmptyOperationalScope));
    }
}
