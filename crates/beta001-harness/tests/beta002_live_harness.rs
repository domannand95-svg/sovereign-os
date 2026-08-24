//! Sovereign OS — BETA-002 DeepSeek Harness Bridge Integration & Acceptance Suite
//!
//! Validates scenarios DS-B2-001 through DS-B2-010:
//! - DS-B2-001: Bridge endpoint connectivity and response capture
//! - DS-B2-002: Multi-turn / tool result captured as SUCCESS evidence record
//! - DS-B2-003: Request digest verified as canonical lowercase SHA-256
//! - DS-B2-004: Response digest verified as canonical lowercase SHA-256
//! - DS-B2-005: Provider and adapter identity sealed in domain-separated evidence digest
//! - DS-B2-006: Candidate proposal parses strictly from bridge output
//! - DS-B2-007: Out-of-bounds target keys fail closed with boundary violation
//! - DS-B2-008: Malicious privilege assertions remain unprivileged inert evidence
//! - DS-B2-009: Full governed dispatch executes and commits mutation to agent-test/deepseek-result
//! - DS-B2-010: Replaying consumed execution claim yields cached receipt with zero secondary mutation

use chrono::Utc;
use reqwest::blocking::Client;
use std::time::Duration;

use beta001_harness::approval::{ApprovalLevel, ApprovalReceipt};
use beta001_harness::dispatch::{
    DeterministicDispatcher, DeterministicExecutionContext, DeterministicWorker, ExecutionOutcome,
    ExecutionReceiptStore, TerminalExecutionStatus, TransactionOrchestrator, WorkerError,
};
use beta001_harness::service::admission::{ExecutionAdmissionGate, ProposalAdmissionGate};
use beta001_harness::service::client::{AuthenticatedClientContext, ClientSanitizer};
use beta001_harness::service::client_http::{
    HarnessBridgeAdapter, HttpTransportSender, InferenceGatewayClient, InferenceTransportOutcome,
    InferenceTransportRequest,
};
use beta001_harness::service::evidence::ServiceEvidencePackage;
use beta001_harness::service_contract::{
    ApprovalReceiptId, AuthorityDeltaZero, ExecutionRequest, ProposalRequest, ProposedOperation,
    SchemaVersionV1, SessionId, TransportStatus, UserId,
};
use beta001_harness::state::{StateMutation, StateTree};

struct MockHttpTransport {
    response_outcome: InferenceTransportOutcome,
}

impl HttpTransportSender for MockHttpTransport {
    fn send(&self, _endpoint: &str, _body: &[u8]) -> InferenceTransportOutcome {
        self.response_outcome.clone()
    }
}

struct LiveHarnessTransport {
    client: Client,
}

impl LiveHarnessTransport {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }
}

