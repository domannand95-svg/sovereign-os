use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum MergeCandidateValidationResult {
    Valid,
    Invalid(String),
}

pub struct MergeCandidateValidator;

impl MergeCandidateValidator {
    pub fn validate(value: &serde_json::Value) -> MergeCandidateValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_MERGE_CANDIDATE-v1")
        {
            return MergeCandidateValidationResult::Invalid(
                "Invalid or missing schema_version".into(),
            );
        }

        // Validate merge_candidate_id pattern
        if let Some(id) = value.get("merge_candidate_id").and_then(|v| v.as_str()) {
            if !id.starts_with("mrg_cand_") {
                return MergeCandidateValidationResult::Invalid(
                    "Invalid merge_candidate_id format".into(),
                );
            }
        } else {
            return MergeCandidateValidationResult::Invalid("Missing merge_candidate_id".into());
        }

        // Validate pull_request_binding presence
        if value.get("pull_request_binding").is_none() {
            return MergeCandidateValidationResult::Invalid("Missing pull_request_binding".into());
        }

        // Validate expected_prestate_oid presence and format
        if let Some(oid) = value.get("expected_prestate_oid").and_then(|v| v.as_str()) {
            if oid.len() != 40 {
                return MergeCandidateValidationResult::Invalid(
                    "Invalid expected_prestate_oid length".into(),
                );
            }
        } else {
            return MergeCandidateValidationResult::Invalid("Missing expected_prestate_oid".into());
        }

        // Validate declared_strategy enum
        if let Some(strat) = value.get("declared_strategy").and_then(|v| v.as_str()) {
            let valid_strategies = ["MERGE_COMMIT", "SQUASH_MERGE", "REBASE_MERGE"];
            if !valid_strategies.contains(&strat) {
                return MergeCandidateValidationResult::Invalid(
                    "Invalid declared_strategy value".into(),
                );
            }
        } else {
            return MergeCandidateValidationResult::Invalid("Missing declared_strategy".into());
        }

        // Injected Authority Check: Ensure no implicit execution or deployment triggers exist
        let allowed_keys = [
            "schema_version",
            "merge_candidate_id",
            "pull_request_binding",
            "target_repository_identity",
            "source_ref",
            "target_ref",
            "expected_prestate_oid",
            "declared_strategy",
        ];
        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return MergeCandidateValidationResult::Invalid(format!(
                        "Injected unauthorized authority field detected: {}",
                        key
                    ));
                }
            }
        }

        MergeCandidateValidationResult::Valid
    }
}

#[cfg(test)]
mod merge_candidate_schema_tests {
    use super::*;

    fn get_valid_merge_candidate() -> serde_json::Value {
        json!({
            "schema_version": "REPOSITORY_MERGE_CANDIDATE-v1",
            "merge_candidate_id": "mrg_cand_01XYZ",
            "pull_request_binding": {
                "candidate_id": "pr_cand_01JXYZ",
                "source_commit_oid": "e9aeb73000000000000000000000000000000000"
            },
            "target_repository_identity": {
                "provider": "github.com",
                "repository_id": "repo_123",
                "owner": "org-sovereign",
                "name": "core-os"
            },
            "source_ref": "refs/heads/feature-xyz",
            "target_ref": "refs/heads/develop",
            "expected_prestate_oid": "b561857000000000000000000000000000000000",
            "declared_strategy": "SQUASH_MERGE"
        })
    }

    #[test]
    fn tc_mrg_cand_001_valid_candidate_accepted() {
        let cand = get_valid_merge_candidate();
        assert_eq!(
            MergeCandidateValidator::validate(&cand),
            MergeCandidateValidationResult::Valid
        );
    }

    #[test]
    fn tc_mrg_cand_002_missing_pr_binding_rejected() {
        let mut cand = get_valid_merge_candidate();
        cand.as_object_mut().unwrap().remove("pull_request_binding");
        assert!(matches!(
            MergeCandidateValidator::validate(&cand),
            MergeCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_mrg_cand_003_invalid_strategy_rejected() {
        let mut cand = get_valid_merge_candidate();
        cand["declared_strategy"] = json!("PROVIDER_DEFAULT_AUTO");
        assert!(matches!(
            MergeCandidateValidator::validate(&cand),
            MergeCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_mrg_cand_004_injected_execution_trigger_rejected() {
        let mut cand = get_valid_merge_candidate();
        cand.as_object_mut()
            .unwrap()
            .insert("trigger_deployment".to_string(), json!(true));
        assert!(matches!(
            MergeCandidateValidator::validate(&cand),
            MergeCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_mrg_cand_005_invalid_candidate_id_format_rejected() {
        let mut cand = get_valid_merge_candidate();
        cand["merge_candidate_id"] = json!("malicious_id_123");
        assert!(matches!(
            MergeCandidateValidator::validate(&cand),
            MergeCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_mrg_cand_006_missing_prestate_oid_rejected() {
        let mut cand = get_valid_merge_candidate();
        cand.as_object_mut()
            .unwrap()
            .remove("expected_prestate_oid");
        assert!(matches!(
            MergeCandidateValidator::validate(&cand),
            MergeCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_mrg_cand_007_merge_commit_strategy_valid() {
        let mut cand = get_valid_merge_candidate();
        cand["declared_strategy"] = json!("MERGE_COMMIT");
        assert_eq!(
            MergeCandidateValidator::validate(&cand),
            MergeCandidateValidationResult::Valid
        );
    }
}
