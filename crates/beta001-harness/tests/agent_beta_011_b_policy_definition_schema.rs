use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyValidationResult {
    Valid,
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub rule_id: String,
    pub description: String,
    pub target_domain: String,
    pub condition_expression: String,
    pub required_evidence_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDefinition {
    pub schema_version: String,
    pub policy_id: String,
    pub rules: Vec<PolicyRule>,
    pub policy_digest: String,
    pub created_at: String,
}

pub struct PolicyDefinitionValidator;

impl PolicyDefinitionValidator {
    pub fn validate(value: &serde_json::Value) -> PolicyValidationResult {
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_GOVERNANCE_POLICY_DEFINITION-v1")
        {
            return PolicyValidationResult::Invalid("Invalid or missing schema_version".into());
        }

        if let Some(id) = value.get("policy_id").and_then(|v| v.as_str()) {
            if !id.starts_with("pol_") {
                return PolicyValidationResult::Invalid("Invalid policy_id format".into());
            }
        } else {
            return PolicyValidationResult::Invalid("Missing policy_id".into());
        }

        let rules = match value.get("rules").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return PolicyValidationResult::Invalid(
                    "Policy must contain at least one rule".into(),
                )
            }
        };

        for rule in rules {
            if let Some(target) = rule.get("target_domain").and_then(|v| v.as_str()) {
                if target.is_empty() {
                    return PolicyValidationResult::Invalid(
                        "Floating policy rule: missing target domain binding".into(),
                    );
                }
            } else {
                return PolicyValidationResult::Invalid("Missing target_domain in rule".into());
            }

            if let Some(cond) = rule.get("condition_expression").and_then(|v| v.as_str()) {
                if cond.contains("EXECUTE_DEPLOYMENT")
                    || cond.contains("MERGE_FORCE")
                    || cond.contains("GRANT_CAPABILITY")
                {
                    return PolicyValidationResult::Invalid(
                        "Forbidden operational command or execution directive in rule".into(),
                    );
                }
            }
        }

        let allowed_keys = [
            "schema_version",
            "policy_id",
            "rules",
            "policy_digest",
            "created_at",
        ];
        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return PolicyValidationResult::Invalid(format!(
                        "Authority injection or forbidden field detected: {}",
                        key
                    ));
                }
            }
        }

        PolicyValidationResult::Valid
    }

    pub fn compute_policy_digest(policy: &PolicyDefinition) -> String {
        let mut hasher = DefaultHasher::new();
        policy.schema_version.hash(&mut hasher);
        policy.policy_id.hash(&mut hasher);
        for rule in &policy.rules {
            rule.rule_id.hash(&mut hasher);
            rule.target_domain.hash(&mut hasher);
            rule.condition_expression.hash(&mut hasher);
            for ev in &rule.required_evidence_types {
                ev.hash(&mut hasher);
            }
        }
        format!("sha256:{:016x}", hasher.finish())
    }
}

#[cfg(test)]
mod policy_definition_schema_tests {
    use super::*;

    fn get_valid_policy() -> PolicyDefinition {
        PolicyDefinition {
            schema_version: "REPOSITORY_GOVERNANCE_POLICY_DEFINITION-v1".into(),
            policy_id: "pol_01XYZ".into(),
            rules: vec![PolicyRule {
                rule_id: "rule_require_review".into(),
                description: "Require at least one verified review".into(),
                target_domain: "PULL_REQUEST".into(),
                condition_expression: "count(evidence.review.approved) >= 1".into(),
                required_evidence_types: vec!["REVIEW".into()],
            }],
            policy_digest:
                "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            created_at: "2026-08-20T07:00:00Z".into(),
        }
    }

    #[test]
    fn tc_pol_def_001_valid_policy_accepted() {
        let policy = get_valid_policy();
        let val = serde_json::to_value(&policy).unwrap();
        assert_eq!(
            PolicyDefinitionValidator::validate(&val),
            PolicyValidationResult::Valid
        );
    }

    #[test]
    fn tc_pol_def_002_floating_policy_rejected() {
        let mut policy = get_valid_policy();
        policy.rules[0].target_domain = "".into();
        let val = serde_json::to_value(&policy).unwrap();
        assert!(matches!(
            PolicyDefinitionValidator::validate(&val),
            PolicyValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pol_def_003_authority_injection_rejected() {
        let policy = get_valid_policy();
        let mut val = serde_json::to_value(&policy).unwrap();
        val.as_object_mut()
            .unwrap()
            .insert("grant_capability".to_string(), json!("admin"));
        assert!(matches!(
            PolicyDefinitionValidator::validate(&val),
            PolicyValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pol_def_004_evidence_scope_enforcement() {
        let mut policy = get_valid_policy();
        policy.rules[0].required_evidence_types = vec![];
        let val = serde_json::to_value(&policy).unwrap();
        assert_eq!(
            PolicyDefinitionValidator::validate(&val),
            PolicyValidationResult::Valid
        );
    }

    #[test]
    fn tc_pol_def_005_execution_directive_rejection() {
        let mut policy = get_valid_policy();
        policy.rules[0].condition_expression = "EXECUTE_DEPLOYMENT == true".into();
        let val = serde_json::to_value(&policy).unwrap();
        assert!(matches!(
            PolicyDefinitionValidator::validate(&val),
            PolicyValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pol_def_006_policy_digest_reproducibility() {
        let policy_a = get_valid_policy();
        let policy_b = get_valid_policy();
        let digest_a = PolicyDefinitionValidator::compute_policy_digest(&policy_a);
        let digest_b = PolicyDefinitionValidator::compute_policy_digest(&policy_b);
        assert_eq!(digest_a, digest_b);
    }

    #[test]
    fn tc_pol_def_007_conflicting_rule_handling() {
        let mut policy = get_valid_policy();
        policy.rules.push(PolicyRule {
            rule_id: "rule_deny_pr".into(),
            description: "Deny all PRs".into(),
            target_domain: "PULL_REQUEST".into(),
            condition_expression: "false".into(),
            required_evidence_types: vec!["PULL_REQUEST".into()],
        });
        let val = serde_json::to_value(&policy).unwrap();
        assert_eq!(
            PolicyDefinitionValidator::validate(&val),
            PolicyValidationResult::Valid
        );
    }
}
