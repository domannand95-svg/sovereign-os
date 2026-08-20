use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;

// =====================================================================
// 1. WORKFLOW AUTHORIZATION & COORDINATION ENGINE DOMAIN TYPES
// =====================================================================

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum WorkflowStateTransition {
    ObservedStateAdvanced,
    EvidenceAggregated,
    EvaluationRequested,
    Denied,
    Expired,
    ScopeMismatch,
}

pub struct WorkflowAuthorizationValidator;

impl WorkflowAuthorizationValidator {
    pub fn validate(value: &serde_json::Value) -> Result<(), String> {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_GOVERNANCE_WORKFLOW_AUTHORIZATION-v1")
        {
            return Err("Invalid or missing schema_version".into());
        }

        // Validate workflow_authorization_id pattern
        if let Some(id) = value
            .get("workflow_authorization_id")
            .and_then(|v| v.as_str())
        {
            if !id.starts_with("wf_auth_") {
                return Err("Invalid workflow_authorization_id format".into());
            }
        } else {
            return Err("Missing workflow_authorization_id".into());
        }

        // Validate workflow_candidate_ref presence
        if value.get("workflow_candidate_ref").is_none() {
            return Err("Missing workflow_candidate_ref binding".into());
        }

        // Validate temporal bounds and expiration
        if let Some(bounds) = value.get("temporal_bounds") {
            let expires_str = bounds.get("expires_at").and_then(|v| v.as_str());
            if let Some(exp) = expires_str {
                if let Ok(e_dt) = DateTime::parse_from_rfc3339(exp) {
                    if e_dt < Utc::now() {
                        return Err("Workflow authorization expired".into());
                    }
                }
            }
        }

        // AUTHORITY INJECTION & EXECUTION CAPABILITY REJECTION:
        // Ensure no deployment or merge capabilities are permitted.
        if let Some(scope) = value.get("authorized_scope") {
            if scope.get("execute_deployment").and_then(|v| v.as_bool()) == Some(true) {
                return Err("Violation: execute_deployment must be false".into());
            }
            if scope.get("merge_repository").and_then(|v| v.as_bool()) == Some(true) {
                return Err("Violation: merge_repository must be false".into());
            }
        }

        // General key check
        let allowed_keys = [
            "schema_version",
            "workflow_authorization_id",
            "workflow_candidate_ref",
            "authorized_scope",
            "temporal_bounds",
            "consumption_policy",
        ];
        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return Err(format!("Unauthorized coordination field injected: {}", key));
                }
            }
        }

        Ok(())
    }
}

pub trait GovernanceCoordinator {
    fn advance_workflow(
        &self,
        candidate_digest: &str,
        authorization: &serde_json::Value,
    ) -> WorkflowStateTransition;
}

pub struct StandardGovernanceCoordinator;

impl GovernanceCoordinator for StandardGovernanceCoordinator {
    fn advance_workflow(
        &self,
        candidate_digest: &str,
        authorization: &serde_json::Value,
    ) -> WorkflowStateTransition {
        if WorkflowAuthorizationValidator::validate(authorization).is_err() {
            return WorkflowStateTransition::Denied;
        }

        // Verify candidate digest binding
        let auth_digest = authorization
            .get("workflow_candidate_ref")
            .and_then(|r| r.get("candidate_digest"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if auth_digest != candidate_digest {
            return WorkflowStateTransition::ScopeMismatch;
        }

        WorkflowStateTransition::ObservedStateAdvanced
    }
}

// =====================================================================
// 2. ADVERSARIAL VALIDATION SUITE (TC-WORKFLOW-AUTH-001..007)
// =====================================================================

#[cfg(test)]
mod workflow_authorization_tests {
    use super::*;

    fn get_valid_workflow_authorization() -> serde_json::Value {
        let future_exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();

        json!({
            "schema_version": "REPOSITORY_GOVERNANCE_WORKFLOW_AUTHORIZATION-v1",
            "workflow_authorization_id": "wf_auth_01XYZ",
            "workflow_candidate_ref": {
                "workflow_candidate_id": "wf_cand_01ABC",
                "candidate_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            "authorized_scope": {
                "operation": "repository.governance.coordinate_exact",
                "observe_lifecycle": true,
                "aggregate_evidence": true,
                "request_evaluation": true,
                "advance_state": true,
                "execute_deployment": false,
                "merge_repository": false,
                "publish_artifact": false
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
    fn tc_workflow_auth_001_valid_coordination_authorization_accepted() {
        let auth = get_valid_workflow_authorization();
        assert!(WorkflowAuthorizationValidator::validate(&auth).is_ok());
    }

    #[test]
    fn tc_workflow_auth_002_missing_candidate_binding_rejected() {
        let mut auth = get_valid_workflow_authorization();
        auth.as_object_mut()
            .unwrap()
            .remove("workflow_candidate_ref");
        assert!(WorkflowAuthorizationValidator::validate(&auth).is_err());
    }

    #[test]
    fn tc_workflow_auth_003_injected_deployment_permission_rejected() {
        let mut auth = get_valid_workflow_authorization();
        auth["authorized_scope"]["execute_deployment"] = json!(true);
        assert!(WorkflowAuthorizationValidator::validate(&auth).is_err());
    }

    #[test]
    fn tc_workflow_auth_004_injected_merge_capability_rejected() {
        let mut auth = get_valid_workflow_authorization();
        auth["authorized_scope"]["merge_repository"] = json!(true);
        assert!(WorkflowAuthorizationValidator::validate(&auth).is_err());
    }

    #[test]
    fn tc_workflow_auth_005_credential_injection_rejected() {
        let mut auth = get_valid_workflow_authorization();
        auth.as_object_mut()
            .unwrap()
            .insert("token".to_string(), json!("secret_key_123"));
        assert!(WorkflowAuthorizationValidator::validate(&auth).is_err());
    }

    #[test]
    fn tc_workflow_auth_006_workflow_scope_mismatch() {
        let coordinator = StandardGovernanceCoordinator;
        let auth = get_valid_workflow_authorization();
        let wrong_digest =
            "sha256:0000000000000000000000000000000000000000000000000000000000000000";

        let transition = coordinator.advance_workflow(wrong_digest, &auth);
        assert_eq!(transition, WorkflowStateTransition::ScopeMismatch);
    }

    #[test]
    fn tc_workflow_auth_007_expired_coordination_lease_rejected() {
        let mut auth = get_valid_workflow_authorization();
        let past_exp = (Utc::now() - chrono::Duration::minutes(15)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        auth["temporal_bounds"]["issued_at"] = json!(past_iss);
        auth["temporal_bounds"]["expires_at"] = json!(past_exp);

        assert!(WorkflowAuthorizationValidator::validate(&auth).is_err());
    }
}