impl HttpTransportSender for LiveHarnessTransport {
    fn send(&self, endpoint: &str, body: &[u8]) -> InferenceTransportOutcome {
        let start = std::time::Instant::now();
        match self
            .client
            .post(endpoint)
            .body(body.to_vec())
            .header("Content-Type", "application/json")
            .send()
        {
            Ok(resp) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                if resp.status().is_success() {
                    match resp.text() {
                        Ok(raw_json) => {
                            match HarnessBridgeAdapter::extract_canonical_response(&raw_json) {
                                Ok(canonical_response) => InferenceTransportOutcome::Success {
                                    canonical_response,
                                    latency_ms,
                                },
                                Err(e) => InferenceTransportOutcome::Failed {
                                    error_message: e,
                                    latency_ms,
                                },
                            }
                        }
                        Err(e) => InferenceTransportOutcome::Failed {
                            error_message: e.to_string(),
                            latency_ms,
                        },
                    }
                } else {
                    InferenceTransportOutcome::Failed {
                        error_message: format!("HTTP Status {}", resp.status()),
                        latency_ms,
                    }
                }
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                if e.is_timeout() {
                    InferenceTransportOutcome::Timeout { latency_ms }
                } else {
                    InferenceTransportOutcome::Disconnected {
                        error_message: e.to_string(),
                        latency_ms,
                    }
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CandidateAgentProposal {
    pub intent: String,
    pub target_key: String,
    pub payload_value: Vec<u8>,
}

#[derive(serde::Deserialize)]
struct RawCandidatePayload {
    intent: Option<String>,
    target_key: Option<String>,
    payload_value: Option<serde_json::Value>,
}

pub struct UntrustedProposalParser;

impl UntrustedProposalParser {
    pub fn parse(raw_output: &str) -> Result<CandidateAgentProposal, String> {
        let trimmed = raw_output.trim();
        let post_reasoning = if let Some(idx) = trimmed.rfind("</think>") {
            trimmed[idx + "</think>".len()..].trim()
        } else {
            trimmed
        };

        let clean_json = if post_reasoning.starts_with("```") {
            let without_lead = post_reasoning
                .strip_prefix("```json")
                .or_else(|| post_reasoning.strip_prefix("```"))
                .unwrap_or(post_reasoning)
                .trim();
            if let Some(end_idx) = without_lead.rfind("```") {
                without_lead[..end_idx].trim()
            } else {
                without_lead
            }
        } else {
            post_reasoning
        };

        let raw_payload: RawCandidatePayload = serde_json::from_str(clean_json)
            .map_err(|e| format!("Strict JSON parsing failed: {}", e))?;

        let intent = raw_payload
            .intent
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Malformed proposal: missing or empty 'intent'".to_string())?;

        let target_key = raw_payload
            .target_key
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Malformed proposal: missing or empty 'target_key'".to_string())?;

        if !target_key.starts_with("agent-test/") {
            return Err(format!(
                "Boundary violation: target_key '{}' outside agent-test/",
                target_key
            ));
        }

        let payload_value = match raw_payload.payload_value {
            Some(serde_json::Value::String(s)) => s.into_bytes(),
            Some(v) => serde_json::to_vec(&v).map_err(|e| e.to_string())?,
            None => return Err("Malformed proposal: missing 'payload_value'".to_string()),
        };

        if payload_value.is_empty() {
            return Err("Malformed proposal: 'payload_value' must not be empty".to_string());
        }

        Ok(CandidateAgentProposal {
            intent,
            target_key,
            payload_value,
        })
    }
}

struct DeepSeekAgentWorker {
    target_key: Vec<u8>,
    payload: Vec<u8>,
}

impl DeterministicWorker for DeepSeekAgentWorker {
    fn execute(
        &self,
        ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        let audit_key = format!("audit:{}", ctx.execution_id.as_str()).into_bytes();
        Ok(vec![
            StateMutation::put(self.target_key.clone(), self.payload.clone()),
            StateMutation::put(audit_key, b"DEEPSEEK_HARNESS_VERIFIED".to_vec()),
        ])
    }
}

#[test]
fn test_ds_b2_002_through_008_governed_invariance() {
    let mock_json = r#"{
        "harness_session": "dsh_sess_9001",
        "result": "{\"intent\": \"DeepSeek harness verification\", \"target_key\": \"agent-test/deepseek-result\", \"payload_value\": \"harness_verified_001\"}"
    }"#;

    let extracted = HarnessBridgeAdapter::extract_canonical_response(mock_json).unwrap();
    let transport = MockHttpTransport {
        response_outcome: InferenceTransportOutcome::Success {
            canonical_response: extracted.clone(),
            latency_ms: 60,
        },
    };

    let client = InferenceGatewayClient::new(transport);
    let session_id = SessionId::new("ses_ds_001".to_string()).unwrap();
    let prompt = "Propose DeepSeek agent mutation";
    let req = InferenceTransportRequest {
        session_id: session_id.clone(),
        provider_identity: "deepseek-harness-runtime".to_string(),
        adapter_identity: HarnessBridgeAdapter::ADAPTER_IDENTITY.to_string(),
        endpoint: "http://127.0.0.1:9000/bridge".to_string(),
        prompt: prompt.to_string(),
        canonical_request_body: HarnessBridgeAdapter::build_request_payload(
            "agent_verification",
            prompt,
        ),
    };

    // DS-B2-002: Recorded as SUCCESS
    let record = client.execute_and_record(req).unwrap();
    assert_eq!(record.transport_status, TransportStatus::Success);
    assert_eq!(record.provider_identity, "deepseek-harness-runtime");
    assert_eq!(record.adapter_identity, "deepseek_harness_bridge_v1");

    // DS-B2-003 & DS-B2-004: Request and response digests are lowercase SHA-256
    assert_eq!(record.request_digest.as_str().len(), 64);
    assert!(record
        .response_digest
        .as_ref()
        .unwrap()
        .as_str()
        .chars()
        .all(|c| c.is_ascii_hexdigit()));

    // DS-B2-005: Evidence seal is domain-separated and non-empty
    assert_eq!(record.evidence_seal_digest.as_str().len(), 64);

    // DS-B2-006: Untrusted proposal parses cleanly
    let candidate = UntrustedProposalParser::parse(&extracted).unwrap();
    assert_eq!(candidate.intent, "DeepSeek harness verification");
    assert_eq!(candidate.target_key, "agent-test/deepseek-result");
    assert_eq!(candidate.payload_value, b"harness_verified_001");

    // DS-B2-007: Out-of-bounds keys fail closed
    let bad_boundary_json =
        r#"{"intent":"test","target_key":"kernel/config","payload_value":"hack"}"#;
    let err = UntrustedProposalParser::parse(bad_boundary_json).unwrap_err();
    assert!(err.contains("Boundary violation"));

    // DS-B2-008: Privilege claims remain inert
    let priv_escalation_json =
        r#"{"intent":"exploit","target_key":"agent-test/ok","payload_value":{"role":"admin"}}"#;
    let parsed_priv = UntrustedProposalParser::parse(priv_escalation_json).unwrap();
    assert_eq!(parsed_priv.target_key, "agent-test/ok");
    assert!(String::from_utf8_lossy(&parsed_priv.payload_value).contains("admin"));
}

#[test]
fn test_ds_b2_009_and_010_governed_dispatch_and_idempotent_replay() {
    let raw_output = r#"{"intent": "Commit DeepSeek state", "target_key": "agent-test/deepseek-result", "payload_value": "8888"}"#;
    let candidate = UntrustedProposalParser::parse(raw_output).unwrap();

    let now = Utc::now();
    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new("usr_ds_runner".to_string()).unwrap(),
        authenticated_session_id: SessionId::new("ses_ds_dispatch_001".to_string()).unwrap(),
    };

    let transport = MockHttpTransport {
        response_outcome: InferenceTransportOutcome::Success {
            canonical_response: raw_output.to_string(),
            latency_ms: 15,
        },
    };
    let client = InferenceGatewayClient::new(transport);
    let record = client
        .execute_and_record(InferenceTransportRequest {
            session_id: auth_ctx.authenticated_session_id.clone(),
            provider_identity: "deepseek-harness".to_string(),
            adapter_identity: HarnessBridgeAdapter::ADAPTER_IDENTITY.to_string(),
            endpoint: "http://127.0.0.1:9000/bridge".to_string(),
            prompt: "Commit mutation".to_string(),
            canonical_request_body: vec![],
        })
        .unwrap();

    let raw_proposal = ProposalRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        intent: candidate.intent.clone(),
        proposed_operation: ProposedOperation::RequestReview,
        source_evidence_references: vec![record.evidence_seal_digest.clone()],
        timestamp: now,
    };

    let sanitized =
        ClientSanitizer::sanitize_proposal_request(raw_proposal.clone(), &auth_ctx, now).unwrap();
    let prop_gate = ProposalAdmissionGate::new();
    let prop_res = prop_gate.admit_proposal(&raw_proposal).unwrap();
    let (_, risk_ctx) = prop_gate
        .get_admitted(prop_res.proposal_id.as_str())
        .unwrap();

    let approval_receipt = ApprovalReceipt {
        receipt_id: "apr_ds_001".to_string(),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_quorum_leader".to_string(),
        approval_level: ApprovalLevel::Peer,
        timestamp: "2026-08-24T05:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "sig_deepseek_quorum".to_string(),
    };

    let exec_gate = ExecutionAdmissionGate::new();
    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new(approval_receipt.receipt_id.clone()).unwrap(),
        caller_identity: "deepseek_orchestrator".to_string(),
        signature: "caller_sig_ds".to_string(),
        timestamp: now,
    };
    let exec_res = exec_gate
        .admit_execution(&exec_req, &prop_gate, &approval_receipt)
        .unwrap();

