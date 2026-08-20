// ============================================================================
// AGENT-BETA-016-E: End-to-End Adversarial Execution Replay Suite
// ============================================================================
// Invariant: Full Pipeline Proof -> Zero Ambient Authority -> Deterministic State
// ============================================================================

use std::collections::{HashMap, HashSet};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionDisposition {
    Success,
    Failed(String),
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReceipt {
    pub receipt_id: String,
    pub token_id: String,
    pub target_resource: String,
    pub operation_executed: String,
    pub disposition: ExecutionDisposition,
    pub pre_state_digest: String,
    pub post_state_digest: String,
    pub executed_at: u64,
    pub residual_authority_retained: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PipelineError {
    AuthorityExpansionViolation,
    ScopeMismatch,
    TokenExpired,
    ReplayDetected,
    AttestationValidationFailed,
}

pub struct ExecutionOrchestrator {
    consumed_tokens: HashSet<String>,
}

impl ExecutionOrchestrator {
    pub fn new() -> Self {
        Self {
            consumed_tokens: HashSet::new(),
        }
    }

    pub fn mint_token(
        &self,
        dossier: &SealedGovernanceDossier,
        current_timestamp: u64,
        nonce: u64,
    ) -> Result<EphemeralCapabilityToken, PipelineError> {
        if dossier.authority_expansion != 0 {
            return Err(PipelineError::AuthorityExpansionViolation);
        }

        let expires_at = current_timestamp + 60;
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"SOVEREIGN_CAPABILITY_TOKEN_V1");
        hasher.update(dossier.proposal_id.0.as_bytes());
        hasher.update(dossier.proposal_hash.as_bytes());
        hasher.update(dossier.capability_scope.target_resource.as_bytes());
        hasher.update(dossier.capability_scope.operation_type.as_bytes());
        hasher.update(&current_timestamp.to_be_bytes());
        hasher.update(&expires_at.to_be_bytes());
        hasher.update(&nonce.to_be_bytes());

        let hex_digest = hasher.finalize().to_hex().to_string();

        Ok(EphemeralCapabilityToken {
            token_id: format!("cap_tok_{}", &hex_digest[..16]),
            proposal_id: dossier.proposal_id.clone(),
            capability_scope: dossier.capability_scope.clone(),
            issued_at: current_timestamp,
            expires_at,
            nonce,
            cryptographic_proof: format!("blake3:{}", hex_digest),
            single_use: true,
        })
    }

    pub fn dispatch_and_execute(
        &mut self,
        token: &EphemeralCapabilityToken,
        requested_scope: &CapabilityScope,
        pre_state: &str,
        post_state: &str,
        current_timestamp: u64,
    ) -> Result<ExecutionReceipt, PipelineError> {
        if current_timestamp > token.expires_at {
            return Err(PipelineError::TokenExpired);
        }

        if self.consumed_tokens.contains(&token.token_id) {
            return Err(PipelineError::ReplayDetected);
        }

        if token.capability_scope.target_resource != requested_scope.target_resource
            || token.capability_scope.operation_type != requested_scope.operation_type
        {
            return Err(PipelineError::ScopeMismatch);
        }

        self.consumed_tokens.insert(token.token_id.clone());

        Ok(ExecutionReceipt {
            receipt_id: format!("rcpt_{}", token.token_id),
            token_id: token.token_id.clone(),
            target_resource: requested_scope.target_resource.clone(),
            operation_executed: requested_scope.operation_type.clone(),
            disposition: ExecutionDisposition::Success,
            pre_state_digest: pre_state.to_string(),
            post_state_digest: post_state.to_string(),
            executed_at: current_timestamp,
            residual_authority_retained: 0,
        })
    }
}

// ============================================================================
// ADVERSARIAL EXECUTION REPLAY SUITE (AER-01..05)
// ============================================================================

#[cfg(test)]
mod adversarial_execution_replay_tests {
    use super::*;

    fn get_valid_dossier() -> SealedGovernanceDossier {
        SealedGovernanceDossier {
            proposal_id: ProposalId("PROP-EXEC-100".into()),
            proposal_hash: "blake3:hash_dossier_100".into(),
            approval_record: ApprovalRecord {
                proposal_id: ProposalId("PROP-EXEC-100".into()),
                approver_identity: "operator_lead".into(),
                granted_approval_level: ApprovalLevel::Operator,
                signature_hash: "SIG-OP-100".into(),
                timestamp: 1710000000,
            },
            capability_scope: CapabilityScope {
                target_resource: "urn:internal:service:registry".into(),
                operation_type: "QUARANTINE".into(),
            },
            sealed_timestamp: 1710000000,
            authority_expansion: 0,
        }
    }

    #[test]
    fn aer_01_full_pipeline_success() {
        let mut orchestrator = ExecutionOrchestrator::new();
        let dossier = get_valid_dossier();
        let token = orchestrator.mint_token(&dossier, 1710000010, 1).unwrap();

        let receipt = orchestrator
            .dispatch_and_execute(
                &token,
                &dossier.capability_scope,
                "blake3:1111111111111111111111111111111111111111111111111111111111111111",
                "blake3:2222222222222222222222222222222222222222222222222222222222222222",
                1710000020,
            )
            .unwrap();

        assert_eq!(receipt.disposition, ExecutionDisposition::Success);
        assert_eq!(receipt.residual_authority_retained, 0);
    }

    #[test]
    fn aer_02_execution_replay_attack_denied() {
        let mut orchestrator = ExecutionOrchestrator::new();
        let dossier = get_valid_dossier();
        let token = orchestrator.mint_token(&dossier, 1710000010, 1).unwrap();

        assert!(orchestrator
            .dispatch_and_execute(
                &token,
                &dossier.capability_scope,
                "blake3:1111",
                "blake3:2222",
                1710000020,
            )
            .is_ok());

        let replay = orchestrator.dispatch_and_execute(
            &token,
            &dossier.capability_scope,
            "blake3:1111",
            "blake3:2222",
            1710000025,
        );

        assert_eq!(replay, Err(PipelineError::ReplayDetected));
    }

    #[test]
    fn aer_03_scope_escalation_attack_denied() {
        let mut orchestrator = ExecutionOrchestrator::new();
        let dossier = get_valid_dossier();
        let token = orchestrator.mint_token(&dossier, 1710000010, 1).unwrap();

        let malicious_scope = CapabilityScope {
            target_resource: "urn:internal:service:registry".into(),
            operation_type: "ROOT_ADMIN_OVERRIDE".into(),
        };

        let res = orchestrator.dispatch_and_execute(
            &token,
            &malicious_scope,
            "blake3:1111",
            "blake3:2222",
            1710000020,
        );

        assert_eq!(res, Err(PipelineError::ScopeMismatch));
    }

    #[test]
    fn aer_04_expired_token_denied() {
        let mut orchestrator = ExecutionOrchestrator::new();
        let dossier = get_valid_dossier();
        let token = orchestrator.mint_token(&dossier, 1710000010, 1).unwrap();

        let res = orchestrator.dispatch_and_execute(
            &token,
            &dossier.capability_scope,
            "blake3:1111",
            "blake3:2222",
            1710000080,
        );

        assert_eq!(res, Err(PipelineError::TokenExpired));
    }

    #[test]
    fn aer_05_authority_expansion_dossier_rejected() {
        let orchestrator = ExecutionOrchestrator::new();
        let mut dossier = get_valid_dossier();
        dossier.authority_expansion = 1;

        let res = orchestrator.mint_token(&dossier, 1710000010, 1);
        assert_eq!(res, Err(PipelineError::AuthorityExpansionViolation));
    }
}
