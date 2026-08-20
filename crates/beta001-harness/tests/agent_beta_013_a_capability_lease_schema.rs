use chrono::{DateTime, Utc};
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentLeaseValidationResult {
    Valid,
    Invalid(String),
}

pub struct AgentLeaseValidator;

impl AgentLeaseValidator {
    pub fn validate(value: &serde_json::Value) -> AgentLeaseValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_AGENT_CAPABILITY_LEASE-v1")
        {
            return AgentLeaseValidationResult::Invalid("Invalid or missing schema_version".into());
        }

        // Validate lease_id pattern
        if let Some(id) = value.get("lease_id").and_then(|v| v.as_str()) {
            if !id.starts_with("lease_") {
                return AgentLeaseValidationResult::Invalid("Invalid lease_id format".into());
            }
        } else {
            return AgentLeaseValidationResult::Invalid("Missing lease_id".into());
        }

        // Validate objective reference binding (TC-AGENT-LEASE-003)
        if value
            .get("objective_reference")
            .and_then(|v| v.as_str())
            .map_or(true, |s| s.is_empty())
        {
            return AgentLeaseValidationResult::Invalid(
                "Capability lease must be bound to a declared objective reference".into(),
            );
        }

        // Validate temporal bounds and expiration (TC-AGENT-LEASE-002)
        if let Some(bounds) = value.get("temporal_bounds") {
            if let Some(exp_str) = bounds.get("expires_at").and_then(|v| v.as_str()) {
                if let Ok(exp_dt) = DateTime::parse_from_rfc3339(exp_str) {
                    if exp_dt < Utc::now() {
                        return AgentLeaseValidationResult::Invalid("Lease expired".into());
                    }
                }
            }
        } else {
            return AgentLeaseValidationResult::Invalid("Missing temporal bounds".into());
        }

        // AUTHORITY ESCALATION & CREDENTIAL INJECTION CHECK (TC-AGENT-LEASE-004, 005, 006):
        // Ensure no embedded credentials, admin grants, wildcards, or bypass flags exist.
        let allowed_keys = [
            "schema_version",
            "lease_id",
            "agent_identity",
            "capability_scope",
            "objective_reference",
            "temporal_bounds",
            "issuing_authority_reference",
        ];

        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return AgentLeaseValidationResult::Invalid(format!(
                        "Authority escalation or credential injection detected: {}",
                        key
                    ));
                }
            }
        }

        // Check capability scope for forbidden wildcards or excessive permissions
        if let Some(scopes) = value.get("capability_scope").and_then(|v| v.as_array()) {
            if scopes.is_empty() {
                return AgentLeaseValidationResult::Invalid(
                    "Capability scope cannot be empty".into(),
                );
            }
            for scope in scopes {
                if let Some(s_str) = scope.as_str() {
                    if s_str == "all" || s_str.contains('*') {
                        return AgentLeaseValidationResult::Invalid(
                            "Wildcard capability scope forbidden".into(),
                        );
                    }
                }
            }
        }

        AgentLeaseValidationResult::Valid
    }
}

#[cfg(test)]
mod agent_capability_lease_tests {
    use super::*;

    fn get_valid_lease() -> serde_json::Value {
        let future_exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();

        json!({
            "schema_version": "REPOSITORY_AGENT_CAPABILITY_LEASE-v1",
            "lease_id": "lease_01XYZ",
            "agent_identity": {
                "agent_id": "agent_alpha",
                "provenance_ref": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            "capability_scope": [
                "REPOSITORY_READ",
                "EVIDENCE_QUERY"
            ],
            "objective_reference": "OBJ-013-001",
            "temporal_bounds": {
                "issued_at": past_iss,
                "expires_at": future_exp,
                "single_use_only": true
            },
            "issuing_authority_reference": "auth_root_01"
        })
    }

    #[test]
    fn tc_agent_lease_001_valid_lease_accepted() {
        let lease = get_valid_lease();
        assert_eq!(
            AgentLeaseValidator::validate(&lease),
            AgentLeaseValidationResult::Valid
        );
    }

    #[test]
    fn tc_agent_lease_002_reject_expired_lease() {
        let mut lease = get_valid_lease();
        let past_exp = (Utc::now() - chrono::Duration::minutes(15)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        lease["temporal_bounds"]["issued_at"] = json!(past_iss);
        lease["temporal_bounds"]["expires_at"] = json!(past_exp);

        assert!(matches!(
            AgentLeaseValidator::validate(&lease),
            AgentLeaseValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_lease_003_reject_missing_objective_binding() {
        let mut lease = get_valid_lease();
        lease["objective_reference"] = json!("");
        assert!(matches!(
            AgentLeaseValidator::validate(&lease),
            AgentLeaseValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_lease_004_reject_embedded_credentials() {
        let mut lease = get_valid_lease();
        lease
            .as_object_mut()
            .unwrap()
            .insert("token".to_string(), json!("secret_api_key_123"));
        assert!(matches!(
            AgentLeaseValidator::validate(&lease),
            AgentLeaseValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_lease_005_reject_authority_escalation_fields() {
        let mut lease = get_valid_lease();
        lease
            .as_object_mut()
            .unwrap()
            .insert("grant_admin".to_string(), json!(true));
        assert!(matches!(
            AgentLeaseValidator::validate(&lease),
            AgentLeaseValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_lease_006_reject_unlimited_wildcard() {
        let mut lease = get_valid_lease();
        lease["capability_scope"] = json!(["all"]);
        assert!(matches!(
            AgentLeaseValidator::validate(&lease),
            AgentLeaseValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_lease_007_reject_delegation_chain_expansion() {
        let mut lease = get_valid_lease();
        lease
            .as_object_mut()
            .unwrap()
            .insert("inherit_parent_capabilities".to_string(), json!(true));
        assert!(matches!(
            AgentLeaseValidator::validate(&lease),
            AgentLeaseValidationResult::Invalid(_)
        ));
    }
}