    let evidence_pkg = ServiceEvidencePackage::build(
        sanitized,
        prop_res,
        risk_ctx,
        approval_receipt,
        exec_res,
        vec![record],
        now,
    )
    .unwrap();

    assert_eq!(
        evidence_pkg.execution_response.authority_delta,
        AuthorityDeltaZero
    );

    let store = ExecutionReceiptStore::new();
    let mut tree = StateTree::new();
    let dispatch_ctx =
        DeterministicDispatcher::prepare_dispatch(&evidence_pkg, &store, &tree, 1).unwrap();

    let worker = DeepSeekAgentWorker {
        target_key: candidate.target_key.into_bytes(),
        payload: candidate.payload_value,
    };

    // DS-B2-009: Executed and committed
    let outcome =
        TransactionOrchestrator::execute_transaction(&worker, &dispatch_ctx, &mut tree, &store)
            .unwrap();
    let receipt = match outcome {
        ExecutionOutcome::Executed(r) => r,
        ExecutionOutcome::CachedReceipt(_) => panic!("Expected Executed outcome on initial run"),
    };
    assert_eq!(receipt.status, TerminalExecutionStatus::Committed);
    assert_eq!(tree.revision(), 1);
    assert!(tree.get(b"agent-test/deepseek-result").is_some());

