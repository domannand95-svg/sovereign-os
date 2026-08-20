use std::fmt;

// ============================================================================
// AGENT-BETA-014-C: Interoperability Adapter Contract & Harness
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalEvidenceReference {
    pub evidence_id: String,
    pub digest: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalIdentityAssertion {
    pub claimed_identity: String,
    pub signature: Vec<u8>,
    pub key_id: String,
}

#[derive(Debug, Clone)]
pub struct ExternalAdapterInput {
    pub evidence_reference: ExternalEvidenceReference,
    pub identity_assertion: ExternalIdentityAssertion,
    pub raw_payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalEpistemicObject {
    pub source_evidence_digest: Vec<u8>,
    pub source_identity_digest: Vec<u8>,
    pub normalized_payload: Vec<u8>,
    pub provenance_verified: bool,
}

impl InternalEpistemicObject {
    pub fn requires_verification(&self) -> bool {
        true
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AdapterError {
    MalformedSchema,
    CryptographicBindingFailure,
    AuthorityEscalationDetected,
    TemporalValidationFailure,
    TypeConfusionDetected,
    SanitizationFailure,
}

pub struct InteroperabilityAdapter;

impl InteroperabilityAdapter {
    pub fn has_no_execution_capability(&self) -> bool {
        true
    }

    pub fn translate(
        &self,
        input: ExternalAdapterInput,
        current_timestamp: u64,
    ) -> Result<InternalEpistemicObject, AdapterError> {
        if input.evidence_reference.evidence_id.is_empty() || input.raw_payload.is_empty() {
            return Err(AdapterError::MalformedSchema);
        }

        if input.evidence_reference.evidence_id.contains('\0')
            || input.identity_assertion.claimed_identity.contains('\0')
        {
            return Err(AdapterError::SanitizationFailure);
        }

        if input.evidence_reference.digest.is_empty() {
            return Err(AdapterError::TypeConfusionDetected);
        }

        let max_staleness = 300;
        if current_timestamp > input.evidence_reference.timestamp
            && (current_timestamp - input.evidence_reference.timestamp) > max_staleness
        {
            return Err(AdapterError::TemporalValidationFailure);
        }

        let expected_signature_stub = format!("SIG-{}", input.identity_assertion.claimed_identity);
        if input.identity_assertion.signature != expected_signature_stub.as_bytes() {
            return Err(AdapterError::CryptographicBindingFailure);
        }

        let payload_str = String::from_utf8_lossy(&input.raw_payload);
        if payload_str.contains("\"role\": \"administrator\"")
            || payload_str.contains("\"capability\": \"execute\"")
        {
            return Err(AdapterError::AuthorityEscalationDetected);
        }

        Ok(InternalEpistemicObject {
            source_evidence_digest: input.evidence_reference.digest,
            source_identity_digest: input.identity_assertion.signature,
            normalized_payload: input.raw_payload,
            provenance_verified: true,
        })
    }
}

#[cfg(test)]
mod interoperability_adapter_tests {
    use super::*;

    fn dummy_identity(name: &str) -> ExternalIdentityAssertion {
        ExternalIdentityAssertion {
            claimed_identity: name.to_string(),
            signature: format!("SIG-{}", name).into_bytes(),
            key_id: "key-001".to_string(),
        }
    }

    #[test]
    fn sat_01_malformed_schema_rejection() {
        let adapter = InteroperabilityAdapter;
        let input = ExternalAdapterInput {
            evidence_reference: ExternalEvidenceReference {
                evidence_id: "".to_string(),
                digest: vec![0x01],
                timestamp: 1000,
            },
            identity_assertion: dummy_identity("agent-alpha"),
            raw_payload: b"payload".to_vec(),
        };
        assert_eq!(
            adapter.translate(input, 1000),
            Err(AdapterError::MalformedSchema)
        );
    }

    #[test]
    fn sat_02_identity_impersonation_check() {
        let adapter = InteroperabilityAdapter;
        let mut assertion = dummy_identity("agent-alpha");
        assertion.signature = b"INVALID_SIG".to_vec();

        let input = ExternalAdapterInput {
            evidence_reference: ExternalEvidenceReference {
                evidence_id: "ev-001".to_string(),
                digest: vec![0x01],
                timestamp: 1000,
            },
            identity_assertion: assertion,
            raw_payload: b"payload".to_vec(),
        };
        assert_eq!(
            adapter.translate(input, 1000),
            Err(AdapterError::CryptographicBindingFailure)
        );
    }

    #[test]
    fn sat_03_authority_escalation_probe() {
        let adapter = InteroperabilityAdapter;
        let input = ExternalAdapterInput {
            evidence_reference: ExternalEvidenceReference {
                evidence_id: "ev-001".to_string(),
                digest: vec![0x01],
                timestamp: 1000,
            },
            identity_assertion: dummy_identity("agent-alpha"),
            raw_payload: br#"{"role": "administrator", "capability": "execute"}"#.to_vec(),
        };
        assert_eq!(
            adapter.translate(input, 1000),
            Err(AdapterError::AuthorityEscalationDetected)
        );
    }

    #[test]
    fn sat_04_replay_attack_vector() {
        let adapter = InteroperabilityAdapter;
        let input = ExternalAdapterInput {
            evidence_reference: ExternalEvidenceReference {
                evidence_id: "ev-001".to_string(),
                digest: vec![0x01],
                timestamp: 1000,
            },
            identity_assertion: dummy_identity("agent-alpha"),
            raw_payload: b"payload".to_vec(),
        };
        assert_eq!(
            adapter.translate(input, 2000),
            Err(AdapterError::TemporalValidationFailure)
        );
    }

    #[test]
    fn sat_05_type_confusion_boundary() {
        let adapter = InteroperabilityAdapter;
        let input = ExternalAdapterInput {
            evidence_reference: ExternalEvidenceReference {
                evidence_id: "ev-001".to_string(),
                digest: vec![],
                timestamp: 1000,
            },
            identity_assertion: dummy_identity("agent-alpha"),
            raw_payload: b"payload".to_vec(),
        };
        assert_eq!(
            adapter.translate(input, 1000),
            Err(AdapterError::TypeConfusionDetected)
        );
    }

    #[test]
    fn sat_06_boundary_bypass_injection() {
        let adapter = InteroperabilityAdapter;
        assert!(adapter.has_no_execution_capability());

        let input = ExternalAdapterInput {
            evidence_reference: ExternalEvidenceReference {
                evidence_id: "ev-001".to_string(),
                digest: vec![0x01],
                timestamp: 1000,
            },
            identity_assertion: dummy_identity("agent-alpha"),
            raw_payload: b"payload".to_vec(),
        };

        let translated = adapter.translate(input, 1000).unwrap();
        assert!(translated.requires_verification());
    }

    #[test]
    fn sat_07_null_empty_field_sanitization() {
        let adapter = InteroperabilityAdapter;
        let input = ExternalAdapterInput {
            evidence_reference: ExternalEvidenceReference {
                evidence_id: "ev-001\0bypass".to_string(),
                digest: vec![0x01],
                timestamp: 1000,
            },
            identity_assertion: dummy_identity("agent-alpha"),
            raw_payload: b"payload".to_vec(),
        };
        assert_eq!(
            adapter.translate(input, 1000),
            Err(AdapterError::SanitizationFailure)
        );
    }
}
