use chrono::{DateTime, Utc};
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum MergeAuthorizationValidationResult {
    Valid,
    Invalid(String),
}

pub struct MergeAuthorizationValidator;

impl MergeAuthorizationValidator {
    pub fn validate(value: &serde_json::Value) -> MergeAuthorizationValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str()) != Some("REPOSITORY_MERGE_AUTHORIZATION-v1") {
            return MergeAuthorizationValidationResult::Invalid("Invalid or missing schema_version".into());
        }

        // Validate authorization_id pattern
        if let Some(id) = value.get("authorization_id").and_then(|v| v.as_str()) {
            if !id.starts_with("merge_auth_") {
                return MergeAuthorizationValidationResult::Invalid("Invalid authorization_id format".into());
            }
        } else {
            return MergeAuthorizationValidationResult::Invalid("Missing authorization_id".into());
        }

        // Validate candidate_binding presence
        if value.get("candidate_binding").is_none() {
            return MergeAuthorizationValidationResult::Invalid("Missing candidate_binding".into());
        }

        // Validate authorized_scope restrictions (protection bypasses and side effects must be explicitly false)
        if let Some(scope) = value.get("authorized_scope") {
            if scope.get("operation").and_then(|v| v.as_str()) != Some("repository.remote.merge_exact") {
                return MergeAuthorizationValidationResult::Invalid("Invalid or unauthorized operation scope".into());
            }
            if scope.get("bypass_protection").and_then(|v| v.as_bool()) != Some(false) {
                return MergeAuthorizationValidationResult::Invalid("Violation: bypass_protection must be false".into());
            }
            if scope.get("force_merge").and_then(|v| v.as_bool()) != Some(false) {
                return MergeAuthorizationValidationResult::Invalid("Violation: force_merge must be false".into());
            }
            if scope.get("deployment_permitted").and_then(|v| v.as_bool()) != Some(false) {
                return MergeAuthorizationValidationResult::Invalid("Violation: deployment_permitted must be false".into());
            }
            if scope.get("publication_permitted").and_then(|v| v.as_bool()) != Some(false) {
                return MergeAuthorizationValidationResult::Invalid("Violation: publication_permitted must be false".into());
            }
        } else {
            return MergeAuthorizationValidationResult::Invalid("Missing authorized_scope".into());
        }

        // Validate strategy_binding enum
        if let Some(sb) = value.get("strategy_binding") {
            if let Some(strat) = sb.get("strategy").and_then(|v| v.as_str()) {
                let valid_strategies = ["MERGE_COMMIT", "SQUASH_MERGE", "REBASE_MERGE"];
                if !valid_strategies.contains(&strat) {
                    return MergeAuthorizationValidationResult::Invalid("Invalid strategy value".into());
                }
            } else {
                return MergeAuthorizationValidationResult::Invalid("Missing strategy in strategy_binding".into());
            }
        } else {
            return MergeAuthorizationValidationResult::Invalid("Missing strategy_binding".into());
        }

        // Validate temporal bounds and expiration
        if let Some(bounds) = value.get("temporal_bounds") {
            let issued_str = bounds.get("issued_at").and_then(|v| v.as_str());
            let expires_str = bounds.get("expires_at").and_then(|v| v.as_str());

            if let (Some(issued), Some(expires)) = (issued_str, expires_str) {
                let issued_dt = DateTime::parse_from_rfc3339(issued);
                let expires_dt = DateTime::parse_from_rfc3339(expires);

                if let (Ok(i), Ok(e)) = (issued_dt, expires_dt) {
                    if e <= i {
                        return MergeAuthorizationValidationResult::Invalid("Temporal bounds violation: expires_at must be after issued_at".into());
                    }
                    if e < Utc::now() {
                        return MergeAuthorizationValidationResult::Invalid("Authorization expired".into());
                    }
                } else {
                    return MergeAuthorizationValidationResult::Invalid("Invalid RFC3339 date format in temporal bounds".into());
                }
            } else {
                return MergeAuthorizationValidationResult::Invalid("Missing issued_at or expires_at".into());
            }
        } else {
            return MergeAuthorizationValidationResult::Invalid("Missing temporal_bounds".into());
        }

        // Validate consumption policy
        if value.get("consumption_policy").and_then(|p| p.get("single_use_only")).and_then(|v| v.as_bool()) != Some(true) {
            return MergeAuthorizationValidationResult::Invalid("single_use_only must be true".into());
        }

        // Check for injected credential or mutation fields
        let allowed_keys = [
            "schema_version", "authorization_id", "candidate_binding",
            "authorized_scope", "strategy_binding", "temporal_bounds", "consumption_policy"
        ];
        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return MergeAuthorizationValidationResult::Invalid(format!("Injected unauthorized merge authorization field detected: {}", key));
                }
            }
        }

        MergeAuthorizationValidationResult::Valid
    }
}

