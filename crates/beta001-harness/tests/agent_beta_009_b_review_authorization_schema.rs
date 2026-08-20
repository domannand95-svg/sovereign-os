use chrono::{DateTime, Utc};
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewAuthorizationValidationResult {
    Valid,
    Invalid(String),
}

pub struct ReviewAuthorizationValidator;

impl ReviewAuthorizationValidator {
    pub fn validate(value: &serde_json::Value) -> ReviewAuthorizationValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_REVIEW_AUTHORIZATION-v1")
        {
            return ReviewAuthorizationValidationResult::Invalid(
                "Invalid or missing schema_version".into(),
            );
        }

        // Validate authorization_id pattern
        if let Some(id) = value.get("authorization_id").and_then(|v| v.as_str()) {
            if !id.starts_with("review_auth_") {
                return ReviewAuthorizationValidationResult::Invalid(
                    "Invalid authorization_id format".into(),
                );
            }
        } else {
            return ReviewAuthorizationValidationResult::Invalid("Missing authorization_id".into());
        }

        // Validate candidate_binding presence
        if value.get("candidate_binding").is_none() {
            return ReviewAuthorizationValidationResult::Invalid(
                "Missing candidate_binding".into(),
            );
        }

        // Validate authorized_scope restrictions (approval_permitted and merge_permitted must be false)
        if let Some(scope) = value.get("authorized_scope") {
            let op = scope.get("operation").and_then(|v| v.as_str());
            if op != Some("repository.remote.review.observe")
                && op != Some("repository.remote.review.submit_comment")
            {
                return ReviewAuthorizationValidationResult::Invalid(
                    "Invalid or unauthorized operation scope".into(),
                );
            }
            if scope.get("approval_permitted").and_then(|v| v.as_bool()) != Some(false) {
                return ReviewAuthorizationValidationResult::Invalid(
                    "Violation: approval_permitted must be false".into(),
                );
            }
            if scope.get("merge_permitted").and_then(|v| v.as_bool()) != Some(false) {
                return ReviewAuthorizationValidationResult::Invalid(
                    "Violation: merge_permitted must be false".into(),
                );
            }
        } else {
            return ReviewAuthorizationValidationResult::Invalid("Missing authorized_scope".into());
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
                        return ReviewAuthorizationValidationResult::Invalid(
                            "Temporal bounds violation: expires_at must be after issued_at".into(),
                        );
                    }
                    if e < Utc::now() {
                        return ReviewAuthorizationValidationResult::Invalid(
                            "Authorization expired".into(),
                        );
                    }
                } else {
                    return ReviewAuthorizationValidationResult::Invalid(
                        "Invalid RFC3339 date format in temporal bounds".into(),
                    );
                }
            } else {
                return ReviewAuthorizationValidationResult::Invalid(
                    "Missing issued_at or expires_at".into(),
                );
            }
        } else {
            return ReviewAuthorizationValidationResult::Invalid("Missing temporal_bounds".into());
        }

        // Validate consumption policy
        if value
            .get("consumption_policy")
            .and_then(|p| p.get("single_use_only"))
            .and_then(|v| v.as_bool())
            != Some(true)
        {
            return ReviewAuthorizationValidationResult::Invalid(
                "single_use_only must be true".into(),
            );
        }

        // Check for injected credential or mutation fields
        let allowed_keys = [
            "schema_version",
            "authorization_id",
            "candidate_binding",
            "authorized_scope",
            "temporal_bounds",
            "consumption_policy",
        ];
        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return ReviewAuthorizationValidationResult::Invalid(format!(
                        "Injected unauthorized review authorization field detected: {}",
                        key
                    ));
                }
            }
        }

        ReviewAuthorizationValidationResult::Valid
    }
}

#[cfg(test)]
mod review_authorization_schema_tests {
    use super::*;

    fn get_valid_review_authorization() -> serde_json::Value {
        let future_exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();

        json!({
            "schema_version": "REPOSITORY_REVIEW_AUTHORIZATION-v1",
            "authorization_id": "review_auth_01XYZ",
            "candidate_binding": {
                "review_candidate_id": "review_cand_01ABC",
                "pull_request_id": "pr_123"
            },
            "authorized_scope": {
                "operation": "repository.remote.review.observe",
                "approval_permitted": false,
                "merge_permitted": false
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
    fn tc_rev_auth_001_valid_authorization_accepted() {
        let auth = get_valid_review_authorization();
        assert_eq!(
            ReviewAuthorizationValidator::validate(&auth),
            ReviewAuthorizationValidationResult::Valid
        );
    }

    #[test]
    fn tc_rev_auth_002_missing_candidate_binding_rejected() {
        let mut auth = get_valid_review_authorization();
        auth.as_object_mut().unwrap().remove("candidate_binding");
        assert!(matches!(
            ReviewAuthorizationValidator::validate(&auth),
            ReviewAuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_rev_auth_003_approval_capability_injection_rejected() {
        let mut auth = get_valid_review_authorization();
        auth["authorized_scope"]["approval_permitted"] = json!(true);
        assert!(matches!(
            ReviewAuthorizationValidator::validate(&auth),
            ReviewAuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_rev_auth_004_merge_capability_injection_rejected() {
        let mut auth = get_valid_review_authorization();
        auth["authorized_scope"]["merge_permitted"] = json!(true);
        assert!(matches!(
            ReviewAuthorizationValidator::validate(&auth),
            ReviewAuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_rev_auth_005_credential_injection_rejected() {
        let mut auth = get_valid_review_authorization();
        auth.as_object_mut()
            .unwrap()
            .insert("secret_token".to_string(), json!("ghp_secret123"));
        assert!(matches!(
            ReviewAuthorizationValidator::validate(&auth),
            ReviewAuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_rev_auth_006_expired_authorization_rejected() {
        let mut auth = get_valid_review_authorization();
        let past_iss = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        let past_exp = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        auth["temporal_bounds"]["issued_at"] = json!(past_iss);
        auth["temporal_bounds"]["expires_at"] = json!(past_exp);
        assert!(matches!(
            ReviewAuthorizationValidator::validate(&auth),
            ReviewAuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_rev_auth_007_temporal_inversion_rejected() {
        let mut auth = get_valid_review_authorization();
        let future_iss = (Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
        let past_exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        auth["temporal_bounds"]["issued_at"] = json!(future_iss);
        auth["temporal_bounds"]["expires_at"] = json!(past_exp);
        assert!(matches!(
            ReviewAuthorizationValidator::validate(&auth),
            ReviewAuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_rev_auth_008_reviewer_identity_impersonation_rejected() {
        let mut auth = get_valid_review_authorization();
        auth.as_object_mut()
            .unwrap()
            .insert("impersonate_principal".to_string(), json!("admin_user"));
        assert!(matches!(
            ReviewAuthorizationValidator::validate(&auth),
            ReviewAuthorizationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_rev_auth_009_single_use_consumption_requirement_enforced() {
        let mut auth = get_valid_review_authorization();
        auth["consumption_policy"]["single_use_only"] = json!(false);
        assert!(matches!(
            ReviewAuthorizationValidator::validate(&auth),
            ReviewAuthorizationValidationResult::Invalid(_)
        ));
    }
}
