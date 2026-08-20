use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentCandidateValidationResult {
    Valid,
    Invalid(String),
}

pub struct DeploymentCandidateValidator;

impl DeploymentCandidateValidator {
    pub fn validate(value: &serde_json::Value) -> DeploymentCandidateValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_DEPLOYMENT_CANDIDATE-v1")
        {
            return DeploymentCandidateValidationResult::Invalid(
                "Invalid or missing schema_version".into(),
            );
        }

        // Validate deployment_candidate_id pattern
        if let Some(id) = value
            .get("deployment_candidate_id")
            .and_then(|v| v.as_str())
        {
            if !id.starts_with("dep_cand_") {
                return DeploymentCandidateValidationResult::Invalid(
                    "Invalid deployment_candidate_id format".into(),
                );
            }
        } else {
            return DeploymentCandidateValidationResult::Invalid(
                "Missing deployment_candidate_id".into(),
            );
        }

        // Validate immutable artifact digest format (must be sha256:...)
        if let Some(digest) = value.get("source_artifact_digest").and_then(|v| v.as_str()) {
            if !digest.starts_with("sha256:") || digest.len() != 71 {
                return DeploymentCandidateValidationResult::Invalid(
                    "Invalid or mutable source_artifact_digest reference (must be sha256 hex)"
                        .into(),
                );
            }
        } else {
            return DeploymentCandidateValidationResult::Invalid(
                "Missing source_artifact_digest".into(),
            );
        }

        // Validate target_runtime_identity environment enum
        if let Some(runtime_id) = value.get("target_runtime_identity") {
            if let Some(env) = runtime_id.get("environment").and_then(|v| v.as_str()) {
                let valid_envs = ["development", "staging", "production"];
                if !valid_envs.contains(&env) {
                    return DeploymentCandidateValidationResult::Invalid(
                        "Invalid environment value".into(),
                    );
                }
            } else {
                return DeploymentCandidateValidationResult::Invalid(
                    "Missing environment in target_runtime_identity".into(),
                );
            }
        } else {
            return DeploymentCandidateValidationResult::Invalid(
                "Missing target_runtime_identity".into(),
            );
        }

        // Validate deployment_strategy enum
        if let Some(strat) = value.get("deployment_strategy").and_then(|v| v.as_str()) {
            let valid_strategies = ["ROLLING", "BLUE_GREEN", "RECREATE"];
            if !valid_strategies.contains(&strat) {
                return DeploymentCandidateValidationResult::Invalid(
                    "Invalid deployment_strategy value".into(),
                );
            }
        } else {
            return DeploymentCandidateValidationResult::Invalid(
                "Missing deployment_strategy".into(),
            );
        }

        // Injected Authority Check: Ensure no implicit credentials or side-effect flags exist
        let allowed_keys = [
            "schema_version",
            "deployment_candidate_id",
            "source_artifact_digest",
            "repository_identity",
            "source_commit_oid",
            "target_runtime_identity",
            "deployment_strategy",
            "expected_runtime_state",
            "proposed_runtime_state",
        ];
        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return DeploymentCandidateValidationResult::Invalid(format!(
                        "Injected unauthorized deployment authority field detected: {}",
                        key
                    ));
                }
            }
        }

        DeploymentCandidateValidationResult::Valid
    }
}

#[cfg(test)]
mod deployment_candidate_schema_tests {
    use super::*;

    fn get_valid_deployment_candidate() -> serde_json::Value {
        json!({
            "schema_version": "REPOSITORY_DEPLOYMENT_CANDIDATE-v1",
            "deployment_candidate_id": "dep_cand_01XYZ",
            "source_artifact_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "repository_identity": {
                "provider": "github.com",
                "repository_id": "repo_123",
                "owner": "org-sovereign",
                "name": "core-os"
            },
            "source_commit_oid": "e9aeb73000000000000000000000000000000000",
            "target_runtime_identity": {
                "provider": "aws",
                "runtime_id": "cluster-us-east-1",
                "environment": "staging"
            },
            "deployment_strategy": "ROLLING",
            "expected_runtime_state": "v1.0.0",
            "proposed_runtime_state": "v1.1.0"
        })
    }

    #[test]
    fn tc_dep_cand_001_valid_candidate_accepted() {
        let cand = get_valid_deployment_candidate();
        assert_eq!(
            DeploymentCandidateValidator::validate(&cand),
            DeploymentCandidateValidationResult::Valid
        );
    }

    #[test]
    fn tc_dep_cand_002_missing_artifact_digest_rejected() {
        let mut cand = get_valid_deployment_candidate();
        cand.as_object_mut()
            .unwrap()
            .remove("source_artifact_digest");
        assert!(matches!(
            DeploymentCandidateValidator::validate(&cand),
            DeploymentCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_dep_cand_003_mutable_artifact_reference_rejected() {
        let mut cand = get_valid_deployment_candidate();
        cand["source_artifact_digest"] = json!("latest");
        assert!(matches!(
            DeploymentCandidateValidator::validate(&cand),
            DeploymentCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_dep_cand_004_invalid_environment_rejected() {
        let mut cand = get_valid_deployment_candidate();
        cand["target_runtime_identity"]["environment"] = json!("unknown_env");
        assert!(matches!(
            DeploymentCandidateValidator::validate(&cand),
            DeploymentCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_dep_cand_005_credential_injection_rejected() {
        let mut cand = get_valid_deployment_candidate();
        cand.as_object_mut()
            .unwrap()
            .insert("token".to_string(), json!("secret_key_123"));
        assert!(matches!(
            DeploymentCandidateValidator::validate(&cand),
            DeploymentCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_dep_cand_006_deployment_side_effect_escalation_rejected() {
        let mut cand = get_valid_deployment_candidate();
        cand.as_object_mut()
            .unwrap()
            .insert("publish_release".to_string(), json!(true));
        assert!(matches!(
            DeploymentCandidateValidator::validate(&cand),
            DeploymentCandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_dep_cand_007_unknown_deployment_strategy_rejected() {
        let mut cand = get_valid_deployment_candidate();
        cand["deployment_strategy"] = json!("MAGIC_DEPLOY");
        assert!(matches!(
            DeploymentCandidateValidator::validate(&cand),
            DeploymentCandidateValidationResult::Invalid(_)
        ));
    }
}