#[cfg(test)]
mod merge_authorization_schema_tests {
    use super::*;

    fn get_valid_merge_authorization() -> serde_json::Value {
        let future_exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();

        json!({
            "schema_version": "REPOSITORY_MERGE_AUTHORIZATION-v1",
            "authorization_id": "merge_auth_01XYZ",
            "candidate_binding": {
                "merge_candidate_id": "mrg_cand_01ABC",
                "candidate_commit_oid": "e9aeb73000000000000000000000000000000000"
            },
            "authorized_scope": {
                "operation": "repository.remote.merge_exact",
                "bypass_protection": false,
                "force_merge": false,
                "deployment_permitted": false,
                "publication_permitted": false
            },
            "strategy_binding": {
                "strategy": "SQUASH_MERGE"
            },
            "temporal_bounds": {
                "issued_at": past_iss,
                "expires_at": future_exp
            },
            "consumption_policy": {
                "single_use_only": true
            }
        })
    }

    #[test]
    fn tc_mrg_auth_001_valid_authorization_accepted() {
        let auth = get_valid_merge_authorization();
        assert_eq!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Valid);
    }

    #[test]
    fn tc_mrg_auth_002_missing_candidate_binding_rejected() {
        let mut auth = get_valid_merge_authorization();
        auth.as_object_mut().unwrap().remove("candidate_binding");
        assert!(matches!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_mrg_auth_003_force_merge_injection_rejected() {
        let mut auth = get_valid_merge_authorization();
        auth["authorized_scope"]["force_merge"] = json!(true);
        assert!(matches!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_mrg_auth_004_protection_bypass_injection_rejected() {
        let mut auth = get_valid_merge_authorization();
        auth["authorized_scope"]["bypass_protection"] = json!(true);
        assert!(matches!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_mrg_auth_005_deployment_capability_injection_rejected() {
        let mut auth = get_valid_merge_authorization();
        auth["authorized_scope"]["deployment_permitted"] = json!(true);
        assert!(matches!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_mrg_auth_006_publication_capability_injection_rejected() {
        let mut auth = get_valid_merge_authorization();
        auth["authorized_scope"]["publication_permitted"] = json!(true);
        assert!(matches!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_mrg_auth_007_credential_injection_rejected() {
        let mut auth = get_valid_merge_authorization();
        auth.as_object_mut().unwrap().insert("secret_token".to_string(), json!("ghp_secret123"));
        assert!(matches!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_mrg_auth_008_invalid_strategy_rejected() {
        let mut auth = get_valid_merge_authorization();
        auth["strategy_binding"]["strategy"] = json!("MAGIC_MERGE");
        assert!(matches!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_mrg_auth_009_temporal_inversion_rejected() {
        let mut auth = get_valid_merge_authorization();
        let future_iss = (Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
        let past_exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        auth["temporal_bounds"]["issued_at"] = json!(future_iss);
        auth["temporal_bounds"]["expires_at"] = json!(past_exp);
        assert!(matches!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_mrg_auth_010_single_use_requirement_enforced() {
        let mut auth = get_valid_merge_authorization();
        auth["consumption_policy"]["single_use_only"] = json!(false);
        assert!(matches!(MergeAuthorizationValidator::validate(&auth), MergeAuthorizationValidationResult::Invalid(_)));
    }
}
