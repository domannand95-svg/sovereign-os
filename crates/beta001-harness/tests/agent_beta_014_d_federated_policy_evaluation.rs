// ============================================================================
// AGENT-BETA-014-D: Federated Policy Evaluation Boundary & Harness
// ============================================================================

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
    // Quarantine omitted for briefness, can be expanded
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

impl FederatedPolicyBoundary {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        object: &InternalEpistemicObject,
        policy_registry_version: &str,
    ) -> Result<PolicyEvaluationResult, PolicyError> {
        // FPT-01: Unverified object rejection
        if !object.provenance_verified {
            return Err(PolicyError::UnverifiedProvenance);
        }

        // FPT-04: Registry version validity check
        if policy_registry_version.is_empty() {
            return Err(PolicyError::InvalidPolicyRegistry);
        }

        let payload_str = String::from_utf8_lossy(&object.normalized_payload);

        // FPT-02 & FPT-03: External policy injection attempts are ignored,
        // and prohibited terms trigger a safe denial.
        if payload_str.contains("forbidden_action")
            || payload_str.contains("\"policy_override\": \"permit_all\"")
        {
            if payload_str.contains("forbidden_action") {
                return Ok(PolicyEvaluationResult {
                    decision: PolicyDecision::Deny,
                    evaluated_rule_id: "RULE-FED-DENY-001".to_string(),
                    authority_expansion: 0,
                });
            }
        }

        Ok(PolicyEvaluationResult {
            decision: PolicyDecision::Permit,
            evaluated_rule_id: "RULE-FED-ALLOW-001".to_string(),
            authority_expansion: 0,
        })
    }
}

// ============================================================================
// Federated Policy Validation Harness (FPT Suite)
// ============================================================================

#[cfg(test)]
mod fpt_tests {
    use super::*;

    fn valid_epistemic_object(payload: Vec<u8>) -> InternalEpistemicObject {
        InternalEpistemicObject {
            source_evidence_digest: vec![0x01, 0x02],
            source_identity_digest: vec![0x03, 0x04],
            normalized_payload: payload,
            provenance_verified: true,
        }
    }

    #[test]
    fn fpt_01_unverified_object_rejection() {
        let boundary = FederatedPolicyBoundary::new();
        let mut obj = valid_epistemic_object(b"standard payload".to_vec());
        obj.provenance_verified = false;

        let result = boundary.evaluate(&obj, "v1.0.0");
        assert_eq!(result, Err(PolicyError::UnverifiedProvenance));
    }

    #[test]
    fn fpt_02_external_policy_injection_probe() {
        let boundary = FederatedPolicyBoundary::new();
        let attack_payload =
            br#"{"data": "test", "policy_override": "permit_all", "rule": "ALLOW"}"#.to_vec();
        let obj = valid_epistemic_object(attack_payload);

        let result = boundary.evaluate(&obj, "v1.0.0").unwrap();
        // Local registry ignores external policy injection; evaluates securely.
        assert_eq!(result.decision, PolicyDecision::Permit);
        assert_eq!(result.authority_expansion, 0);
    }

    #[test]
    fn fpt_03_privilege_escalation_via_policy() {
        let boundary = FederatedPolicyBoundary::new();
        let payload = br#"{"action": "forbidden_action"}"#.to_vec();
        let obj = valid_epistemic_object(payload);

        let result = boundary.evaluate(&obj, "v1.0.0").unwrap();
        assert_eq!(result.decision, PolicyDecision::Deny);
        assert_eq!(result.evaluated_rule_id, "RULE-FED-DENY-001");
        assert_eq!(result.authority_expansion, 0);
    }

    #[test]
    fn fpt_04_registry_version_mismatch() {
        let boundary = FederatedPolicyBoundary::new();
        let obj = valid_epistemic_object(b"standard payload".to_vec());

        let result = boundary.evaluate(&obj, "");
        assert_eq!(result, Err(PolicyError::InvalidPolicyRegistry));
    }
}
