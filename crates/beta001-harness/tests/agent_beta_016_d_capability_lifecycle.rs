// ============================================================================
// AGENT-BETA-016-D: Token Revocation & Consumption Lifecycle
// ============================================================================
// Invariant: Minted -> Consumed | Revoked (Terminal States Cannot Regress)
// ============================================================================

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenLifecycleState {
    Minted,
    Consumed { consumed_at: u64, receipt_id: String },
    Revoked { revoked_at: u64, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityLifecycleRecord {
    pub token_id: String,
    pub state: TokenLifecycleState,
    pub updated_at: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LifecycleError {
    TokenNotFound,
    IllegalStateTransition,
    TokenAlreadyTerminal,
    TemporalInversion,
}

pub struct CapabilityLifecycleEngine {
    records: HashMap<String, CapabilityLifecycleRecord>,
}

impl CapabilityLifecycleEngine {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    pub fn register_minted_token(&mut self, token_id: String, issued_at: u64) {
        self.records.insert(
            token_id.clone(),
            CapabilityLifecycleRecord {
                token_id,
                state: TokenLifecycleState::Minted,
                updated_at: issued_at,
            },
        );
    }

    pub fn consume_token(
        &mut self,
        token_id: &str,
        receipt_id: String,
        consumed_at: u64,
    ) -> Result<(), LifecycleError> {
        let record = self.records.get_mut(token_id).ok_or(LifecycleError::TokenNotFound)?;

        if consumed_at < record.updated_at {
            return Err(LifecycleError::TemporalInversion);
        }

        match record.state {
            TokenLifecycleState::Minted => {
                record.state = TokenLifecycleState::Consumed {
                    consumed_at,
                    receipt_id,
                };
                record.updated_at = consumed_at;
                Ok(())
            }
            _ => Err(LifecycleError::TokenAlreadyTerminal),
        }
    }

    pub fn revoke_token(
        &mut self,
        token_id: &str,
        reason: String,
        revoked_at: u64,
    ) -> Result<(), LifecycleError> {
        let record = self.records.get_mut(token_id).ok_or(LifecycleError::TokenNotFound)?;

        if revoked_at < record.updated_at {
            return Err(LifecycleError::TemporalInversion);
        }

        match record.state {
            TokenLifecycleState::Minted => {
                record.state = TokenLifecycleState::Revoked {
                    revoked_at,
                    reason,
                };
                record.updated_at = revoked_at;
                Ok(())
            }
            _ => Err(LifecycleError::TokenAlreadyTerminal),
        }
    }

    pub fn get_state(&self, token_id: &str) -> Option<&TokenLifecycleState> {
        self.records.get(token_id).map(|r| &r.state)
    }
}

// ============================================================================
// LIFECYCLE VALIDATION SUITE (LIF-01..05)
// ============================================================================

#[cfg(test)]
mod capability_lifecycle_tests {
    use super::*;

    #[test]
    fn lif_01_mint_to_consumption_flow() {
        let mut engine = CapabilityLifecycleEngine::new();
        engine.register_minted_token("cap_tok_01".into(), 1710000000);

        assert_eq!(engine.get_state("cap_tok_01"), Some(&TokenLifecycleState::Minted));

        let res = engine.consume_token("cap_tok_01", "rcpt_01".into(), 1710000020);
        assert!(res.is_ok());

        assert!(matches!(
            engine.get_state("cap_tok_01"),
            Some(TokenLifecycleState::Consumed { .. })
        ));
    }

    #[test]
    fn lif_02_mint_to_revocation_flow() {
        let mut engine = CapabilityLifecycleEngine::new();
        engine.register_minted_token("cap_tok_02".into(), 1710000000);

        let res = engine.revoke_token("cap_tok_02", "Timeout occurred".into(), 1710000030);
        assert!(res.is_ok());

        assert!(matches!(
            engine.get_state("cap_tok_02"),
            Some(TokenLifecycleState::Revoked { .. })
        ));
    }

    #[test]
    fn lif_03_consumed_token_cannot_be_revoked() {
        let mut engine = CapabilityLifecycleEngine::new();
        engine.register_minted_token("cap_tok_03".into(), 1710000000);
        engine.consume_token("cap_tok_03", "rcpt_03".into(), 1710000010).unwrap();

        let res = engine.revoke_token("cap_tok_03", "Attempt revoke".into(), 1710000020);
        assert_eq!(res, Err(LifecycleError::TokenAlreadyTerminal));
    }

    #[test]
    fn lif_04_revoked_token_cannot_be_consumed() {
        let mut engine = CapabilityLifecycleEngine::new();
        engine.register_minted_token("cap_tok_04".into(), 1710000000);
        engine.revoke_token("cap_tok_04", "Security alert".into(), 1710000010).unwrap();

        let res = engine.consume_token("cap_tok_04", "rcpt_04".into(), 1710000020);
        assert_eq!(res, Err(LifecycleError::TokenAlreadyTerminal));
    }

    #[test]
    fn lif_05_temporal_inversion_rejection() {
        let mut engine = CapabilityLifecycleEngine::new();
        engine.register_minted_token("cap_tok_05".into(), 1710000050);

        let res = engine.consume_token("cap_tok_05", "rcpt_05".into(), 1710000010);
        assert_eq!(res, Err(LifecycleError::TemporalInversion));
    }
}
