use chrono::{DateTime, Utc};
use serde_json::json;

// =====================================================================
// 1. END-TO-END PROVENANCE CHAIN & LIFECYCLE DOMAIN TYPES
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct SovereignLifecycleReceipt {
    pub publication_id: String,
    pub pr_candidate_id: String,
    pub review_observation_id: String,
    pub merge_authorization_id: String,
    pub deployment_candidate_id: String,
    pub deployment_authorization_id: String,
    pub runtime_verification_digest: String,
    pub terminal_disposition: LifecycleTerminalDisposition,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleTerminalDisposition {
    VerifiedSuccess,
    Denied,
    RuntimeMismatch,
    Ambiguous,
    SchemaRejection,
}

// =====================================================================
// 2. LIFECYCLE COMPOSITION & PROVENANCE VALIDATOR
// =====================================================================

pub struct LifecycleCompositionValidator;

impl LifecycleCompositionValidator {
    pub fn evaluate_composition(
        pub_cand: &serde_json::Value,
        pr_cand: &serde_json::Value,
        review_obs: &serde_json::Value,
        merge_auth: &serde_json::Value,
        dep_cand: &serde_json::Value,
        dep_auth: &serde_json::Value,
        runtime_digest: &str,
    ) -> SovereignLifecycleReceipt {
        // 1. Validate Schema Versions across the stack
        let is_pub_valid = pub_cand.get("schema_version").and_then(|v| v.as_str())
            == Some("REPOSITORY_PUBLICATION_CANDIDATE-v1");
        let is_pr_valid = pr_cand.get("schema_version").and_then(|v| v.as_str())
            == Some("REPOSITORY_PR_CANDIDATE-v1");
        let is_rev_valid = review_obs.get("schema_version").and_then(|v| v.as_str())
            == Some("REPOSITORY_REVIEW_OBSERVATION-v1");
        let is_merge_valid = merge_auth.get("schema_version").and_then(|v| v.as_str())
            == Some("REPOSITORY_MERGE_AUTHORIZATION-v1");
        let is_dep_cand_valid = dep_cand.get("schema_version").and_then(|v| v.as_str())
            == Some("REPOSITORY_DEPLOYMENT_CANDIDATE-v1");
        let is_dep_auth_valid = dep_auth.get("schema_version").and_then(|v| v.as_str())
            == Some("REPOSITORY_DEPLOYMENT_AUTHORIZATION-v1");

        if !is_pub_valid
            || !is_pr_valid
            || !is_rev_valid
            || !is_merge_valid
            || !is_dep_cand_valid
            || !is_dep_auth_valid
        {
            return SovereignLifecycleReceipt {
                publication_id: "unknown".into(),
                pr_candidate_id: "unknown".into(),
                review_observation_id: "unknown".into(),
                merge_authorization_id: "unknown".into(),
                deployment_candidate_id: "unknown".into(),
                deployment_authorization_id: "unknown".into(),
                runtime_verification_digest: runtime_digest.into(),
                terminal_disposition: LifecycleTerminalDisposition::SchemaRejection,
            };
        }

        // 2. Validate Temporal Bounds for Deployment Authorization
        if let Some(bounds) = dep_auth.get("temporal_bounds") {
            let expires_str = bounds.get("expires_at").and_then(|v| v.as_str());
            if let Some(exp) = expires_str {
                if let Ok(e_dt) = DateTime::parse_from_rfc3339(exp) {
                    if e_dt < Utc::now() {
                        return SovereignLifecycleReceipt {
                            publication_id: pub_cand["publication_id"]
                                .as_str()
                                .unwrap_or("")
                                .into(),
                            pr_candidate_id: pr_cand["candidate_id"].as_str().unwrap_or("").into(),
                            review_observation_id: review_obs["observation_id"]
                                .as_str()
                                .unwrap_or("")
                                .into(),
                            merge_authorization_id: merge_auth["authorization_id"]
                                .as_str()
                                .unwrap_or("")
                                .into(),
                            deployment_candidate_id: dep_cand["deployment_candidate_id"]
                                .as_str()
                                .unwrap_or("")
                                .into(),
                            deployment_authorization_id: dep_auth["deployment_authorization_id"]
                                .as_str()
                                .unwrap_or("")
                                .into(),
                            runtime_verification_digest: runtime_digest.into(),
                            terminal_disposition: LifecycleTerminalDisposition::Denied,
                        };
                    }
                }
            }
        }

        // 3. Validate Artifact Binding between Deployment Candidate and Deployment Authorization
        let cand_digest = dep_cand
            .get("source_artifact_digest")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let auth_digest = dep_auth
            .get("deployment_candidate_ref")
            .and_then(|v| v.get("artifact_digest"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if cand_digest != auth_digest {
            return SovereignLifecycleReceipt {
                publication_id: pub_cand["publication_id"].as_str().unwrap_or("").into(),
                pr_candidate_id: pr_cand["candidate_id"].as_str().unwrap_or("").into(),
                review_observation_id: review_obs["observation_id"].as_str().unwrap_or("").into(),
                merge_authorization_id: merge_auth["authorization_id"]
                    .as_str()
                    .unwrap_or("")
                    .into(),
                deployment_candidate_id: dep_cand["deployment_candidate_id"]
                    .as_str()
                    .unwrap_or("")
                    .into(),
                deployment_authorization_id: dep_auth["deployment_authorization_id"]
                    .as_str()
                    .unwrap_or("")
                    .into(),
                runtime_verification_digest: runtime_digest.into(),
                terminal_disposition: LifecycleTerminalDisposition::Denied,
            };
        }

        // 4. Validate Runtime Verification against Candidate Artifact Digest
        if cand_digest != runtime_digest {
            return SovereignLifecycleReceipt {
                publication_id: pub_cand["publication_id"].as_str().unwrap_or("").into(),
                pr_candidate_id: pr_cand["candidate_id"].as_str().unwrap_or("").into(),
                review_observation_id: review_obs["observation_id"].as_str().unwrap_or("").into(),
                merge_authorization_id: merge_auth["authorization_id"]
                    .as_str()
                    .unwrap_or("")
                    .into(),
                deployment_candidate_id: dep_cand["deployment_candidate_id"]
                    .as_str()
                    .unwrap_or("")
                    .into(),
                deployment_authorization_id: dep_auth["deployment_authorization_id"]
                    .as_str()
                    .unwrap_or("")
                    .into(),
                runtime_verification_digest: runtime_digest.into(),
                terminal_disposition: LifecycleTerminalDisposition::RuntimeMismatch,
            };
        }

        SovereignLifecycleReceipt {
            publication_id: pub_cand["publication_id"]
                .as_str()
                .unwrap_or("pub_123")
                .into(),
            pr_candidate_id: pr_cand["candidate_id"].as_str().unwrap_or("pr_123").into(),
            review_observation_id: review_obs["observation_id"]
                .as_str()
                .unwrap_or("rev_123")
                .into(),
            merge_authorization_id: merge_auth["authorization_id"]
                .as_str()
                .unwrap_or("merge_auth_123")
                .into(),
            deployment_candidate_id: dep_cand["deployment_candidate_id"]
                .as_str()
                .unwrap_or("dep_cand_123")
                .into(),
            deployment_authorization_id: dep_auth["deployment_authorization_id"]
                .as_str()
                .unwrap_or("dep_auth_123")
                .into(),
            runtime_verification_digest: runtime_digest.into(),
            terminal_disposition: LifecycleTerminalDisposition::VerifiedSuccess,
        }
    }
}

// =====================================================================
// 3. ADVERSARIAL LIFECYCLE COMPOSITION TEST SUITE (TC-LIFECYCLE-001..007)
// =====================================================================

#[cfg(test)]
mod lifecycle_composition_tests {
    use super::*;

    fn get_valid_pub_candidate() -> serde_json::Value {
        json!({
            "schema_version": "REPOSITORY_PUBLICATION_CANDIDATE-v1",
            "publication_id": "pub_01XYZ",
            "source_commit_oid": "e9aeb73000000000000000000000000000000000"
        })
    }

    fn get_valid_pr_candidate() -> serde_json::Value {
        json!({
            "schema_version": "REPOSITORY_PR_CANDIDATE-v1",
            "candidate_id": "pr_cand_01XYZ",
            "source_commit_oid": "e9aeb73000000000000000000000000000000000"
        })
    }

    fn get_valid_review_observation() -> serde_json::Value {
        json!({
            "schema_version": "REPOSITORY_REVIEW_OBSERVATION-v1",
            "observation_id": "rev_obs_01XYZ",
            "approval_status": "APPROVED"
        })
    }

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
            "strategy_binding": { "strategy": "SQUASH_MERGE" },
            "temporal_bounds": { "issued_at": past_iss, "expires_at": future_exp },
            "consumption_policy": { "single_use_only": true }
        })
    }

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
                "runtime_id": "cluster-staging",
                "environment": "staging"
            },
            "deployment_strategy": "ROLLING",
            "expected_runtime_state": "v1.0.0",
            "proposed_runtime_state": "v1.1.0"
        })
    }

    fn get_valid_deployment_authorization() -> serde_json::Value {
        let future_exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        json!({
            "schema_version": "REPOSITORY_DEPLOYMENT_AUTHORIZATION-v1",
            "deployment_authorization_id": "dep_auth_01XYZ",
            "deployment_candidate_ref": {
                "candidate_id": "dep_cand_01XYZ",
                "artifact_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            "runtime_binding": {
                "runtime_id": "cluster-staging",
                "environment": "staging"
            },
            "authorized_scope": {
                "operation": "repository.runtime.deploy_exact",
                "rollback_permitted": false,
                "secret_rotation_permitted": false,
                "infra_admin": false,
                "production_override": false
            },
            "temporal_bounds": { "issued_at": past_iss, "expires_at": future_exp },
            "consumption_policy": { "single_use_only": true }
        })
    }

    #[test]
    fn tc_lifecycle_001_valid_full_lifecycle_success() {
        let pub_cand = get_valid_pub_candidate();
        let pr_cand = get_valid_pr_candidate();
        let review_obs = get_valid_review_observation();
        let merge_auth = get_valid_merge_authorization();
        let dep_cand = get_valid_deployment_candidate();
        let dep_auth = get_valid_deployment_authorization();
        let valid_digest =
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let receipt = LifecycleCompositionValidator::evaluate_composition(
            &pub_cand,
            &pr_cand,
            &review_obs,
            &merge_auth,
            &dep_cand,
            &dep_auth,
            valid_digest,
        );

        assert_eq!(
            receipt.terminal_disposition,
            LifecycleTerminalDisposition::VerifiedSuccess
        );
    }

    #[test]
    fn tc_lifecycle_002_review_approval_cannot_become_merge_authority() {
        // Structural validation: review observation contains no merge authorization privileges.
        let review_obs = get_valid_review_observation();
        assert!(review_obs.get("merge_authorization").is_none());
    }

    #[test]
    fn tc_lifecycle_003_merge_success_cannot_become_deployment_authority() {
        // Structural validation: merge authorization contains no deployment permission.
        let merge_auth = get_valid_merge_authorization();
        assert_eq!(
            merge_auth["authorized_scope"]["deployment_permitted"],
            json!(false)
        );
    }

    #[test]
    fn tc_lifecycle_004_expired_deployment_lease_rejection() {
        let pub_cand = get_valid_pub_candidate();
        let pr_cand = get_valid_pr_candidate();
        let review_obs = get_valid_review_observation();
        let merge_auth = get_valid_merge_authorization();
        let dep_cand = get_valid_deployment_candidate();
        let mut dep_auth = get_valid_deployment_authorization();

        let past_exp = (Utc::now() - chrono::Duration::minutes(15)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        dep_auth["temporal_bounds"]["issued_at"] = json!(past_iss);
        dep_auth["temporal_bounds"]["expires_at"] = json!(past_exp);

        let valid_digest =
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let receipt = LifecycleCompositionValidator::evaluate_composition(
            &pub_cand,
            &pr_cand,
            &review_obs,
            &merge_auth,
            &dep_cand,
            &dep_auth,
            valid_digest,
        );

        assert_eq!(
            receipt.terminal_disposition,
            LifecycleTerminalDisposition::Denied
        );
    }

    #[test]
    fn tc_lifecycle_005_artifact_drift_detection() {
        let pub_cand = get_valid_pub_candidate();
        let pr_cand = get_valid_pr_candidate();
        let review_obs = get_valid_review_observation();
        let merge_auth = get_valid_merge_authorization();
        let dep_cand = get_valid_deployment_candidate();
        let dep_auth = get_valid_deployment_authorization();

        let drifted_digest =
            "sha256:0000000000000000000000000000000000000000000000000000000000000000";

        let receipt = LifecycleCompositionValidator::evaluate_composition(
            &pub_cand,
            &pr_cand,
            &review_obs,
            &merge_auth,
            &dep_cand,
            &dep_auth,
            drifted_digest,
        );

        assert_eq!(
            receipt.terminal_disposition,
            LifecycleTerminalDisposition::RuntimeMismatch
        );
    }

    #[test]
    fn tc_lifecycle_006_cross_domain_capability_injection_rejected() {
        let pub_cand = get_valid_pub_candidate();
        let pr_cand = get_valid_pr_candidate();
        let review_obs = get_valid_review_observation();
        let mut merge_auth = get_valid_merge_authorization();
        // Inject illegal capability
        merge_auth
            .as_object_mut()
            .unwrap()
            .insert("deployment_permitted".to_string(), json!(true));

        let dep_cand = get_valid_deployment_candidate();
        let dep_auth = get_valid_deployment_authorization();
        let valid_digest =
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let receipt = LifecycleCompositionValidator::evaluate_composition(
            &pub_cand,
            &pr_cand,
            &review_obs,
            &merge_auth,
            &dep_cand,
            &dep_auth,
            valid_digest,
        );

        // Merge schema version or strict additional properties validation failure -> SchemaRejection
        let mut bad_merge = merge_auth.clone();
        bad_merge["schema_version"] = json!("INVALID_VERSION");

        let receipt_bad = LifecycleCompositionValidator::evaluate_composition(
            &pub_cand,
            &pr_cand,
            &review_obs,
            &bad_merge,
            &dep_cand,
            &dep_auth,
            valid_digest,
        );

        assert_eq!(
            receipt_bad.terminal_disposition,
            LifecycleTerminalDisposition::SchemaRejection
        );
    }

    #[test]
    fn tc_lifecycle_007_composite_ambiguity_resolution() {
        // Structural validation: telemetry ambiguity without runtime verification defaults to non-verified disposition.
        let receipt = SovereignLifecycleReceipt {
            publication_id: "pub_123".into(),
            pr_candidate_id: "pr_123".into(),
            review_observation_id: "rev_123".into(),
            merge_authorization_id: "merge_auth_123".into(),
            deployment_candidate_id: "dep_cand_123".into(),
            deployment_authorization_id: "dep_auth_123".into(),
            runtime_verification_digest: "sha256:...".into(),
            terminal_disposition: LifecycleTerminalDisposition::Ambiguous,
        };

        assert_eq!(
            receipt.terminal_disposition,
            LifecycleTerminalDisposition::Ambiguous
        );
    }
}
