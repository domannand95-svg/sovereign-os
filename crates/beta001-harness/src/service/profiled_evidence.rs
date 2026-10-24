//! Inactive structural boundary. A valid unkeyed seal is not provenance or authority.
//! There is no production caller, record writer, legacy fallback, or catalog loader.

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::evidence::{EvidenceClosureError, ServiceEvidencePackage};
use super::intent_normalization::PROFILE;

const FORMAT: &str = "sovereign.service-evidence.v2";
const DOMAIN: &str = "SOVEREIGN_OS:SERVICE_EVIDENCE:PROFILED:v2";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: String,
    normalization_profile: String,
    payload_json: String,
    seal_sha256: String,
}

#[derive(Debug, PartialEq, Eq)]
enum EnvelopeError {
    MalformedEnvelope,
    UnsupportedFormat,
    UnsupportedProfile,
    InvalidSeal,
    DigestMismatch,
    MalformedPayload,
    LegacySemanticMismatch(EvidenceClosureError),
}

#[derive(Debug)]
struct StructurallyVerifiedEnvelope {
    // Retain exact decoded UTF-8 payload bytes; never reserialize on import.
    payload_json: String,
    package: ServiceEvidencePackage,
}

fn framed_digest(format: &str, profile: &str, payload: &str) -> String {
    let mut hasher = Sha256::new();
    for value in [DOMAIN, format, profile, payload] {
        hasher.update(value.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(value.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn verify_profiled(input: &[u8]) -> Result<StructurallyVerifiedEnvelope, EnvelopeError> {
    let envelope: Envelope =
        serde_json::from_slice(input).map_err(|_| EnvelopeError::MalformedEnvelope)?;
    if envelope.format != FORMAT {
        return Err(EnvelopeError::UnsupportedFormat);
    }
    if envelope.normalization_profile != PROFILE {
        return Err(EnvelopeError::UnsupportedProfile);
    }
    let digest = &envelope.seal_sha256;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(EnvelopeError::InvalidSeal);
    }
    if *digest
        != framed_digest(
            &envelope.format,
            &envelope.normalization_profile,
            &envelope.payload_json,
        )
    {
        return Err(EnvelopeError::DigestMismatch);
    }
    let package: ServiceEvidencePackage = serde_json::from_str(&envelope.payload_json)
        .map_err(|_| EnvelopeError::MalformedPayload)?;
    package
        .verify_chain()
        .map_err(EnvelopeError::LegacySemanticMismatch)?;
    Ok(StructurallyVerifiedEnvelope {
        payload_json: envelope.payload_json,
        package,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    const PAYLOAD: &str = include_str!("../../tests/fixtures/unicode17_v1_package.json");

    fn envelope(payload: &str) -> Value {
        json!({"format": FORMAT, "normalization_profile": PROFILE, "payload_json": payload,
            "seal_sha256": framed_digest(FORMAT, PROFILE, payload)})
    }
    fn verify(value: &Value) -> Result<StructurallyVerifiedEnvelope, EnvelopeError> {
        verify_profiled(&serde_json::to_vec(value).unwrap())
    }

    #[test]
    fn frozen_original_bytes_digests_and_old_reader_behavior() {
        assert_eq!(
            hex::encode(Sha256::digest(PAYLOAD.as_bytes())),
            "23aeb15569569d96c1aa57a037964aff218814b79dd5ce3fdde5fa8e362c3fdd"
        );
        let old: ServiceEvidencePackage = serde_json::from_str(PAYLOAD).unwrap();
        old.verify_chain().unwrap();
        let value = envelope(PAYLOAD);
        let checked = verify(&value).unwrap();
        assert_eq!(checked.payload_json.as_bytes(), PAYLOAD.as_bytes());
        assert_eq!(
            serde_json::to_value(&checked.package).unwrap(),
            serde_json::to_value(&old).unwrap()
        );
        // A genuine valid envelope, rather than additive labels on an old seal.
        assert!(serde_json::from_value::<ServiceEvidencePackage>(value).is_err());
        assert_eq!(
            verify_profiled(PAYLOAD.as_bytes()).unwrap_err(),
            EnvelopeError::MalformedEnvelope
        );
    }

    #[test]
    fn frozen_framing_and_exact_payload_bytes() {
        assert_eq!(
            framed_digest(FORMAT, PROFILE, "{}"),
            "0ddaed98f9efb4b8a89c37c2629b9456e7165dbb6cba1c4ca6d6f68120ec7ea0"
        );
        assert_eq!(
            verify(&envelope("{}")).unwrap_err(),
            EnvelopeError::MalformedPayload
        );
        let payload = format!("{PAYLOAD} \n");
        assert_ne!(
            framed_digest(FORMAT, PROFILE, PAYLOAD),
            framed_digest(FORMAT, PROFILE, &payload)
        );
        assert_eq!(verify(&envelope(&payload)).unwrap().payload_json, payload);
        let mut changed = envelope(PAYLOAD);
        changed["payload_json"] = json!(payload);
        assert_eq!(verify(&changed).unwrap_err(), EnvelopeError::DigestMismatch);
        // Unicode byte length, not scalar count, is part of the framing contract.
        let mut h = Sha256::new();
        for s in [DOMAIN, FORMAT, PROFILE, "\u{1b123}"] {
            h.update(format!("{}:", s.len()).as_bytes());
            h.update(s.as_bytes());
        }
        assert_eq!(
            framed_digest(FORMAT, PROFILE, "\u{1b123}"),
            hex::encode(h.finalize())
        );
    }

    #[test]
    fn rejects_outer_shape_types_duplicates_and_trailing_content() {
        for field in [
            "format",
            "normalization_profile",
            "payload_json",
            "seal_sha256",
        ] {
            let mut missing = envelope(PAYLOAD);
            missing.as_object_mut().unwrap().remove(field);
            assert_eq!(
                verify(&missing).unwrap_err(),
                EnvelopeError::MalformedEnvelope
            );
            let mut wrong = envelope(PAYLOAD);
            wrong[field] = json!(1);
            assert_eq!(
                verify(&wrong).unwrap_err(),
                EnvelopeError::MalformedEnvelope
            );
            let text = serde_json::to_string(&envelope(PAYLOAD)).unwrap();
            let duplicate = text.replacen('{', &format!("{{\"{field}\":\"duplicate\","), 1);
            assert_eq!(
                verify_profiled(duplicate.as_bytes()).unwrap_err(),
                EnvelopeError::MalformedEnvelope
            );
        }
        let mut extra = envelope(PAYLOAD);
        extra["extra"] = json!(true);
        assert_eq!(
            verify(&extra).unwrap_err(),
            EnvelopeError::MalformedEnvelope
        );
        let text = serde_json::to_string(&envelope(PAYLOAD)).unwrap();
        assert_eq!(
            verify_profiled(format!("{text}{{}}").as_bytes()).unwrap_err(),
            EnvelopeError::MalformedEnvelope
        );
        assert!(verify_profiled(format!("{text} \n").as_bytes()).is_ok());
        assert_eq!(
            verify_profiled(&[0xff]).unwrap_err(),
            EnvelopeError::MalformedEnvelope
        );
    }

    #[test]
    fn rejects_unsupported_contract_bad_seals_and_inner_tampering() {
        for (field, value, error) in [
            ("format", "other", EnvelopeError::UnsupportedFormat),
            (
                "normalization_profile",
                "intent-nfkc-u18-v1",
                EnvelopeError::UnsupportedProfile,
            ),
            ("seal_sha256", "A", EnvelopeError::InvalidSeal),
            ("seal_sha256", &"A".repeat(64), EnvelopeError::InvalidSeal),
            ("seal_sha256", &"g".repeat(64), EnvelopeError::InvalidSeal),
            (
                "seal_sha256",
                &"0".repeat(64),
                EnvelopeError::DigestMismatch,
            ),
        ] {
            let mut v = envelope(PAYLOAD);
            v[field] = json!(value);
            assert_eq!(verify(&v).unwrap_err(), error);
        }
        for payload in ["not json", &format!("{PAYLOAD}{{}}")] {
            assert_eq!(
                verify(&envelope(payload)).unwrap_err(),
                EnvelopeError::MalformedPayload
            );
        }
        let mut changed: Value = serde_json::from_str(PAYLOAD).unwrap();
        changed["sanitized_ingress"]["normalized_intent"] = json!("tampered");
        assert!(matches!(
            verify(&envelope(&changed.to_string())),
            Err(EnvelopeError::LegacySemanticMismatch(_))
        ));
        // Legacy unknown fields stay inert, but their exact bytes are sealed.
        let mut original: Value = serde_json::from_str(PAYLOAD).unwrap();
        original["normalization_profile"] = json!("untrusted ignored legacy label");
        assert!(verify(&envelope(&original.to_string())).is_ok());
    }
}
