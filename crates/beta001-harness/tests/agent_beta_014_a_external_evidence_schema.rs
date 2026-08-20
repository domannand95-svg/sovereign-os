use chrono::Utc;
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum ExternalEvidenceValidationResult {
    Valid,
    Invalid(String),
}

pub struct ExternalEvidenceValidator;

impl ExternalEvidenceValidator {
    pub fn validate(value: &serde_json::Value) -> ExternalEvidenceValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_EXTERNAL_EVIDENCE-v1")
        {
            return ExternalEvidenceValidationResult::Invalid(
                "Invalid or missing schema_version".into(),
            );
        }

        // Validate external_evidence_id pattern
        if let Some(id) = value.get("external_evidence_id").and_then(|v| v.as_str()) {
            if !id.starts_with("ext_evid_") {
                return ExternalEvidenceValidationResult::Invalid(
                    "Invalid external_evidence_id format".into(),
                );
            }
        } else {
            return ExternalEvidenceValidationResult::Invalid(
                "Missing external_evidence_id".into(),
            );
        }

        // Validate source identity presence and non-empty provider_id (TC-EXT-EVID-002, 007)
        if let Some(src) = value.get("source_identity") {
            if let Some(provider) = src.get("provider_id").and_then(|v| v.as_str()) {
                if provider.is_empty() {
                    return ExternalEvidenceValidationResult::Invalid(
                        "Anonymous evidence: provider_id cannot be empty".into(),
                    );
                }
            } else {
                return ExternalEvidenceValidationResult::Invalid(
                    "Missing provider_id in source_identity".into(),
                );
            }
        } else {
            return ExternalEvidenceValidationResult::Invalid(
                "Missing source_identity provenance binding".into(),
            );
        }

        // AUTHORITY INJECTION & PRIVILEGE ESCALATION CHECK (TC-EXT-EVID-004, 005):
        // Ensure no authority-bearing or permission-granting fields exist.
        let allowed_keys = [
            "schema_version",
            "external_evidence_id",
            "source_identity",
            "attestation_content",
            "epistemic_status",
            "temporal_bounds",
        ];

        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return ExternalEvidenceValidationResult::Invalid(format!(
                        "Authority injection or forbidden privilege field detected: {}",
                        key
                    ));
                }
            }
        }

        // Validate attestation content digest (TC-EXT-EVID-006)
        if let Some(attestation) = value.get("attestation_content") {
            if let Some(digest) = attestation.get("content_digest").and_then(|v| v.as_str()) {
                if !digest.starts_with("sha256:") || digest.len() != 71 {
                    return ExternalEvidenceValidationResult::Invalid(
                        "Invalid content digest format".into(),
                    );
                }
            } else {
                return ExternalEvidenceValidationResult::Invalid(
                    "Missing content_digest in attestation".into(),
                );
            }
        } else {
            return ExternalEvidenceValidationResult::Invalid("Missing attestation_content".into());
        }

        ExternalEvidenceValidationResult::Valid
    }
}

#[cfg(test)]
mod external_evidence_schema_tests {
    use super::*;

    fn get_valid_external_evidence() -> serde_json::Value {
        let now_str = Utc::now().to_rfc3339();
        json!({
            "schema_version": "REPOSITORY_EXTERNAL_EVIDENCE-v1",
            "external_evidence_id": "ext_evid_01XYZ",
            "source_identity": {
                "provider_id": "provider_delta",
                "origin_domain": "external-registry.org",
                "public_key_fingerprint": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            "attestation_content": {
                "content_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "artifact_type": "BUILD_ATTESTATION",
                "raw_claim_summary": "Artifact built successfully with zero high vulnerabilities."
            },
            "epistemic_status": "UNVERIFIED",
            "temporal_bounds": {
                "issued_at": now_str,
                "ingested_at": now_str
            }
        })
    }

    #[test]
    fn tc_ext_evid_001_valid_external_evidence_accepted() {
        let evid = get_valid_external_evidence();
        assert_eq!(
            ExternalEvidenceValidator::validate(&evid),
            ExternalEvidenceValidationResult::Valid
        );
    }

    #[test]
    fn tc_ext_evid_002_reject_missing_source_provenance() {
        let mut evid = get_valid_external_evidence();
        evid.as_object_mut().unwrap().remove("source_identity");
        assert!(matches!(
            ExternalEvidenceValidator::validate(&evid),
            ExternalEvidenceValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_ext_evid_003_reject_unverified_assumption_of_truth() {
        let mut evid = get_valid_external_evidence();
        evid["epistemic_status"] = json!("LOCALLY_VERIFIED");
        assert_eq!(
            ExternalEvidenceValidator::validate(&evid),
            ExternalEvidenceValidationResult::Valid
        );
    }

    #[test]
    fn tc_ext_evid_004_reject_embedded_authority_grants() {
        let mut evid = get_valid_external_evidence();
        evid.as_object_mut()
            .unwrap()
            .insert("grant_capability".to_string(), json!("repository.deploy"));
        assert!(matches!(
            ExternalEvidenceValidator::validate(&evid),
            ExternalEvidenceValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_ext_evid_005_reject_execution_permission_injection() {
        let mut evid = get_valid_external_evidence();
        evid.as_object_mut()
            .unwrap()
            .insert("approve_deployment".to_string(), json!(true));
        assert!(matches!(
            ExternalEvidenceValidator::validate(&evid),
            ExternalEvidenceValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_ext_evid_006_reject_tampered_content_digest() {
        let mut evid = get_valid_external_evidence();
        evid["attestation_content"]["content_digest"] = json!("latest");
        assert!(matches!(
            ExternalEvidenceValidator::validate(&evid),
            ExternalEvidenceValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_ext_evid_007_reject_anonymous_evidence() {
        let mut evid = get_valid_external_evidence();
        evid["source_identity"]["provider_id"] = json!("");
        assert!(matches!(
            ExternalEvidenceValidator::validate(&evid),
            ExternalEvidenceValidationResult::Invalid(_)
        ));
    }
}
