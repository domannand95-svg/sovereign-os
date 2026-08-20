use chrono::{DateTime, Utc};
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewCandidateValidationResult {
    Valid,
    Invalid(String),
}

pub struct ReviewObservationCandidateValidator;

impl ReviewObservationCandidateValidator {
    pub fn validate(value: &serde_json::Value) -> ReviewCandidateValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str()) != Some("REPOSITORY_REVIEW_OBSERVATION_CANDIDATE-v1") {
            return ReviewCandidateValidationResult::Invalid("Invalid or missing schema_version".into());
        }

        // Validate review_observation_id pattern
        if let Some(id) = value.get("review_observation_id").and_then(|v| v.as_str()) {
            if !id.starts_with("rev_obs_") {
                return ReviewCandidateValidationResult::Invalid("Invalid review_observation_id format".into());
            }
        } else {
            return ReviewCandidateValidationResult::Invalid("Missing review_observation_id".into());
        }

        // Validate pull_request_binding presence
        if value.get("pull_request_binding").is_none() {
            return ReviewCandidateValidationResult::Invalid("Missing pull_request_binding".into());
        }

        // Validate reviewer_identity presence
        if value.get("reviewer_identity").is_none() {
            return ReviewCandidateValidationResult::Invalid("Missing reviewer_identity".into());
        }

        // Validate review_state enum
        if let Some(state) = value.get("review_state").and_then(|v| v.as_str()) {
            let valid_states = ["COMMENTED", "CHANGES_REQUESTED", "APPROVED_OBSERVED", "DISMISSED"];
            if !valid_states.contains(&state) {
                return ReviewCandidateValidationResult::Invalid("Invalid review_state value".into());
            }
        } else {
            return ReviewCandidateValidationResult::Invalid("Missing review_state".into());
        }

        // Injected Authority Check: Ensure no implicit merge or write-to-merge tokens exist
        let allowed_keys = [
            "schema_version", "review_observation_id", "pull_request_binding",
            "reviewer_identity", "review_state", "observed_at"
        ];
        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return ReviewCandidateValidationResult::Invalid(format!("Injected unauthorized authority field detected: {}", key));
                }
            }
        }

        ReviewCandidateValidationResult::Valid
    }
}

#[cfg(test)]
mod review_candidate_schema_tests {
    use super::*;

    fn get_valid_review_observation() -> serde_json::Value {
        json!({
            "schema_version": "REPOSITORY_REVIEW_OBSERVATION_CANDIDATE-v1",
            "review_observation_id": "rev_obs_01XYZ",
            "pull_request_binding": {
                "candidate_id": "pr_cand_01JXYZ",
                "source_commit_oid": "e9aeb73000000000000000000000000000000000"
            },
            "reviewer_identity": {
                "principal_id": "auditor_agent_01",
                "principal_type": "agent"
            },
            "review_state": "COMMENTED",
            "observed_at": Utc::now().to_rfc3339()
        })
    }

    #[test]
    fn tc_rev_cand_001_valid_observation_accepted() {
        let obs = get_valid_review_observation();
        assert_eq!(ReviewObservationCandidateValidator::validate(&obs), ReviewCandidateValidationResult::Valid);
    }

    #[test]
    fn tc_rev_cand_002_missing_pr_binding_rejected() {
        let mut obs = get_valid_review_observation();
        obs.as_object_mut().unwrap().remove("pull_request_binding");
        assert!(matches!(ReviewObservationCandidateValidator::validate(&obs), ReviewCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_rev_cand_003_invalid_review_state_rejected() {
        let mut obs = get_valid_review_observation();
        obs["review_state"] = json!("FORCE_MERGE_GRANTED");
        assert!(matches!(ReviewObservationCandidateValidator::validate(&obs), ReviewCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_rev_cand_004_injected_merge_authority_rejected() {
        let mut obs = get_valid_review_observation();
        obs.as_object_mut().unwrap().insert("grants_merge_authority".to_string(), json!(true));
        assert!(matches!(ReviewObservationCandidateValidator::validate(&obs), ReviewCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_rev_cand_005_missing_reviewer_identity_rejected() {
        let mut obs = get_valid_review_observation();
        obs.as_object_mut().unwrap().remove("reviewer_identity");
        assert!(matches!(ReviewObservationCandidateValidator::validate(&obs), ReviewCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_rev_cand_006_invalid_observation_id_format_rejected() {
        let mut obs = get_valid_review_observation();
        obs["review_observation_id"] = json!("malicious_id_123");
        assert!(matches!(ReviewObservationCandidateValidator::validate(&obs), ReviewCandidateValidationResult::Invalid(_)));
    }

    #[test]
    fn tc_rev_cand_007_approved_observed_state_valid() {
        let mut obs = get_valid_review_observation();
        obs["review_state"] = json!("APPROVED_OBSERVED");
        assert_eq!(ReviewObservationCandidateValidator::validate(&obs), ReviewCandidateValidationResult::Valid);
    }
}
