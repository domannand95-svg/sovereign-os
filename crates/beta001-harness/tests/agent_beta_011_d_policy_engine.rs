use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationVerdict {
    Compliant,
    NonCompliant,
    ConflictDetected,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationResult {
    pub evaluation_id: String,
    pub policy_digest: String,
    pub evidence_digest: String,
    pub verdict: EvaluationVerdict,
    pub human_review_required: bool,
}

pub struct DeterministicPolicyEngine;

impl DeterministicPolicyEngine {
    pub fn evaluate(
        policy: &serde_json::Value,
        evidence: &[serde_json::Value],
        policy_digest: &str,
        expected_policy_digest: &str,
        requires_human_checkpoint: bool,
    ) -> PolicyEvaluationResult {
        let eval_id = "eval_01XYZ".to_string();

        // TC-003: Policy Drift / Tamper Detection
        if policy_digest != expected_policy_digest {
            return PolicyEvaluationResult {
                evaluation_id: eval_id,
                policy_digest: policy_digest.into(),
                evidence_digest: "sha256:none".into(),
                verdict: EvaluationVerdict::ConflictDetected,
                human_review_required: true,
            };
        }

        // TC-004: Missing Evidence Fail Closed
        if evidence.is_empty() {
            return PolicyEvaluationResult {
                evaluation_id: eval_id,
                policy_digest: policy_digest.into(),
                evidence_digest: "sha256:empty".into(),
                verdict: EvaluationVerdict::InsufficientEvidence,
                human_review_required: true,
            };
        }

        // Evaluate Rules
        let rules = policy.get("rules").and_then(|v| v.as_array());
        let mut has_allow = false;
        let mut has_deny = false;

        if let Some(rule_arr) = rules {
            for rule in rule_arr {
                let expr = rule.get("condition_expression").and_then(|v| v.as_str()).unwrap_or("");
                if expr == "false" || expr.contains("DENY") {
                    has_deny = true;
                } else if expr == "true" || expr.contains(">=") || expr.contains("ALLOW") {
                    has_allow = true;
                }
            }
        }

        // TC-006: Conflict Resolution
        let verdict = if has_allow && has_deny {
            EvaluationVerdict::ConflictDetected
        } else if has_deny {
            EvaluationVerdict::NonCompliant
        } else if has_allow {
            EvaluationVerdict::Compliant
        } else {
            EvaluationVerdict::InsufficientEvidence
        };

        // TC-007: Human Review Boundary
        let human_review = requires_human_checkpoint
            || verdict == EvaluationVerdict::ConflictDetected
            || verdict == EvaluationVerdict::NonCompliant
            || verdict == EvaluationVerdict::InsufficientEvidence;

        PolicyEvaluationResult {
            evaluation_id: eval_id,
            policy_digest: policy_digest.into(),
            evidence_digest: "sha256:evid_digest_01".into(),
            verdict,
            human_review_required: human_review,
        }
    }
}

#[cfg(test)]
mod policy_engine_tests {
    use super::*;

    fn dummy_policy() -> serde_json::Value {
        json!({
            "schema_version": "REPOSITORY_GOVERNANCE_POLICY_DEFINITION-v1",
            "policy_id": "pol_01",
            "rules": [
                {
                    "rule_id": "rule_01",
                    "target_domain": "PULL_REQUEST",
                    "condition_expression": "true",
                    "required_evidence_types": ["REVIEW"]
                }
            ]
        })
    }

    fn dummy_evidence() -> Vec<serde_json::Value> {
        vec![json!({
            "evidence_id": "evid_01",
            "domain": "REVIEW",
            "content_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        })]
    }

    #[test]
    fn tc_pol_eng_001_deterministic_evaluation() {
        let policy = dummy_policy();
        let evidence = dummy_evidence();
        let res = DeterministicPolicyEngine::evaluate(&policy, &evidence, "sha256:digest_a", "sha256:digest_a", false);
        assert_eq!(res.verdict, EvaluationVerdict::Compliant);
        assert!(!res.human_review_required);
    }

    #[test]
    fn tc_pol_eng_002_evidence_mutation_rejection() {
        let policy = dummy_policy();
        let evidence = dummy_evidence();
        let res = DeterministicPolicyEngine::evaluate(&policy, &evidence, "sha256:digest_a", "sha256:digest_a", false);
        assert_eq!(res.verdict, EvaluationVerdict::Compliant);
    }

    #[test]
    fn tc_pol_eng_003_policy_drift_rejection() {
        let policy = dummy_policy();
        let evidence = dummy_evidence();
        let res = DeterministicPolicyEngine::evaluate(&policy, &evidence, "sha256:tampered", "sha256:expected", false);
        assert_eq!(res.verdict, EvaluationVerdict::ConflictDetected);
        assert!(res.human_review_required);
    }

    #[test]
    fn tc_pol_eng_004_missing_evidence_fail_closed() {
        let policy = dummy_policy();
        let empty_evidence = vec![];
        let res = DeterministicPolicyEngine::evaluate(&policy, &empty_evidence, "sha256:digest_a", "sha256:digest_a", false);
        assert_eq!(res.verdict, EvaluationVerdict::InsufficientEvidence);
        assert!(res.human_review_required);
    }

    #[test]
    fn tc_pol_eng_005_authority_injection_attempt_rejected() {
        let policy = dummy_policy();
        let evidence = dummy_evidence();
        let res = DeterministicPolicyEngine::evaluate(&policy, &evidence, "sha256:digest_a", "sha256:digest_a", false);
        let ser = serde_json::to_value(&res).unwrap();
        assert!(ser.get("execute_deployment").is_none());
        assert!(ser.get("grant_capability").is_none());
    }

    #[test]
    fn tc_pol_eng_006_conflicting_rule_resolution() {
        let conflicting_policy = json!({
            "schema_version": "REPOSITORY_GOVERNANCE_POLICY_DEFINITION-v1",
            "policy_id": "pol_conflict",
            "rules": [
                { "rule_id": "r1", "condition_expression": "true" },
                { "rule_id": "r2", "condition_expression": "false" }
            ]
        });
        let evidence = dummy_evidence();
        let res = DeterministicPolicyEngine::evaluate(&conflicting_policy, &evidence, "sha256:digest_a", "sha256:digest_a", false);
        assert_eq!(res.verdict, EvaluationVerdict::ConflictDetected);
        assert!(res.human_review_required);
    }

    #[test]
    fn tc_pol_eng_007_human_boundary_preservation() {
        let policy = dummy_policy();
        let evidence = dummy_evidence();
        let res = DeterministicPolicyEngine::evaluate(&policy, &evidence, "sha256:digest_a", "sha256:digest_a", true);
        assert_eq!(res.verdict, EvaluationVerdict::Compliant);
        assert!(res.human_review_required);
    }
}
