use serde_json::{json, Value};

pub struct AuthorizationSemanticValidator;

impl AuthorizationSemanticValidator {
    pub fn validate(auth: &Value) -> Result<(), String> {
        let allowed_keys = [
            "schema_version", "authorization_id", "purpose", "authorized_candidate",
            "temporal_bounds", "authorized_scope", "consumption_policy"
        ];
        
        if let Some(obj) = auth.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return Err(format!("Extraneous property found: {}", key));
                }
            }
        }

        let cand = auth.get("authorized_candidate").ok_or("AUTHORIZATION_INVALID: Missing authorized_candidate")?;
        
        // Enforce candidate schema version equality strictly
        if cand.get("candidate_schema_version").and_then(|v| v.as_str()) != Some("REPOSITORY_PUBLICATION_CANDIDATE-v1") {
            return Err("AUTHORIZATION_INVALID: candidate_schema_version mismatch".into());
        }

        let scope = auth.get("authorized_scope").ok_or("AUTHORIZATION_INVALID: Missing authorized_scope")?;
        if scope.get("operation").and_then(|v| v.as_str()) != Some("repository.remote.publish_exact") {
            return Err("AUTHORIZATION_INVALID: Unknown or substituted operation".into());
        }

        // Temporal bounds validation (ISO 8601 lexicographical sort)
        let bounds = auth.get("temporal_bounds").ok_or("AUTHORIZATION_INVALID: Missing temporal_bounds")?;
        let issued = bounds.get("issued_at").and_then(|v| v.as_str()).ok_or("Missing issued_at")?;
        let expires = bounds.get("expires_at").and_then(|v| v.as_str()).ok_or("Missing expires_at")?;

        if expires <= issued {
            return Err("AUTHORIZATION_INVALID: expires_at must be strictly after issued_at".into());
        }

        // Consumption check
        let consumption = auth.get("consumption_policy").ok_or("AUTHORIZATION_INVALID: Missing consumption_policy")?;
        if consumption.get("consumption_id").is_none() {
            return Err("AUTHORIZATION_INVALID: Missing consumption_id".into());
        }

        Ok(())
    }
}

fn valid_authorization_base() -> Value {
    json!({
        "schema_version": "REPOSITORY_PUBLICATION_AUTHORIZATION-v1",
        "authorization_id": "pub_auth_001",
        "purpose": "REMOTE_PUBLICATION",
        "authorized_candidate": {
            "candidate_schema_version": "REPOSITORY_PUBLICATION_CANDIDATE-v1",
            "candidate_id": "pub_cand_001",
            "candidate_commit_oid": "cccccccccccccccccccccccccccccccccccccccc"
        },
        "temporal_bounds": {
            "issued_at": "2026-08-20T10:00:00Z",
            "expires_at": "2026-08-20T11:00:00Z"
        },
        "authorized_scope": {
            "operation": "repository.remote.publish_exact"
        },
        "consumption_policy": {
            "single_use_only": true,
            "consumption_id": "pub_consume_abc123"
        }
    })
}

#[test]
fn test_tc_pub_auth_001_valid_authorization_accepted() {
    assert!(AuthorizationSemanticValidator::validate(&valid_authorization_base()).is_ok());
}

#[test]
fn test_tc_pub_auth_002_missing_candidate_rejected() {
    let mut auth = valid_authorization_base();
    auth.as_object_mut().unwrap().remove("authorized_candidate");
    let res = AuthorizationSemanticValidator::validate(&auth);
    assert_eq!(res.unwrap_err(), "AUTHORIZATION_INVALID: Missing authorized_candidate");
}

#[test]
fn test_tc_pub_auth_005_temporal_bound_inversion_rejected() {
    let mut auth = valid_authorization_base();
    auth["temporal_bounds"]["issued_at"] = json!("2026-08-20T10:00:00Z");
    auth["temporal_bounds"]["expires_at"] = json!("2026-08-19T10:00:00Z");
    let res = AuthorizationSemanticValidator::validate(&auth);
    assert_eq!(res.unwrap_err(), "AUTHORIZATION_INVALID: expires_at must be strictly after issued_at");
}

#[test]
fn test_tc_pub_auth_006_credential_material_injection_rejected() {
    let mut auth = valid_authorization_base();
    auth["credential_id"] = json!("secret_token");
    let res = AuthorizationSemanticValidator::validate(&auth);
    assert!(res.unwrap_err().contains("Extraneous property"));
}

#[test]
fn test_tc_pub_auth_007_unknown_operation_authority_injected() {
    let mut auth = valid_authorization_base();
    auth["authorized_scope"]["operation"] = json!("repository.remote.delete_ref");
    let res = AuthorizationSemanticValidator::validate(&auth);
    assert_eq!(res.unwrap_err(), "AUTHORIZATION_INVALID: Unknown or substituted operation");
}

#[test]
fn test_tc_pub_auth_008_candidate_schema_version_mismatch() {
    let mut auth = valid_authorization_base();
    auth["authorized_candidate"]["candidate_schema_version"] = json!("REPOSITORY_PUBLICATION_CANDIDATE-v2");
    let res = AuthorizationSemanticValidator::validate(&auth);
    assert_eq!(res.unwrap_err(), "AUTHORIZATION_INVALID: candidate_schema_version mismatch");
}

#[test]
fn test_tc_pub_auth_009_authorization_replay_identity_missing() {
    let mut auth = valid_authorization_base();
    auth["consumption_policy"].as_object_mut().unwrap().remove("consumption_id");
    let res = AuthorizationSemanticValidator::validate(&auth);
    assert_eq!(res.unwrap_err(), "AUTHORIZATION_INVALID: Missing consumption_id");
}

#[test]
fn test_tc_pub_auth_010_merge_operation_substituted() {
    let mut auth = valid_authorization_base();
    auth["authorized_scope"]["operation"] = json!("repository.remote.merge");
    let res = AuthorizationSemanticValidator::validate(&auth);
    assert_eq!(res.unwrap_err(), "AUTHORIZATION_INVALID: Unknown or substituted operation");
}
