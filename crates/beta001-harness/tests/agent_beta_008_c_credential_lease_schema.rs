use serde_json::{json, Value};

pub struct CredentialLeaseSemanticValidator;

impl CredentialLeaseSemanticValidator {
    pub fn validate(
        lease: &Value,
        expected_provider: &str,
        expected_auth_id: &str,
    ) -> Result<(), String> {
        let allowed_keys = [
            "schema_version",
            "lease_id",
            "credential_capability_id",
            "authorized_use_reference",
            "provider",
            "principal_identity",
            "temporal_bounds",
            "consumption_policy",
            "broker_reference",
            "technical_scope",
        ];

        if let Some(obj) = lease.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return Err(format!("Extraneous property found: {}", key));
                }
            }
        }

        // Structural presence checks
        let _broker_ref = lease
            .get("broker_reference")
            .ok_or("CREDENTIAL_INVALID: Missing broker_reference")?;
        let _cap_id = lease
            .get("credential_capability_id")
            .ok_or("CREDENTIAL_INVALID: Missing credential_capability_id")?;

        // Authorization context check
        let auth_ref = lease
            .get("authorized_use_reference")
            .and_then(|v| v.as_str())
            .ok_or("CREDENTIAL_INVALID: Missing authorized_use_reference")?;
        if auth_ref != expected_auth_id {
            return Err("CREDENTIAL_INVALID: Authorization binding mismatch".into());
        }

        // Provider context check
        let provider = lease
            .get("provider")
            .and_then(|v| v.as_str())
            .ok_or("CREDENTIAL_INVALID: Missing provider")?;
        if provider != expected_provider {
            return Err("CREDENTIAL_INVALID: Provider mismatch".into());
        }

        let principal_provider = lease["principal_identity"]["provider"]
            .as_str()
            .unwrap_or("");
        if principal_provider != expected_provider {
            return Err("CREDENTIAL_INVALID: Principal provider mismatch".into());
        }

        // Technical scope check (controlled vocabulary)
        let allowed_scopes = ["contents:read", "contents:write", "metadata:read"];
        if let Some(scopes) = lease.get("technical_scope").and_then(|v| v.as_array()) {
            if scopes.is_empty() {
                return Err("CREDENTIAL_INVALID: technical_scope cannot be empty".into());
            }
            for scope in scopes {
                let scope_str = scope.as_str().unwrap_or("");
                if !allowed_scopes.contains(&scope_str) {
                    return Err(format!(
                        "CREDENTIAL_INVALID: Unknown technical scope '{}'",
                        scope_str
                    ));
                }
            }
        } else {
            return Err("CREDENTIAL_INVALID: Missing technical_scope".into());
        }

        // Temporal bounds validation
        let bounds = lease
            .get("temporal_bounds")
            .ok_or("CREDENTIAL_INVALID: Missing temporal_bounds")?;
        let issued = bounds
            .get("issued_at")
            .and_then(|v| v.as_str())
            .ok_or("Missing issued_at")?;
        let expires = bounds
            .get("expires_at")
            .and_then(|v| v.as_str())
            .ok_or("Missing expires_at")?;
        if expires <= issued {
            return Err("CREDENTIAL_INVALID: expires_at must be strictly after issued_at".into());
        }

        // Consumption policy check
        let consumption = lease
            .get("consumption_policy")
            .ok_or("CREDENTIAL_INVALID: Missing consumption_policy")?;
        if consumption.get("single_use_only").and_then(|v| v.as_bool()) != Some(true) {
            return Err("CREDENTIAL_INVALID: Lease must be single_use_only".into());
        }

        Ok(())
    }
}

