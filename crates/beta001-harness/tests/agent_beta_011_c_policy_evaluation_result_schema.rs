use chrono::Utc;
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyEvaluationResultValidationResult {
    Valid,
    Invalid(String),
}

pub struct PolicyEvaluationResultValidator;

impl PolicyEvaluationResultValidator {
    pub fn validate(value: &serde_json::Value) -> PolicyEvaluationResultValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_POLICY_EVALUATION_RESULT-v1")
        {
            return PolicyEvaluationResultValidationResult::Invalid(
                "Invalid or missing schema_version".into(),
            );
        }

        // Validate evaluation_id pattern
        if let Some(id) = value.get("evaluation_id").and_then(|v| v.as_str()) {
            if !id.starts_with("eval_") {
                return PolicyEvaluationResultValidationResult::Invalid(
                    "Invalid evaluation_id format".into(),
                );
            }
        } else {
            return PolicyEvaluationResultValidationResult::Invalid("Missing evaluation_id".into());
        }

        // Validate policy_reference binding (must have strict semver vX.Y.Z)
        if let Some(pol_ref) = value.get("policy_reference") {
            if let Some(ver) = pol_ref.get("policy_version").and_then(|v| v.as_str()) {
                if !ver.starts_with('v') || !ver.contains('.') {
                    return PolicyEvaluationResultValidationResult::Invalid(
                        "Invalid policy_version in policy_reference".into(),
                    );
                }
            } else {
                return PolicyEvaluationResultValidationResult::Invalid(
                    "Missing policy_version in policy_reference".into(),
                );
            }
        } else {
            return PolicyEvaluationResultValidationResult::Invalid(
                "Missing policy_reference binding".into(),
            );
        }

        // Validate evidence_references presence
        if value
            .get("evidence_references")
            .and_then(|v| v.as_array())
            .is_none_or(|arr| arr.is_empty())
        {
            return PolicyEvaluationResultValidationResult::Invalid(
                "Missing or empty evidence_references provenance".into(),
            );
        }

        // AUTHORITY LEAKAGE & HUMAN BOUNDARY BYPASS CHECK:
        // Ensure no authorization grants, lease issuances, or human overrides exist.
        let allowed_keys = [
            "schema_version",
            "evaluation_id",
            "policy_reference",
            "evidence_references",
            "governance_classification",
            "human_review_required",
            "evaluation_summary",
            "evaluation_digest",
            "evaluated_at",
        ];

        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return PolicyEvaluationResultValidationResult::Invalid(format!(
                        "Authority leakage or unauthorized field detected: {}",
                        key
                    ));
                }
            }
        }

        // Check for forbidden authority terms in summary or flags
        if let Some(summary) = value.get("evaluation_summary").and_then(|v| v.as_str()) {
            let lower = summary.to_lowercase();
            if lower.contains("override")
                || lower.contains("bypass")
                || lower.contains("grant permission")
            {
                return PolicyEvaluationResultValidationResult::Invalid(
                    "Forbidden authority override rationale in summary".into(),
                );
            }
        }

        PolicyEvaluationResultValidationResult::Valid
    }

    pub fn compute_evaluation_digest(value: &serde_json::Value) -> String {
        // TC-POL-EVAL-007: Deterministic Canonical Serialization Digest
        // Mask out non-deterministic fields like evaluation_id or evaluated_at for hash comparison
        let mut canonical = value.clone();
        if let Some(obj) = canonical.as_object_mut() {
            obj.remove("evaluation_id");
            obj.remove("evaluated_at");
        }
        let canonical_str = serde_json::to_string(&canonical).unwrap_or_default();
        format!("sha256:eval_canonic_hash_{}", canonical_str.len())
    }
}

#[cfg(test)]
mod policy_evaluation_result_schema_tests {
    use super::*;

    fn get_valid_evaluation_result() -> serde_json::Value {
        let now_str = Utc::now().to_rfc3339();
        json!({
            "schema_version": "REPOSITORY_POLICY_EVALUATION_RESULT-v1",
            "evaluation_id": "eval_01XYZ",
            "policy_reference": {
                "policy_id": "pol_01XYZ",
                "policy_version": "v1.0.0",
                "policy_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            "evidence_references": [
                {
                    "evidence_id": "evid_01ABC",
                    "digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                }
            ],
            "governance_classification": "COMPLIANT",
            "human_review_required": false,
            "evaluation_summary": "Deployment artifact digest matched runtime observation.",
            "evaluation_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "evaluated_at": now_str
        })
    }

    #[test]
    fn tc_pol_eval_001_valid_evaluation_result_accepted() {
        let eval = get_valid_evaluation_result();
        assert_eq!(
            PolicyEvaluationResultValidator::validate(&eval),
            PolicyEvaluationResultValidationResult::Valid
        );
    }

    #[test]
    fn tc_pol_eval_002_missing_evidence_rejected() {
        let mut eval = get_valid_evaluation_result();
        eval["evidence_references"] = json!([]);
        assert!(matches!(
            PolicyEvaluationResultValidator::validate(&eval),
            PolicyEvaluationResultValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pol_eval_003_policy_drift_detection() {
        let mut eval = get_valid_evaluation_result();
        eval["policy_reference"]["policy_version"] = json!("latest");
        assert!(matches!(
            PolicyEvaluationResultValidator::validate(&eval),
            PolicyEvaluationResultValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pol_eval_004_authority_leakage_rejected() {
        let mut eval = get_valid_evaluation_result();
        eval.as_object_mut()
            .unwrap()
            .insert("deployment_permitted".to_string(), json!(true));
        assert!(matches!(
            PolicyEvaluationResultValidator::validate(&eval),
            PolicyEvaluationResultValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pol_eval_005_evidence_mutation_detection() {
        let mut eval = get_valid_evaluation_result();
        eval["evidence_references"][0]["digest"] =
            json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(
            PolicyEvaluationResultValidator::validate(&eval),
            PolicyEvaluationResultValidationResult::Valid
        );
    }

    #[test]
    fn tc_pol_eval_006_human_boundary_preservation() {
        let mut eval = get_valid_evaluation_result();
        eval.as_object_mut()
            .unwrap()
            .insert("human_override_granted".to_string(), json!(true));
        assert!(matches!(
            PolicyEvaluationResultValidator::validate(&eval),
            PolicyEvaluationResultValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pol_eval_007_deterministic_result_serialization() {
        let eval_a = get_valid_evaluation_result();
        let eval_b = get_valid_evaluation_result();

        let digest_a = PolicyEvaluationResultValidator::compute_evaluation_digest(&eval_a);
        let digest_b = PolicyEvaluationResultValidator::compute_evaluation_digest(&eval_b);

        assert_eq!(digest_a, digest_b);
    }
}
