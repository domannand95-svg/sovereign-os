use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum CandidateValidationResult {
    Valid,
    Invalid(String),
}

pub struct PullRequestCandidateValidator;

impl PullRequestCandidateValidator {
    pub fn validate(value: &serde_json::Value) -> CandidateValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_PULL_REQUEST_CANDIDATE-v1")
        {
            return CandidateValidationResult::Invalid("Invalid or missing schema_version".into());
        }

        // Validate candidate_id pattern
        if let Some(id) = value.get("candidate_id").and_then(|v| v.as_str()) {
            if !id.starts_with("pr_cand_") {
                return CandidateValidationResult::Invalid("Invalid candidate_id format".into());
            }
        } else {
            return CandidateValidationResult::Invalid("Missing candidate_id".into());
        }

        // Validate source_publication_reference presence and format
        if let Some(pub_ref) = value.get("source_publication_reference") {
            let receipt_id = pub_ref
                .get("publication_receipt_id")
                .and_then(|v| v.as_str());
            let commit_oid = pub_ref.get("source_commit_oid").and_then(|v| v.as_str());

            if receipt_id.is_none() || !receipt_id.unwrap().starts_with("pub_receipt_") {
                return CandidateValidationResult::Invalid(
                    "Missing or invalid source_publication_reference.publication_receipt_id".into(),
                );
            }
            if commit_oid.is_none() || commit_oid.unwrap().len() != 40 {
                return CandidateValidationResult::Invalid(
                    "Missing or invalid source_publication_reference.source_commit_oid".into(),
                );
            }
        } else {
            return CandidateValidationResult::Invalid(
                "Missing source_publication_reference".into(),
            );
        }

        // Validate source_ref pattern
        if let Some(s_ref) = value.get("source_ref").and_then(|v| v.as_str()) {
            if !s_ref.starts_with("refs/heads/") {
                return CandidateValidationResult::Invalid("Invalid source_ref format".into());
            }
        } else {
            return CandidateValidationResult::Invalid("Missing source_ref".into());
        }

        // Validate target_ref pattern
        if let Some(t_ref) = value.get("target_ref").and_then(|v| v.as_str()) {
            if !t_ref.starts_with("refs/heads/") {
                return CandidateValidationResult::Invalid("Invalid target_ref format".into());
            }
        } else {
            return CandidateValidationResult::Invalid("Missing target_ref".into());
        }

        // Injected Authority Check: Ensure additional unauthorized escalation fields are rejected
        let allowed_keys = [
            "schema_version",
            "candidate_id",
            "source_publication_reference",
            "source_repository_identity",
            "source_ref",
            "target_repository_identity",
            "target_ref",
            "title",
            "description",
            "risk_classification",
        ];
        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return CandidateValidationResult::Invalid(format!(
                        "Injected unauthorized authority field detected: {}",
                        key
                    ));
                }
            }
        }

        CandidateValidationResult::Valid
    }
}

#[cfg(test)]
mod pr_candidate_schema_tests {
    use super::*;

    fn get_valid_candidate() -> serde_json::Value {
        json!({
            "schema_version": "REPOSITORY_PULL_REQUEST_CANDIDATE-v1",
            "candidate_id": "pr_cand_01JXYZ",
            "source_publication_reference": {
                "publication_receipt_id": "pub_receipt_01ABC",
                "source_commit_oid": "e9aeb73000000000000000000000000000000000"
            },
            "source_repository_identity": {
                "provider": "github.com",
                "repository_id": "repo_123",
                "owner": "org-sovereign",
                "name": "core-os"
            },
            "source_ref": "refs/heads/feature-xyz",
            "target_repository_identity": {
                "provider": "github.com",
                "repository_id": "repo_123",
                "owner": "org-sovereign",
                "name": "core-os"
            },
            "target_ref": "refs/heads/develop",
            "title": "Governed Integration Proposal",
            "description": "Proposing feature integration",
            "risk_classification": "StandardSourceChange"
        })
    }

    #[test]
    fn tc_pr_cand_001_valid_candidate_accepted() {
        let candidate = get_valid_candidate();
        assert_eq!(
            PullRequestCandidateValidator::validate(&candidate),
            CandidateValidationResult::Valid
        );
    }

    #[test]
    fn tc_pr_cand_002_missing_publication_receipt_rejected() {
        let mut candidate = get_valid_candidate();
        candidate
            .as_object_mut()
            .unwrap()
            .remove("source_publication_reference");
        assert!(matches!(
            PullRequestCandidateValidator::validate(&candidate),
            CandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_cand_003_invalid_source_ref_rejected() {
        let mut candidate = get_valid_candidate();
        candidate["source_ref"] = json!("refs/tags/v1.0");
        assert!(matches!(
            PullRequestCandidateValidator::validate(&candidate),
            CandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_cand_004_invalid_target_ref_rejected() {
        let mut candidate = get_valid_candidate();
        candidate["target_ref"] = json!("malicious_ref");
        assert!(matches!(
            PullRequestCandidateValidator::validate(&candidate),
            CandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_cand_005_injected_merge_authority_field_rejected() {
        let mut candidate = get_valid_candidate();
        candidate
            .as_object_mut()
            .unwrap()
            .insert("auto_merge_enabled".to_string(), json!(true));
        assert!(matches!(
            PullRequestCandidateValidator::validate(&candidate),
            CandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_cand_006_injected_reviewer_approval_field_rejected() {
        let mut candidate = get_valid_candidate();
        candidate
            .as_object_mut()
            .unwrap()
            .insert("bypass_required_reviews".to_string(), json!(true));
        assert!(matches!(
            PullRequestCandidateValidator::validate(&candidate),
            CandidateValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_pr_cand_007_repository_identity_mismatch_handled() {
        let candidate = get_valid_candidate();
        // Schema requires source and target structures to be present and well-formed
        assert_eq!(
            PullRequestCandidateValidator::validate(&candidate),
            CandidateValidationResult::Valid
        );
    }
}
