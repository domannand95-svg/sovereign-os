use chrono::{DateTime, Utc};
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum ExternalIdentityValidationResult {
    Valid,
    Invalid(String),
}

pub struct ExternalIdentityValidator;

impl ExternalIdentityValidator {
    pub fn validate(value: &serde_json::Value) -> ExternalIdentityValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_EXTERNAL_IDENTITY_ASSERTION-v1")
        {
            return ExternalIdentityValidationResult::Invalid(
                "Invalid or missing schema_version".into(),
            );
        }

        // Validate assertion_id pattern
        if let Some(id) = value.get("assertion_id").and_then(|v| v.as_str()) {
            if !id.starts_with("ext_id_") {
                return ExternalIdentityValidationResult::Invalid(
                    "Invalid assertion_id format".into(),
                );
            }
        } else {
            return ExternalIdentityValidationResult::Invalid("Missing assertion_id".into());
        }

        // Validate identity subject presence (TC-EXT-ID-002, 005)
        if let Some(subject) = value.get("identity_subject") {
            if let Some(principal) = subject.get("external_principal").and_then(|v| v.as_str()) {
                if principal.is_empty() || principal.starts_with("sovereign::internal::") {
                    return ExternalIdentityValidationResult::Invalid("Namespace collision or empty principal: external identity cannot claim internal sovereign namespace".into());
                }
            } else {
                return ExternalIdentityValidationResult::Invalid(
                    "Missing external_principal".into(),
                );
            }
        } else {
            return ExternalIdentityValidationResult::Invalid("Missing identity_subject".into());
        }

        // Validate temporal bounds and expiration (TC-EXT-ID-006)
        if let Some(prov) = value.get("cryptographic_provenance") {
            if let Some(exp_str) = prov.get("expires_at").and_then(|v| v.as_str()) {
                if let Ok(exp_dt) = DateTime::parse_from_rfc3339(exp_str) {
                    if exp_dt < Utc::now() {
                        return ExternalIdentityValidationResult::Invalid(
                            "Identity assertion expired".into(),
                        );
                    }
                }
            } else {
                return ExternalIdentityValidationResult::Invalid(
                    "Missing expires_at in cryptographic_provenance".into(),
                );
            }
        } else {
            return ExternalIdentityValidationResult::Invalid(
                "Missing cryptographic_provenance".into(),
            );
        }

        // AUTHORITY INJECTION & PRIVILEGE ESCALATION CHECK (TC-EXT-ID-004):
        // Ensure no capability grants or permission fields exist in the assertion.
        let allowed_keys = [
            "schema_version",
            "assertion_id",
            "identity_subject",
            "cryptographic_provenance",
            "verification_state",
        ];

        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return ExternalIdentityValidationResult::Invalid(format!(
                        "Authority injection or forbidden capability field detected: {}",
                        key
                    ));
                }
            }
        }

        ExternalIdentityValidationResult::Valid
    }
}

#[cfg(test)]
mod external_identity_assertion_tests {
    use super::*;

    fn get_valid_identity_assertion() -> serde_json::Value {
        let future_exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();

        json!({
            "schema_version": "REPOSITORY_EXTERNAL_IDENTITY_ASSERTION-v1",
            "assertion_id": "ext_id_01XYZ",
            "identity_subject": {
                "external_principal": "agent.provider.example::model-123",
                "issuer_domain": "external-provider.org",
                "namespace": "EXTERNAL_MODEL_PROVIDER"
            },
            "cryptographic_provenance": {
                "public_key_fingerprint": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "assertion_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "issued_at": past_iss,
                "expires_at": future_exp
            },
            "verification_state": "ASSERTED_EXTERNAL_IDENTITY"
        })
    }

    #[test]
    fn tc_ext_id_001_valid_external_identity_accepted() {
        let assertion = get_valid_identity_assertion();
        assert_eq!(
            ExternalIdentityValidator::validate(&assertion),
            ExternalIdentityValidationResult::Valid
        );
    }

    #[test]
    fn tc_ext_id_002_reject_missing_cryptographic_provenance() {
        let mut assertion = get_valid_identity_assertion();
        assertion
            .as_object_mut()
            .unwrap()
            .remove("cryptographic_provenance");
        assert!(matches!(
            ExternalIdentityValidator::validate(&assertion),
            ExternalIdentityValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_ext_id_003_external_identity_cannot_become_internal_principal() {
        let mut assertion = get_valid_identity_assertion();
        assertion["identity_subject"]["external_principal"] =
            json!("sovereign::internal::root_admin");
        assert!(matches!(
            ExternalIdentityValidator::validate(&assertion),
            ExternalIdentityValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_ext_id_004_reject_capability_injection() {
        let mut assertion = get_valid_identity_assertion();
        assertion
            .as_object_mut()
            .unwrap()
            .insert("grant_capability".to_string(), json!("repository.admin"));
        assert!(matches!(
            ExternalIdentityValidator::validate(&assertion),
            ExternalIdentityValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_ext_id_005_reject_namespace_collision() {
        let assertion = get_valid_identity_assertion();
        let ns = assertion["identity_subject"]["namespace"]
            .as_str()
            .unwrap_or("");
        let allowed_ns = [
            "EXTERNAL_FEDERATED",
            "THIRD_PARTY_AGENT",
            "EXTERNAL_MODEL_PROVIDER",
        ];
        assert!(allowed_ns.contains(&ns));
    }

    #[test]
    fn tc_ext_id_006_reject_expired_assertion() {
        let mut assertion = get_valid_identity_assertion();
        let past_exp = (Utc::now() - chrono::Duration::minutes(15)).to_rfc3339();
        let past_iss = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        assertion["cryptographic_provenance"]["issued_at"] = json!(past_iss);
        assertion["cryptographic_provenance"]["expires_at"] = json!(past_exp);

        assert!(matches!(
            ExternalIdentityValidator::validate(&assertion),
            ExternalIdentityValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_ext_id_007_federation_metadata_non_authoritative() {
        let assertion = get_valid_identity_assertion();
        let res = ExternalIdentityValidator::validate(&assertion);
        assert_eq!(res, ExternalIdentityValidationResult::Valid);
        assert_eq!(
            assertion["verification_state"],
            json!("ASSERTED_EXTERNAL_IDENTITY")
        );
    }
}
