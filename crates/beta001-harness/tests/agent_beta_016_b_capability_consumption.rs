// ============================================================================
// AGENT-BETA-016-B: Capability Consumption & Single-Use Enforcement Boundary
// ============================================================================
// Invariant: Token Minted -> Single-Use Consumption -> Permanent Invalidation
// ============================================================================

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityScope {
    pub target_resource: String,
    pub operation_type: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumptionReceipt {
    pub receipt_id: String,
    pub token_id: String,
    pub consumed_at: u64,
    pub operation_executed: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConsumptionError {
    TokenExpired,
    TokenPremature,
    TokenAlreadyConsumed,
    ScopeMismatch,
    ProofMismatch,
}

pub struct CapabilityConsumptionLedger {
    consumed_tokens: HashSet<String>,
}

impl CapabilityConsumptionLedger {
    pub fn new() -> Self {
        Self {
            consumed_tokens: HashSet::new(),
        }
    }

    pub fn consume_token(
        &mut self,
        token: &EphemeralCapabilityToken,
        requested_scope: &CapabilityScope,
        current_timestamp: u64,
    ) -> Result<ConsumptionReceipt, ConsumptionError> {
        if current_timestamp < token.issued_at {
            return Err(ConsumptionError::TokenPremature);
        }
        if current_timestamp > token.expires_at {
            return Err(ConsumptionError::TokenExpired);
        }

        if self.consumed_tokens.contains(&token.token_id) {
            return Err(ConsumptionError::TokenAlreadyConsumed);
        }

        if token.capability_scope.target_resource != requested_scope.target_resource
            || token.capability_scope.operation_type != requested_scope.operation_type
        {
            return Err(ConsumptionError::ScopeMismatch);
        }

        if !token.cryptographic_proof.starts_with("blake3:") || token.cryptographic_proof.len() != 71 {
            return Err(ConsumptionError::ProofMismatch);
        }

        self.consumed_tokens.insert(token.token_id.clone());

        let receipt_id = format!("rcpt_{}_{}", token.token_id, current_timestamp);
        Ok(ConsumptionReceipt {
            receipt_id,
            token_id: token.token_id.clone(),
            consumed_at: current_timestamp,
            operation_executed: token.capability_scope.operation_type.clone(),
        })
    }
}

// ============================================================================
// CONSUMPTION VALIDATION SUITE (CON-01..05)
// ============================================================================

#[cfg(test)]
mod capability_consumption_tests {
    use super::*;

    fn get_valid_token() -> EphemeralCapabilityToken {
        EphemeralCapabilityToken {
            token_id: "cap_tok_0123456789abcdef".into(),
            proposal_id: ProposalId("PROP-EXEC-001".into()),
            capability_scope: CapabilityScope {
                target_resource: "urn:internal:entity:x".into(),
                operation_type: "QUARANTINE".into(),
            },
            issued_at: 1710000000,
            expires_at: 1710000060,
            nonce: 101,
            cryptographic_proof: "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            single_use: true,
        }
    }

    #[test]
    fn con_01_valid_single_consumption() {
        let mut ledger = CapabilityConsumptionLedger::new();
        let token = get_valid_token();
        let scope = token.capability_scope.clone();

        let receipt = ledger.consume_token(&token, &scope, 1710000020).unwrap();
        assert_eq!(receipt.token_id, token.token_id);
        assert_eq!(receipt.operation_executed, "QUARANTINE");
        assert_eq!(receipt.consumed_at, 1710000020);
    }

    #[test]
    fn con_02_replay_attack_denied() {
        let mut ledger = CapabilityConsumptionLedger::new();
        let token = get_valid_token();
        let scope = token.capability_scope.clone();

        assert!(ledger.consume_token(&token, &scope, 1710000020).is_ok());
        let replay = ledger.consume_token(&token, &scope, 1710000025);
        assert_eq!(replay, Err(ConsumptionError::TokenAlreadyConsumed));
    }

    #[test]
    fn con_03_expired_token_denied() {
        let mut ledger = CapabilityConsumptionLedger::new();
        let token = get_valid_token();
        let scope = token.capability_scope.clone();

        let res = ledger.consume_token(&token, &scope, 1710000070);
        assert_eq!(res, Err(ConsumptionError::TokenExpired));
    }

    #[test]
    fn con_04_scope_mismatch_denied() {
        let mut ledger = CapabilityConsumptionLedger::new();
        let token = get_valid_token();
        let mismatch_scope = CapabilityScope {
            target_resource: "urn:internal:entity:x".into(),
            operation_type: "MUTATE_STATE".into(),
        };

        let res = ledger.consume_token(&token, &mismatch_scope, 1710000020);
        assert_eq!(res, Err(ConsumptionError::ScopeMismatch));
    }

    #[test]
    fn con_05_tampered_proof_denied() {
        let mut ledger = CapabilityConsumptionLedger::new();
        let mut token = get_valid_token();
        token.cryptographic_proof = "invalid_proof".into();
        let scope = token.capability_scope.clone();

        let res = ledger.consume_token(&token, &scope, 1710000020);
        assert_eq!(res, Err(ConsumptionError::ProofMismatch));
    }
}
