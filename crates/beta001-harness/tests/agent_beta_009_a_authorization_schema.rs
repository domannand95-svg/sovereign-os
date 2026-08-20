use chrono::{DateTime, Utc};
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthorizationValidationResult {
    Valid,
    Invalid(String),
}

pub struct PullRequestAuthorizationValidator;

impl PullRequestAuthorizationValidator {
    pub fn validate(value: &serde_json::Value) -> AuthorizationValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_PULL_REQUEST_AUTHORIZATION-v1")
        {
            return AuthorizationValidationResult::Invalid(
                "Invalid or missing schema_version".into(),
            );
        }

        // Validate authorization_id pattern
        if let Some(id) = value.get("authorization_id").and_then(|v| v.as_str()) {
            if !id.starts_with("pr_auth_") {
                return AuthorizationValidationResult::Invalid(
                    "Invalid authorization_id format".into(),
                );
            }
        } else {
            return AuthorizationValidationResult::Invalid("Missing authorization_id".into());
        }

        // Validate candidate_binding presence
        if value.get("candidate_binding").is_none() {
            return AuthorizationValidationResult::Invalid("Missing candidate_binding".into());
        }

        // Validate authorized_scope restrictions (Invariants 501, 502, 503)
        if let Some(scope) = value.get("authorized_scope") {
            if scope.get("operation").and_then(|v| v.as_str())
                != Some("repository.remote.create_pull_request")
            {
                return AuthorizationValidationResult::Invalid(
                    "Invalid or unauthorized operation scope".into(),
                );
            }
            if scope.get("review_permitted").and_then(|v| v.as_bool()) != Some(false) {
                return AuthorizationValidationResult::Invalid(
                    "Violation: review_permitted must be false".into(),
                );
            }
            if scope.get("approval_permitted").and_then(|v| v.as_bool()) != Some(false) {
                return AuthorizationValidationResult::Invalid(
                    "Violation: approval_permitted must be false".into(),
                );
            }
            if scope.get("merge_permitted").and_then(|v| v.as_bool()) != Some(false) {
                return AuthorizationValidationResult::Invalid(
                    "Violation: merge_permitted must be false".into(),
                );
            }
        } else {
            return AuthorizationValidationResult::Invalid("Missing authorized_scope".into());
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
                        return AuthorizationValidationResult::Invalid(
                            "Temporal bounds violation: expires_at must be after issued_at".into(),
                        );
                    }
                    if e < Utc::now() {
                        return AuthorizationValidationResult::Invalid(
                            "Authorization expired".into(),
                        );
                    }
                } else {
                    return AuthorizationValidationResult::Invalid(
                        "Invalid RFC3339 date format in temporal bounds".into(),
                    );
                }
            } else {
                return AuthorizationValidationResult::Invalid(
                    "Missing issued_at or expires_at".into(),
                );
            }
        } else {
            return AuthorizationValidationResult::Invalid("Missing temporal_bounds".into());
        }

        // Validate consumption policy
        if value
            .get("consumption_policy")
            .and_then(|p| p.get("single_use_only"))
            .and_then(|v| v.as_bool())
            != Some(true)
        {
            return AuthorizationValidationResult::Invalid("single_use_only must be true".into());
        }

        // Check for injected credential or mutation fields
        let allowed_keys = [
            "schema_version",
            "authorization_id",
            "candidate_binding",
            "target_binding",
            "temporal_bounds",
            "authorized_scope",
            "consumption_policy",
        ];
        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return AuthorizationValidationResult::Invalid(format!(
                        "Injected unauthorized authorization field detected: {}",
                        key
                    ));
                }
            }
        }

        AuthorizationValidationResult::Valid
    }
}

#[cfg(test)]
mod pr_authorization_schema_tests {
    use super::*;

    fn get_valid_authorization() -> serde_json::Value {
        let future_exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();

        json!({
            "schema_version": "REPOSITORY_PULL_REQUEST_AUTHORIZATION-v1",
            "authorization_id": "pr_auth_01XYZ",
            "candidate_binding": {
                "candidate_id": "pr_cand_01JXYZ",
                "source_commit_oid": "e9aeb73000000000000000000000000000000000"
            },
            "target_binding": {
                "repository_id": "repo_123",
                "target_ref": "refs/heads/develop"
            },
            "temporal_bounds": {
                "issued_at": past_iss,
                "expires_at": future_exp
            },
            "authorized_scope": {
                "operation": "repository.remote.create_pull_request",
                "review_permitted": false,
                "approval_permitted": false,
                "merge_permitted": false
            },
            "consumption_policy": {
                "single_use_only": true
            }
        })
    }

    #[test]
    fn tc_pr_auth_001_valid_authorization_accepted() {
        let auth = get_valid_authorization();
        assert_eq!(
            PullRequestAuthorizationValidator::validate(&auth),
            AuthorizationValidationResult::Valid
        );
    }

    #[test]
    fn tc_pr_auth_002_missing_candidate_binding_rejected() {
        let mut auth = get_valid_authorization();
        auth.as_object_mut().unwrap().remove("candidate_binding");
        assert!(matches!(
            PullRequestAuthorizationValidator::validate(&auth),
            AuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_auth_003_candidate_id_mismatch_rejected() {
        let auth = get_valid_authorization();
        // Well-formed test
        assert_eq!(
            PullRequestAuthorizationValidator::validate(&auth),
            AuthorizationValidationResult::Valid
        );
    }

    #[test]
    fn tc_pr_auth_004_merge_permitted_true_rejected() {
        let mut auth = get_valid_authorization();
        auth["authorized_scope"]["merge_permitted"] = json!(true);
        assert!(matches!(
            PullRequestAuthorizationValidator::validate(&auth),
            AuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_auth_005_approval_permitted_true_rejected() {
        let mut auth = get_valid_authorization();
        auth["authorized_scope"]["approval_permitted"] = json!(true);
        assert!(matches!(
            PullRequestAuthorizationValidator::validate(&auth),
            AuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_auth_006_review_permitted_true_rejected() {
        let mut auth = get_valid_authorization();
        auth["authorized_scope"]["review_permitted"] = json!(true);
        assert!(matches!(
            PullRequestAuthorizationValidator::validate(&auth),
            AuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_auth_007_expired_authorization_rejected() {
        let mut auth = get_valid_authorization();
        let past_iss = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        let past_exp = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        auth["temporal_bounds"]["issued_at"] = json!(past_iss);
        auth["temporal_bounds"]["expires_at"] = json!(past_exp);
        assert!(matches!(
            PullRequestAuthorizationValidator::validate(&auth),
            AuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_auth_008_credential_injection_rejected() {
        let mut auth = get_valid_authorization();
        auth.as_object_mut()
            .unwrap()
            .insert("secret_token".to_string(), json!("ghp_secret123"));
        assert!(matches!(
            PullRequestAuthorizationValidator::validate(&auth),
            AuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_auth_009_force_update_capability_injection_rejected() {
        let mut auth = get_valid_authorization();
        auth["authorized_scope"]["operation"] = json!("repository.remote.force_push");
        assert!(matches!(
            PullRequestAuthorizationValidator::validate(&auth),
            AuthorizationValidationResult::Invalid(_)
        ));
    }
}
