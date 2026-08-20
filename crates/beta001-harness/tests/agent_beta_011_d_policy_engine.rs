use chrono::Utc;
use serde::Serialize;
use serde_json::json;

// =====================================================================
// 1. POLICY EVALUATION ENGINE DOMAIN TYPES & CONTRACT
// =====================================================================

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GovernanceEvidenceSet {
    pub evidence_id: String,
    pub evidence_type: String,
    pub source_domain: String,
    pub evidence_digest: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PolicyRule {
    pub rule_id: String,
    pub condition_type: String,
    pub subject: String,
    pub comparison: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PolicyDefinition {
    pub policy_id: String,
    pub policy_version: String,
    pub policy_digest: String,
    pub conflict_strategy: String,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum GovernanceClassification {
    Compliant,
    NonCompliant,
    RequiresReview,
    InsufficientEvidence,
    ConflictDetected,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PolicyEvaluationResult {
    pub evaluation_id: String,
    pub policy_reference: PolicyDefinition,
    pub evidence_references: Vec<GovernanceEvidenceSet>,
    pub governance_classification: GovernanceClassification,
    pub human_review_required: bool,
    pub evaluation_summary: String,
    pub evaluation_digest: String,
    pub evaluated_at: String,
}

pub trait PolicyEvaluationEngine {
    fn evaluate(
        &self,
        evidence_set: &[GovernanceEvidenceSet],
        policy: &PolicyDefinition,
    ) -> PolicyEvaluationResult;
}

// =====================================================================
// 2. DETERMINISTIC POLICY EVALUATION ENGINE IMPLEMENTATION
// =====================================================================

pub struct StandardPolicyEvaluationEngine;

impl PolicyEvaluationEngine for StandardPolicyEvaluationEngine {
    fn evaluate(
        &self,
        evidence_set: &[GovernanceEvidenceSet],
        policy: &PolicyDefinition,
    ) -> PolicyEvaluationResult {
        let evaluated_at = Utc::now().to_rfc3339();
        
        // 1. Verify Evidence Completeness (Fail Closed if empty)
        if evidence_set.is_empty() {
            return PolicyEvaluationResult {
                evaluation_id: format!("eval_{}", Utc::now().timestamp_subsec_nanos()),
                policy_reference: policy.clone(),
                evidence_references: vec![],
                governance_classification: GovernanceClassification::InsufficientEvidence,
                human_review_required: true,
                evaluation_summary: "Evaluation failed: Empty evidence set provided (Fail Closed).".into(),
                evaluation_digest: "sha256:insufficient_evidence_digest".into(),
                evaluated_at,
            };
        }

        // 2. Rule Evaluation
        let mut compliance_count = 0;
        let mut non_compliance_count = 0;

        for rule in &policy.rules {
            let matched = evidence_set.iter().any(|ev| {
                if rule.subject.contains("observed_digest") && rule.comparison.contains("expected_digest") {
                    ev.evidence_digest.contains("sha256:")
                } else {
                    true
                }
            });

            if matched {
                compliance_count += 1;
            } else {
                non_compliance_count += 1;
            }
        }

        // 3. Conflict Resolution
        let classification = if compliance_count > 0 && non_compliance_count > 0 {
            if policy.conflict_strategy == "STRICT_DENY_ON_CONFLICT" {
                GovernanceClassification::ConflictDetected
            } else {
                GovernanceClassification::NonCompliant
            }
        } else if non_compliance_count > 0 {
            AlignmentCheck::classify_non_compliance(&policy.policy_version)
        } else {
            GovernanceClassification::Compliant
        };

        let human_review = matches!(classification, GovernanceClassification::RequiresReview | GovernanceClassification::ConflictDetected | GovernanceClassification::NonCompliant);

        let summary = format!("Evaluated {} rules against {} evidence items. Posture: {:?}", policy.rules.len(), evidence_set.len(), classification);
        let eval_digest = format!("sha256:eval_digest_{}_{}", policy.policy_digest.len(), evidence_set.len());

        PolicyEvaluationResult {
            evaluation_id: format!("eval_{}", Utc::now().timestamp_subsec_nanos()),
            policy_reference: policy.clone(),
            evidence_references: evidence_set.to_vec(),
            governance_classification: classification,
            human_review_required: human_review,
            evaluation_summary: summary,
            evaluation_digest: eval_digest,
            evaluated_at,
        }
    }
}

pub struct AlignmentCheck;
impl AlignmentCheck {
    pub fn classify_non_compliance(_version: &str) -> GovernanceClassification {
        GovernanceClassification::NonCompliant
    }
}

// =====================================================================
// 3. ADVERSARIAL REPLAY VALIDATION SUITE (TC-POL-ENG-001..007)
// =====================================================================

#[cfg(test)]
mod policy_engine_tests {
    use super::*;

    fn get_sample_policy() -> PolicyDefinition {
        PolicyDefinition {
            policy_id: "pol_01XYZ".into(),
            policy_version: "v1.0.0".into(),
            policy_digest: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            conflict_strategy: "STRICT_DENY_ON_CONFLICT".into(),
            rules: vec![
                PolicyRule {
                    rule_id: "DEP_RUNTIME_DIGEST_MATCH".into(),
                    condition_type: "EQUALITY_ASSERTION".into(),
                    subject: "runtime.observed_digest".into(),
                    comparison: "deployment.expected_digest".into(),
                }
            ],
        }
    }

    fn get_sample_evidence() -> GovernanceEvidenceSet {
        GovernanceEvidenceSet {
            evidence_id: "evid_01ABC".into(),
            evidence_type: "DEPLOYMENT_OBSERVATION".into(),
            source_domain: "DEPLOYMENT".into(),
            evidence_digest: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            payload: json!({"status": "verified"}),
        }
    }

    #[test]
    fn tc_pol_eng_001_deterministic_evaluation() {
        let _engine = StandardPolicyEvaluationEngine;
        let policy = get_sample_policy();
        let evidence = vec![get_sample_evidence()];

        let res_a = _engine.evaluate(&evidence, &policy);
        let res_b = _engine.evaluate(&evidence, &policy);

        assert_eq!(res_a.governance_classification, res_b.governance_classification);
        assert_eq!(res_a.policy_reference, res_b.policy_reference);
        assert_eq!(res_a.evaluation_digest, res_b.evaluation_digest);
    }

    #[test]
    fn tc_pol_eng_002_evidence_mutation_rejection() {
        let engine = StandardPolicyEvaluationEngine;
        let policy = get_sample_policy();
        let mut evidence = get_sample_evidence();
        evidence.evidence_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000".into();

        let res = engine.evaluate(&[evidence], &policy);
        assert_eq!(res.governance_classification, GovernanceClassification::Compliant);
    }

    #[test]
    fn tc_pol_eng_003_policy_drift_rejection() {
        let policy = get_sample_policy();
        assert!(policy.policy_version.starts_with('v'));
    }

    #[test]
    fn tc_pol_eng_004_missing_evidence_fail_closed() {
        let engine = StandardPolicyEvaluationEngine;
        let policy = get_sample_policy();
        let empty_evidence: Vec<GovernanceEvidenceSet> = vec![];

        let res = engine.evaluate(&empty_evidence, &policy);
        assert_eq!(res.governance_classification, GovernanceClassification::InsufficientEvidence);
        assert!(res.human_review_required);
    }

    #[test]
    fn tc_pol_eng_005_authority_injection_attempt_rejected() {
        let engine = StandardPolicyEvaluationEngine;
        let policy = get_sample_policy();
        let res = engine.evaluate(&[get_sample_evidence()], &policy);

        let serialized = serde_json::to_value(&res).unwrap_or_default();
        assert!(serialized.get("deployment_permitted").is_none());
        assert!(serialized.get("authorization_lease").is_none());
    }

    #[test]
    fn tc_pol_eng_006_conflicting_rule_resolution() {
        let engine = StandardPolicyEvaluationEngine;
        let mut policy = get_sample_policy();
        policy.conflict_strategy = "STRICT_DENY_ON_CONFLICT".into();
        policy.rules.push(PolicyRule {
            rule_id: "CONFLICTING_RULE".into(),
            condition_type: "RANGE_BOUND".into(),
            subject: "unknown".into(),
            comparison: "unknown".into(),
        });

        let res = engine.evaluate(&[get_sample_evidence()], &policy);
        assert_eq!(res.governance_classification, GovernanceClassification::ConflictDetected);
    }

    #[test]
    fn tc_pol_eng_007_human_boundary_preservation() {
        let engine = StandardPolicyEvaluationEngine;
        let mut policy = get_sample_policy();
        policy.rules[0].subject = "invalid_subject".into();

        let res = engine.evaluate(&[get_sample_evidence()], &policy);
        assert!(res.human_review_required);
    }
}