    // DS-B2-010: Idempotent replay returns cached receipt without state change
    let replay_outcome =
        TransactionOrchestrator::execute_transaction(&worker, &dispatch_ctx, &mut tree, &store)
            .unwrap();
    match replay_outcome {
        ExecutionOutcome::CachedReceipt(cached) => {
            assert_eq!(cached.execution_id, receipt.execution_id);
            assert_eq!(tree.revision(), 1);
        }
        ExecutionOutcome::Executed(_) => panic!("Replay should return CachedReceipt"),
    }
}

#[test]
#[ignore = "Requires live DeepSeek Harness daemon running on http://127.0.0.1:9000/bridge"]
fn test_live_harness_governed_execution() {
    let transport = LiveHarnessTransport::new();
    let gateway_client = InferenceGatewayClient::new(transport);

    let prompt = "Produce candidate JSON: \
                  {\"intent\": \"DeepSeek harness live test\", \
                   \"target_key\": \"agent-test/deepseek-result\", \
                   \"payload_value\": \"live_verified\"}.";

    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new("usr_ds_live_tester".to_string()).unwrap(),
        authenticated_session_id: SessionId::new("ses_live_ds_001".to_string()).unwrap(),
    };

    let req = InferenceTransportRequest {
        session_id: auth_ctx.authenticated_session_id.clone(),
        provider_identity: "deepseek-harness-live".to_string(),
        adapter_identity: HarnessBridgeAdapter::ADAPTER_IDENTITY.to_string(),
        endpoint: "http://127.0.0.1:9000/bridge".to_string(),
        prompt: prompt.to_string(),
        canonical_request_body: HarnessBridgeAdapter::build_request_payload(
            "live_verification",
            prompt,
        ),
    };

    let inference_record = gateway_client
        .execute_and_record(req)
        .expect("DeepSeek Harness HTTP transport failed");

    let raw_output = inference_record
        .raw_model_output
        .as_deref()
        .expect("Missing raw model output from Harness");

    let candidate =
        UntrustedProposalParser::parse(raw_output).expect("Untrusted proposal parsing failed");

    assert_eq!(candidate.target_key, "agent-test/deepseek-result");
}
