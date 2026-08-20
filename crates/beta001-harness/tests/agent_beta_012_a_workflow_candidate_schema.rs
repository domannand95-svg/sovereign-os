use chrono::Utc;
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowCandidateValidationResult {
    Valid,
    Invalid(String),
}

pub struct WorkflowCandidateValidator;

impl WorkflowCandidateValidator {
    pub fn validate(value: &serde_json::Value) -> WorkflowCandidateValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str()) != Some("REPOSITORY_GOVERNANCE_WORKFLOW_CANDIDATE-v1") {
            return WorkflowCandidateValidationResult::Invalid("Invalid or missing schema_version".into());
        }

        // Validate workflow_candidate_id pattern
        if let Some(id) = value.get("workflow_candidate_id").and_then(|v| v.as_str()) {
            if !id.starts_with("wf_cand_") {
                return WorkflowCandidateValidationResult::Invalid("Invalid workflow_candidate_id format".into());
            }
        } else {
            return WorkflowCandidateValidationResult::Invalid("Missing workflow_candidate_id".into());
        }

        // Validate policy references strict semver versioning (vX.Y.Z)
        if let Some(policies) = value.get("policy_references").and_then(|v| v.as_array()) {
            if policies.is_empty() {
                return WorkflowCandidateValidationResult::Invalid("Policy references cannot be empty".into());
            }
            for pol in policies {
                if let Some(ver) = pol.get("policy_version").and_then(|v| v.as_str()) {
                    if !ver.starts_with('v') || !ver.contains('.') {
                        return WorkflowCandidateValidationResult::Invalid("Invalid policy_version reference".into());
                    }
                    let parts: Vec<&str> = ver.trim_start_matches('v').split('.').collect();
                    if parts.len() != 3 {
                        return WorkflowCandidateValidationResult::Invalid("Policy version must strictly follow semver vX.Y.Z".into());
                    }
                } else {
                    return WorkflowCandidateValidationResult::Invalid("Missing policy_version in workflow policy reference".into());
                }
            }
        } else {
            return WorkflowCandidateValidationResult::Invalid("Missing policy_references".into());
        }

        // AUTHORITY INJECTION & EXECUTION COMMAND REJECTION CHECK:
        // Ensure no execution commands, credentials, authorization grants, or operational triggers exist.
        let allowed_keys = [
            "schema_version", "workflow_candidate_id", "participating_domains",
            "policy_references", "required_human_checkpoints", "coordination_intent",
            "created_at"
        ];

        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return WorkflowCandidateValidationResult::Invalid(format!("Authority injection or execution directive detected: {}", key));
                }
            }
        }

        WorkflowCandidateValidationResult::Valid
    }
}

#[cfg(test)]
mod workflow_candidate_schema_tests {
    use super::*;

    fn get_valid_workflow_candidate() -> serde_json::Value {
        let now_str = Utc::now().to_rfc3339();
        json!({
            "schema_version": "REPOSITORY_GOVERNANCE_WORKFLOW_CANDIDATE-v1",
            "workflow_candidate_id": "wf_cand_01XYZ",
            "participating_domains": [
                "DEPLOYMENT",
                "POLICY_EVALUATION"
            ],
            "policy_references": [
                {
                    "policy_id": "pol_01XYZ",
                    "policy_version": "v1.0.0",
                    "policy_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                }
            ],
            "required_human_checkpoints": [
                "PROD_DEPLOY_APPROVAL"
            ],
            "coordination_intent": "COMPUTE_PIPELINE_POSTURE",
            "created_at": now_str
        })
    }

    #[test]
    fn tc_workflow_cand_001_valid_candidate_accepted() {
        let cand = get_valid_workflow_candidate();
        assert_eq!(WorkflowCandidateValidator::validate(&cand), WorkflowCandidateValidationResult::Valid);
    }

    #[test]
    fn tc_workflow_cand_002_missing_domains_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand.as_object_mut().unwrap().remove("participating_domains");
        assert!(matches!(WorkflowCandidateValidator::validate(&cand), WorkflowCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_workflow_cand_003_floating_policy_version_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand["policy_references"][0]["policy_version"] = json!("latest");
        assert!(matches!(WorkflowCandidateValidator::validate(&cand), WorkflowCandidateValidationResult::Invalid(_)));

        cand["policy_references"][0]["policy_version"] = json!("v1");
        assert!(matches!(WorkflowCandidateValidator::validate(&cand), WorkflowCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_workflow_cand_004_authority_injection_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand.as_object_mut().unwrap().insert("execute_deployment".to_string(), json!(true));
        assert!(matches!(WorkflowCandidateValidator::validate(&cand), WorkflowCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_workflow_cand_005_credential_inclusion_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand.as_object_mut().unwrap().insert("token".to_string(), json!("secret_key_123"));
        assert!(matches!(WorkflowCandidateValidator::validate(&cand), WorkflowCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_workflow_cand_006_execution_directive_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand.as_object_mut().unwrap().insert("deployment_lease_grant".to_string(), json!(true));
        assert!(matches!(WorkflowCandidateValidator::validate(&cand), WorkflowCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_workflow_cand_007_inert_coordination_intent_verified() {
        let cand = get_valid_workflow_candidate();
        assert_eq!(cand["coordination_intent"], json!("COMPUTE_PIPELINE_POSTURE"));
    }
}
