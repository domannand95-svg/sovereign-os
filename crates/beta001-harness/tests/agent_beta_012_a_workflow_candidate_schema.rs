use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowCandidateValidationResult {
    Valid,
    Invalid(String),
}

pub struct WorkflowCandidateValidator;

impl WorkflowCandidateValidator {
    pub fn validate(value: &serde_json::Value) -> WorkflowCandidateValidationResult {
        // Enforce schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_GOVERNANCE_WORKFLOW_CANDIDATE-v1")
        {
            return WorkflowCandidateValidationResult::Invalid(
                "Invalid or missing schema_version".into(),
            );
        }

        // Validate workflow_candidate_id pattern
        if let Some(id) = value.get("workflow_candidate_id").and_then(|v| v.as_str()) {
            if !id.starts_with("wf_cand_") {
                return WorkflowCandidateValidationResult::Invalid(
                    "Invalid workflow_candidate_id format".into(),
                );
            }
        } else {
            return WorkflowCandidateValidationResult::Invalid(
                "Missing workflow_candidate_id".into(),
            );
        }

        // Validate participating_domains presence and non-empty requirement (TC-WORKFLOW-CAND-002)
        match value
            .get("participating_domains")
            .and_then(|v| v.as_array())
        {
            Some(domains) if !domains.is_empty() => (),
            _ => {
                return WorkflowCandidateValidationResult::Invalid(
                    "Missing or empty participating_domains".into(),
                )
            }
        }

        // Validate policy references version pinning (TC-WORKFLOW-CAND-003)
        if let Some(refs) = value.get("policy_references").and_then(|v| v.as_array()) {
            for r in refs {
                if r.get("policy_digest")
                    .and_then(|v| v.as_str())
                    .map_or(true, |d| !d.starts_with("sha256:"))
                {
                    return WorkflowCandidateValidationResult::Invalid(
                        "Floating or unpinned policy digest reference".into(),
                    );
                }
            }
        }

        // AUTHORITY & CREDENTIAL INJECTION REJECTION (TC-WORKFLOW-CAND-004, 005, 006)
        let allowed_keys = [
            "schema_version",
            "workflow_candidate_id",
            "participating_domains",
            "policy_references",
            "required_human_checkpoints",
            "coordination_intent",
            "created_at",
        ];

        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return WorkflowCandidateValidationResult::Invalid(format!(
                        "Authority injection or forbidden field detected: {}",
                        key
                    ));
                }
            }
        }

        if let Some(intent) = value.get("coordination_intent").and_then(|v| v.as_object()) {
            if intent.get("execute_deployment").and_then(|v| v.as_bool()) == Some(true)
                || intent.get("merge_repository").and_then(|v| v.as_bool()) == Some(true)
                || intent.get("grant_privilege").and_then(|v| v.as_bool()) == Some(true)
            {
                return WorkflowCandidateValidationResult::Invalid(
                    "Forbidden operational execution directive in intent".into(),
                );
            }
        }

        WorkflowCandidateValidationResult::Valid
    }
}

#[cfg(test)]
mod workflow_candidate_schema_tests {
    use super::*;

    fn get_valid_workflow_candidate() -> serde_json::Value {
        json!({
            "schema_version": "REPOSITORY_GOVERNANCE_WORKFLOW_CANDIDATE-v1",
            "workflow_candidate_id": "wf_cand_01XYZ",
            "participating_domains": ["PUBLICATION", "PULL_REQUEST", "REVIEW"],
            "policy_references": [
                {
                    "policy_id": "pol_01",
                    "policy_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                }
            ],
            "required_human_checkpoints": ["DEPLOYMENT_GATE"],
            "coordination_intent": {
                "description": "Coordinate validation cycle across publication and PR domains",
                "execute_deployment": false,
                "merge_repository": false
            },
            "created_at": "2026-08-20T07:00:00Z"
        })
    }

    #[test]
    fn tc_workflow_cand_001_valid_candidate_accepted() {
        let cand = get_valid_workflow_candidate();
        assert_eq!(
            WorkflowCandidateValidator::validate(&cand),
            WorkflowCandidateValidationResult::Valid
        );
    }

    #[test]
    fn tc_workflow_cand_002_missing_domains_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand.as_object_mut()
            .unwrap()
            .remove("participating_domains");
        assert!(matches!(
            WorkflowCandidateValidator::validate(&cand),
            WorkflowCandidateValidationResult::Invalid(_)
        ));

        let mut empty_domains = get_valid_workflow_candidate();
        empty_domains["participating_domains"] = json!([]);
        assert!(matches!(
            WorkflowCandidateValidator::validate(&empty_domains),
            WorkflowCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_workflow_cand_003_floating_policy_version_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand["policy_references"] = json!([{"policy_id": "pol_01", "policy_digest": "latest"}]);
        assert!(matches!(
            WorkflowCandidateValidator::validate(&cand),
            WorkflowCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_workflow_cand_004_authority_injection_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand.as_object_mut()
            .unwrap()
            .insert("grant_capability".to_string(), json!("root"));
        assert!(matches!(
            WorkflowCandidateValidator::validate(&cand),
            WorkflowCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_workflow_cand_005_credential_inclusion_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand.as_object_mut()
            .unwrap()
            .insert("token".to_string(), json!("secret_key_123"));
        assert!(matches!(
            WorkflowCandidateValidator::validate(&cand),
            WorkflowCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_workflow_cand_006_execution_directive_rejected() {
        let mut cand = get_valid_workflow_candidate();
        cand["coordination_intent"]["execute_deployment"] = json!(true);
        assert!(matches!(
            WorkflowCandidateValidator::validate(&cand),
            WorkflowCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_workflow_cand_007_inert_coordination_intent_verified() {
        let cand = get_valid_workflow_candidate();
        let res = WorkflowCandidateValidator::validate(&cand);
        assert_eq!(res, WorkflowCandidateValidationResult::Valid);
    }
}
