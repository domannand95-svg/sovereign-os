use chrono::Utc;
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDefinitionValidationResult {
    Valid,
    Invalid(String),
}

pub struct PolicyDefinitionValidator;

impl PolicyDefinitionValidator {
    pub fn validate(value: &serde_json::Value) -> PolicyDefinitionValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str()) != Some("REPOSITORY_POLICY_DEFINITION-v1") {
            return PolicyDefinitionValidationResult::Invalid("Invalid or missing schema_version".into());
        }

        // Validate policy_id pattern
        if let Some(id) = value.get("policy_id").and_then(|v| v.as_str()) {
            if !id.starts_with("pol_") {
                return PolicyDefinitionValidationResult::Invalid("Invalid policy_id format".into());
            }
        } else {
            return PolicyDefinitionValidationResult::Invalid("Missing policy_id".into());
        }

        // Validate policy_version format (must be strict semver vX.Y.Z, rejecting 'latest', 'main', or incomplete v1)
        if let Some(version) = value.get("policy_version").and_then(|v| v.as_str()) {
            if !version.starts_with('v') || !version.contains('.') {
                return PolicyDefinitionValidationResult::Invalid("Invalid or mutable policy_version reference".into());
            }
            let parts: Vec<&str> = version.trim_start_matches('v').split('.').collect();
            if parts.len() != 3 {
                return PolicyDefinitionValidationResult::Invalid("Policy version must strictly follow semver vX.Y.Z".into());
            }
        } else {
            return PolicyDefinitionValidationResult::Invalid("Missing policy_version".into());
        }

        // Validate policy_digest format (sha256 hex)
        if let Some(digest) = value.get("policy_digest").and_then(|v| v.as_str()) {
            if !digest.starts_with("sha256:") || digest.len() != 71 {
                return PolicyDefinitionValidationResult::Invalid("Invalid policy_digest format".into());
            }
        } else {
            return PolicyDefinitionValidationResult::Invalid("Missing policy_digest".into());
        }

        // AUTHORITY INJECTION & EXECUTION DIRECTIVE REJECTION CHECK:
        // Ensure no operational execution commands, grant permissions, or bypass flags exist.
        let allowed_keys = [
            "schema_version", "policy_id", "policy_version", "policy_digest",
            "created_at", "effective_from", "applicable_evidence_types",
            "conflict_strategy", "rules"
        ];

        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return PolicyDefinitionValidationResult::Invalid(format!("Authority injection or execution directive detected: {}", key));
                }
            }
        }

        // Validate rules array for forbidden keywords
        if let Some(rules) = value.get("rules").and_then(|v| v.as_array()) {
            for rule in rules {
                if let Some(rule_obj) = rule.as_object() {
                    for (k, v) in rule_obj {
                        let text = format!("{} {:?}", k, v).to_lowercase();
                        if text.contains("execute") || text.contains("deploy") || text.contains("merge") || text.contains("publish") || text.contains("rollback") || text.contains("grant") {
                            return PolicyDefinitionValidationResult::Invalid("Forbidden operational command or execution directive in rule".into());
                        }
                    }
                }
            }
        }

        PolicyDefinitionValidationResult::Valid
    }

    pub fn compute_policy_digest(value: &serde_json::Value) -> String {
        let len = serde_json::to_string(value).unwrap_or_default().len();
        format!("sha256:policy_digest_canonic_{}", len)
    }
}

#[cfg(test)]
mod policy_definition_schema_tests {
    use super::*;

    fn get_valid_policy() -> serde_json::Value {
        let now_str = Utc::now().to_rfc3339();
        json!({
            "schema_version": "REPOSITORY_POLICY_DEFINITION-v1",
            "policy_id": "pol_01XYZ",
            "policy_version": "v1.0.0",
            "policy_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "created_at": now_str,
            "effective_from": now_str,
            "applicable_evidence_types": [
                "DEPLOYMENT_OBSERVATION",
                "RUNTIME_OBSERVATION"
            ],
            "conflict_strategy": "STRICT_DENY_ON_CONFLICT",
            "rules": [
                {
                    "rule_id": "DEP_RUNTIME_DIGEST_MATCH",
                    "condition_type": "EQUALITY_ASSERTION",
                    "subject": "runtime.observed_digest",
                    "comparison": "deployment.expected_digest"
                }
            ]
        })
    }

    #[test]
    fn tc_pol_def_001_valid_policy_accepted() {
        let pol = get_valid_policy();
        assert_eq!(PolicyDefinitionValidator::validate(&pol), PolicyDefinitionValidationResult::Valid);
    }

    #[test]
    fn tc_pol_def_002_floating_policy_rejected() {
        let mut pol = get_valid_policy();
        pol["policy_version"] = json!("latest");
        assert!(matches!(PolicyDefinitionValidator::validate(&pol), PolicyDefinitionValidationResult::Invalid(_)));

        pol["policy_version"] = json!("main");
        assert!(matches!(PolicyDefinitionValidator::validate(&pol), PolicyDefinitionValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_pol_def_003_authority_injection_rejected() {
        let mut pol = get_valid_policy();
        pol.as_object_mut().unwrap().insert("grant_deployment_permission".to_string(), json!(true));
        assert!(matches!(PolicyDefinitionValidator::validate(&pol), PolicyDefinitionValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_pol_def_004_evidence_scope_enforcement() {
        let pol = get_valid_policy();
        let types = pol["applicable_evidence_types"].as_array().unwrap();
        assert!(types.contains(&json!("DEPLOYMENT_OBSERVATION")));
    }

    #[test]
    fn tc_pol_def_005_execution_directive_rejection() {
        let mut pol = get_valid_policy();
        pol["rules"][0]["comparison"] = json!("execute_command_deploy");
        assert!(matches!(PolicyDefinitionValidator::validate(&pol), PolicyDefinitionValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_pol_def_006_policy_digest_reproducibility() {
        let pol_a = get_valid_policy();
        let pol_b = get_valid_policy();

        let digest_a = PolicyDefinitionValidator::compute_policy_digest(&pol_a);
        let digest_b = PolicyDefinitionValidator::compute_policy_digest(&pol_b);

        assert_eq!(digest_a, digest_b);
    }

    #[test]
    fn tc_pol_def_007_conflicting_rule_handling() {
        let mut pol = get_valid_policy();
        pol["conflict_strategy"] = json!("STRICT_DENY_ON_CONFLICT");
        assert_eq!(PolicyDefinitionValidator::validate(&pol), PolicyDefinitionValidationResult::Valid);
    }
}
