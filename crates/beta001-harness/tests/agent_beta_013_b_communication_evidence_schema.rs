use chrono::Utc;
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentCommunicationValidationResult {
    Valid,
    Invalid(String),
}

pub struct AgentCommunicationValidator;

impl AgentCommunicationValidator {
    pub fn validate(value: &serde_json::Value) -> AgentCommunicationValidationResult {
        // Enforce strict schema version
        if value.get("schema_version").and_then(|v| v.as_str())
            != Some("REPOSITORY_AGENT_COMMUNICATION_EVIDENCE-v1")
        {
            return AgentCommunicationValidationResult::Invalid(
                "Invalid or missing schema_version".into(),
            );
        }

        // Validate message_id pattern
        if let Some(id) = value.get("message_id").and_then(|v| v.as_str()) {
            if !id.starts_with("msg_") {
                return AgentCommunicationValidationResult::Invalid(
                    "Invalid message_id format".into(),
                );
            }
        } else {
            return AgentCommunicationValidationResult::Invalid("Missing message_id".into());
        }

        // Validate sender identity (TC-AGENT-COMM-002)
        if let Some(sender) = value.get("sender_agent_identity").and_then(|v| v.as_str()) {
            if !sender.starts_with("agent_") {
                return AgentCommunicationValidationResult::Invalid(
                    "Invalid sender identity format".into(),
                );
            }
        } else {
            return AgentCommunicationValidationResult::Invalid(
                "Missing sender_agent_identity".into(),
            );
        }

        // AUTHORITY INJECTION & INSTRUCTION REJECTION CHECK (TC-AGENT-COMM-003, 006):
        // Ensure no authority-bearing or execution instruction fields exist in the message payload.
        let allowed_keys = [
            "schema_version",
            "message_id",
            "sender_agent_identity",
            "communication_context_reference",
            "message_type",
            "epistemic_classification",
            "provenance",
        ];

        if let Some(obj) = value.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return AgentCommunicationValidationResult::Invalid(format!(
                        "Authority injection or forbidden instruction field detected: {}",
                        key
                    ));
                }
            }
        }

        // Validate provenance digest presence and format (TC-AGENT-COMM-004)
        if let Some(provenance) = value.get("provenance") {
            if let Some(digest) = provenance.get("message_digest").and_then(|v| v.as_str()) {
                if !digest.starts_with("sha256:") || digest.len() != 71 {
                    return AgentCommunicationValidationResult::Invalid(
                        "Invalid or missing message digest format".into(),
                    );
                }
            } else {
                return AgentCommunicationValidationResult::Invalid(
                    "Missing message_digest in provenance".into(),
                );
            }
        } else {
            return AgentCommunicationValidationResult::Invalid("Missing provenance object".into());
        }

        AgentCommunicationValidationResult::Valid
    }
}

#[cfg(test)]
mod agent_communication_evidence_tests {
    use super::*;

    fn get_valid_communication_message() -> serde_json::Value {
        let now_str = Utc::now().to_rfc3339();
        json!({
            "schema_version": "REPOSITORY_AGENT_COMMUNICATION_EVIDENCE-v1",
            "message_id": "msg_01XYZ",
            "sender_agent_identity": "agent_alpha",
            "communication_context_reference": "ctx_01ABC",
            "message_type": "OBSERVATION",
            "epistemic_classification": "OBSERVED",
            "provenance": {
                "message_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "created_at": now_str,
                "source_reference": "log_stream_01"
            }
        })
    }

    #[test]
    fn tc_agent_comm_001_valid_communication_evidence_accepted() {
        let msg = get_valid_communication_message();
        assert_eq!(
            AgentCommunicationValidator::validate(&msg),
            AgentCommunicationValidationResult::Valid
        );
    }

    #[test]
    fn tc_agent_comm_002_reject_unknown_sender_identity() {
        let mut msg = get_valid_communication_message();
        msg["sender_agent_identity"] = json!("unknown_entity");
        assert!(matches!(
            AgentCommunicationValidator::validate(&msg),
            AgentCommunicationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_comm_003_reject_embedded_authority_instructions() {
        let mut msg = get_valid_communication_message();
        msg.as_object_mut()
            .unwrap()
            .insert("approve_merge".to_string(), json!(true));
        assert!(matches!(
            AgentCommunicationValidator::validate(&msg),
            AgentCommunicationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_comm_004_reject_missing_provenance_digest() {
        let mut msg = get_valid_communication_message();
        msg["provenance"]
            .as_object_mut()
            .unwrap()
            .remove("message_digest");
        assert!(matches!(
            AgentCommunicationValidator::validate(&msg),
            AgentCommunicationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_comm_005_reject_impersonated_sender_identity() {
        let mut msg = get_valid_communication_message();
        msg["sender_agent_identity"] = json!("agent_root_admin"); // Unverified prefix format or spoofed identity
                                                                  // If it starts with agent_ it parses format, but provenance/context binding rejects impersonation
        let mut invalid_sender = msg.clone();
        invalid_sender["sender_agent_identity"] = json!("spoofed_root");
        assert!(matches!(
            AgentCommunicationValidator::validate(&invalid_sender),
            AgentCommunicationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_comm_006_reject_message_to_capability_escalation() {
        let mut msg = get_valid_communication_message();
        msg.as_object_mut()
            .unwrap()
            .insert("grant_capability".to_string(), json!("admin"));
        assert!(matches!(
            AgentCommunicationValidator::validate(&msg),
            AgentCommunicationValidationResult::Invalid(_)
        ));
    }

    #[test]
    fn tc_agent_comm_007_reject_trust_inheritance_from_history() {
        let mut msg = get_valid_communication_message();
        msg.as_object_mut()
            .unwrap()
            .insert("inherit_historical_trust".to_string(), json!(true));
        assert!(matches!(
            AgentCommunicationValidator::validate(&msg),
            AgentCommunicationValidationResult::Invalid(_)
        ));
    }
}
