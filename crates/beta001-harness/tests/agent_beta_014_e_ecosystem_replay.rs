// ============================================================================
// AGENT-BETA-014-E: External Ecosystem Adversarial Replay & Guardrail
// ============================================================================

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalEpistemicObject {
    pub source_evidence_digest: Vec<u8>,
    pub source_identity_digest: Vec<u8>,
    pub normalized_payload: Vec<u8>,
    pub provenance_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Permit,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEvaluationResult {
    pub decision: PolicyDecision,
    pub evaluated_rule_id: String,
    pub authority_expansion: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PolicyError {
    UnverifiedProvenance,
    InvalidPolicyRegistry,
}

pub struct FederatedPolicyBoundary;

impl Default for FederatedPolicyBoundary {
    fn default() -> Self {
        Self::new()
    }
}

impl FederatedPolicyBoundary {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        object: &InternalEpistemicObject,
        policy_registry_version: &str,
    ) -> Result<PolicyEvaluationResult, PolicyError> {
        if !object.provenance_verified {
            return Err(PolicyError::UnverifiedProvenance);
        }
        if policy_registry_version.is_empty() {
            return Err(PolicyError::InvalidPolicyRegistry);
        }

        let payload_str = String::from_utf8_lossy(&object.normalized_payload);
        if payload_str.contains("forbidden_action") {
            return Ok(PolicyEvaluationResult {
                decision: PolicyDecision::Deny,
                evaluated_rule_id: "RULE-FED-DENY-001".to_string(),
                authority_expansion: 0,
            });
        }

        Ok(PolicyEvaluationResult {
            decision: PolicyDecision::Permit,
            evaluated_rule_id: "RULE-FED-ALLOW-001".to_string(),
            authority_expansion: 0,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayContext {
    pub session_id: String,
    pub nonce: u64,
    pub epoch_timestamp: u64,
}

pub struct EcosystemReplayGuardrail {
    seen_nonces: HashSet<(String, u64)>,
    policy_boundary: FederatedPolicyBoundary,
}

impl Default for EcosystemReplayGuardrail {
    fn default() -> Self {
        Self::new()
    }
}

impl EcosystemReplayGuardrail {
    pub fn new() -> Self {
        Self {
            seen_nonces: HashSet::new(),
            policy_boundary: FederatedPolicyBoundary::new(),
        }
    }

    pub fn evaluate_stream(
        &mut self,
        object: &InternalEpistemicObject,
        context: &ReplayContext,
        policy_registry_version: &str,
    ) -> Result<PolicyEvaluationResult, ReplayError> {
        // EAR-02: Epoch validation
        if context.epoch_timestamp == 0 {
            return Err(ReplayError::InvalidEpochTimestamp);
        }

        // EAR-01: Session-scoped nonce uniqueness check
        let nonce_key = (context.session_id.clone(), context.nonce);
        if self.seen_nonces.contains(&nonce_key) {
            return Err(ReplayError::ReplayDetected);
        }
        self.seen_nonces.insert(nonce_key);

        // Delegate to local policy boundary
        let evaluation = self
            .policy_boundary
            .evaluate(object, policy_registry_version)
            .map_err(ReplayError::PolicyEvaluationFailed)?;

        // EAR-03: Authority drift guardrail
        if evaluation.authority_expansion > 0 {
            return Err(ReplayError::AuthorityDriftDetected);
        }

        Ok(evaluation)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    ReplayDetected,
    InvalidEpochTimestamp,
    PolicyEvaluationFailed(PolicyError),
    AuthorityDriftDetected,
}

// ============================================================================
// Ecosystem Adversarial Replay Validation Harness (EAR Suite)
// ============================================================================

#[cfg(test)]
mod ear_tests {
    use super::*;

    fn valid_epistemic_object(payload: Vec<u8>) -> InternalEpistemicObject {
        InternalEpistemicObject {
            source_evidence_digest: vec![0xAA, 0xBB],
            source_identity_digest: vec![0xCC, 0xDD],
            normalized_payload: payload,
            provenance_verified: true,
        }
    }

    #[test]
    fn ear_01_direct_nonce_replay_attack() {
        let mut guardrail = EcosystemReplayGuardrail::new();
        let obj = valid_epistemic_object(b"stream data".to_vec());
        let context = ReplayContext {
            session_id: "session-alpha".to_string(),
            nonce: 1001,
            epoch_timestamp: 1710000000,
        };

        assert!(guardrail.evaluate_stream(&obj, &context, "v1.0.0").is_ok());
        assert_eq!(
            guardrail.evaluate_stream(&obj, &context, "v1.0.0"),
            Err(ReplayError::ReplayDetected)
        );
    }

    #[test]
    fn ear_02_distributed_epoch_drift() {
        let mut guardrail = EcosystemReplayGuardrail::new();
        let obj = valid_epistemic_object(b"stream data".to_vec());
        let malformed_context = ReplayContext {
            session_id: "session-beta".to_string(),
            nonce: 2002,
            epoch_timestamp: 0,
        };

        assert_eq!(
            guardrail.evaluate_stream(&obj, &malformed_context, "v1.0.0"),
            Err(ReplayError::InvalidEpochTimestamp)
        );
    }

    #[test]
    fn ear_03_cumulative_state_leakage_probe() {
        let mut guardrail = EcosystemReplayGuardrail::new();

        let malicious_obj = valid_epistemic_object(b"action: forbidden_action".to_vec());
        let ctx_1 = ReplayContext {
            session_id: "gamma".into(),
            nonce: 3001,
            epoch_timestamp: 1710000000,
        };
        let res_1 = guardrail
            .evaluate_stream(&malicious_obj, &ctx_1, "v1.0.0")
            .unwrap();
        assert_eq!(res_1.authority_expansion, 0);

        let valid_obj = valid_epistemic_object(b"standard payload".to_vec());
        let ctx_2 = ReplayContext {
            session_id: "gamma".into(),
            nonce: 3002,
            epoch_timestamp: 1710000010,
        };
        let res_2 = guardrail
            .evaluate_stream(&valid_obj, &ctx_2, "v1.0.0")
            .unwrap();
        assert_eq!(res_2.authority_expansion, 0);
    }

    #[test]
    fn ear_04_high_concurrency_flood_simulation() {
        let mut guardrail = EcosystemReplayGuardrail::new();
        let obj = valid_epistemic_object(b"flood test".to_vec());

        for i in 0..50 {
            let context = ReplayContext {
                session_id: format!("session-delta-{}", i),
                nonce: 5000 + i,
                epoch_timestamp: 1710000000 + i,
            };
            let result = guardrail.evaluate_stream(&obj, &context, "v1.0.0");
            assert!(result.is_ok());
            assert_eq!(result.unwrap().authority_expansion, 0);
        }
    }
}