fn valid_lease_base() -> Value {
    json!({
        "schema_version": "REPOSITORY_CREDENTIAL_LEASE-v1",
        "lease_id": "cred_lease_001",
        "credential_capability_id": "cred_cap_123",
        "authorized_use_reference": "pub_auth_001",
        "provider": "github.com",
        "principal_identity": {
            "provider": "github.com",
            "principal_type": "github_app_installation",
            "principal_id": "inst_789"
        },
        "temporal_bounds": {
            "issued_at": "2026-08-20T10:00:00Z",
            "expires_at": "2026-08-20T10:05:00Z"
        },
        "consumption_policy": {
            "single_use_only": true
        },
        "broker_reference": "broker_ref_abc",
        "technical_scope": ["contents:write"]
    })
}

#[test]
fn test_tc_cred_lease_001_valid_lease_accepted() {
    assert!(CredentialLeaseSemanticValidator::validate(
        &valid_lease_base(),
        "github.com",
        "pub_auth_001"
    )
    .is_ok());
}

#[test]
fn test_tc_cred_lease_002_missing_broker_reference_rejected() {
    let mut lease = valid_lease_base();
    lease.as_object_mut().unwrap().remove("broker_reference");
    let res = CredentialLeaseSemanticValidator::validate(&lease, "github.com", "pub_auth_001");
    assert_eq!(
        res.unwrap_err(),
        "CREDENTIAL_INVALID: Missing broker_reference"
    );
}

#[test]
fn test_tc_cred_lease_003_raw_token_injection_rejected() {
    let mut lease = valid_lease_base();
    lease["token"] = json!("ghp_secret_string");
    let res = CredentialLeaseSemanticValidator::validate(&lease, "github.com", "pub_auth_001");
    assert!(res.unwrap_err().contains("Extraneous property"));
}

#[test]
fn test_tc_cred_lease_004_temporal_inversion_rejected() {
    let mut lease = valid_lease_base();
    lease["temporal_bounds"]["expires_at"] = json!("2026-08-19T10:00:00Z");
    let res = CredentialLeaseSemanticValidator::validate(&lease, "github.com", "pub_auth_001");
    assert_eq!(
        res.unwrap_err(),
        "CREDENTIAL_INVALID: expires_at must be strictly after issued_at"
    );
}

#[test]
fn test_tc_cred_lease_005_provider_mismatch_rejected() {
    let lease = valid_lease_base();
    let res = CredentialLeaseSemanticValidator::validate(&lease, "gitlab.com", "pub_auth_001");
    assert_eq!(res.unwrap_err(), "CREDENTIAL_INVALID: Provider mismatch");
}

#[test]
fn test_tc_cred_lease_006_credential_capability_identity_missing_rejected() {
    let mut lease = valid_lease_base();
    lease
        .as_object_mut()
        .unwrap()
        .remove("credential_capability_id");
    let res = CredentialLeaseSemanticValidator::validate(&lease, "github.com", "pub_auth_001");
    assert_eq!(
        res.unwrap_err(),
        "CREDENTIAL_INVALID: Missing credential_capability_id"
    );
}

#[test]
fn test_tc_cred_lease_007_authorization_binding_mismatch_rejected() {
    let lease = valid_lease_base();
    let res = CredentialLeaseSemanticValidator::validate(&lease, "github.com", "pub_auth_002");
    assert_eq!(
        res.unwrap_err(),
        "CREDENTIAL_INVALID: Authorization binding mismatch"
    );
}

#[test]
fn test_tc_cred_lease_008_technical_scope_outside_controlled_vocabulary_rejected() {
    let mut lease = valid_lease_base();
    lease["technical_scope"] = json!(["admin:org"]);
    let res = CredentialLeaseSemanticValidator::validate(&lease, "github.com", "pub_auth_001");
    assert_eq!(
        res.unwrap_err(),
        "CREDENTIAL_INVALID: Unknown technical scope 'admin:org'"
    );
}

#[test]
fn test_tc_cred_lease_009_lease_replay_consumption_violation_rejected() {
    let mut lease = valid_lease_base();
    lease["consumption_policy"]["single_use_only"] = json!(false);
    let res = CredentialLeaseSemanticValidator::validate(&lease, "github.com", "pub_auth_001");
    assert_eq!(
        res.unwrap_err(),
        "CREDENTIAL_INVALID: Lease must be single_use_only"
    );
}
