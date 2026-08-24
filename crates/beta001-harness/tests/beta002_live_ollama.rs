//! Sovereign OS — BETA-002 Live Ollama Governed Vertical Slice
//!
//! Executes a live end-to-end governed transaction against a local Ollama daemon:
//! 1. Dispatches prompt to Ollama at http://127.0.0.1:11434/api/generate
//! 2. Captures raw canonical response in InferenceGateway (\Delta Authority \equiv 0)
//! 3. Parses untrusted candidate proposal with fail-closed boundary validation
//! 4. Traverses ProposalAdmissionGate -> QuorumGate -> ExecutionAdmissionGate
//! 5. Dispatches to sandboxed worker and commits state mutation to agent-test/result

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
    HttpTransportSender, InferenceGatewayClient, InferenceTransportOutcome,
    InferenceTransportRequest, OllamaHttpAdapter,
};
use beta001_harness::service::evidence::ServiceEvidencePackage;
use beta001_harness::service_contract::{
    ApprovalReceiptId, AuthorityDeltaZero, ExecutionRequest, ProposalRequest, ProposedOperation,
    SchemaVersionV1, SessionId, UserId,
};
use beta001_harness::state::{StateMutation, StateTree};

struct LiveHttpTransport {
    client: Client,
}

impl LiveHttpTransport {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

impl HttpTransportSender for LiveHttpTransport {
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
                            match OllamaHttpAdapter::extract_canonical_response(&raw_json) {
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

struct LiveAgentWorker {
    target_key: Vec<u8>,
    payload: Vec<u8>,
}

impl DeterministicWorker for LiveAgentWorker {
    fn execute(
        &self,
        ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        let audit_key = format!("audit:{}", ctx.execution_id.as_str()).into_bytes();
        Ok(vec![
            StateMutation::put(self.target_key.clone(), self.payload.clone()),
            StateMutation::put(audit_key, b"LIVE_OLLAMA_VERIFIED".to_vec()),
        ])
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

#[test]
#[ignore = "Requires live Ollama daemon running on http://127.0.0.1:11434"]
fn test_live_ollama_governed_execution() {
    let transport = LiveHttpTransport::new();
    let gateway_client = InferenceGatewayClient::new(transport);

    let prompt = "You are a sandboxed agent. Produce a JSON object with: \
                  {\"intent\": \"Write calculation to state\", \
                   \"target_key\": \"agent-test/result\", \
                   \"payload_value\": \"42\"}. \
                  Only output valid JSON.";

    let now = Utc::now();
    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new("usr_live_tester".to_string()).unwrap(),
        authenticated_session_id: SessionId::new("ses_live_ollama_001".to_string()).unwrap(),
    };

    let req = InferenceTransportRequest {
        session_id: auth_ctx.authenticated_session_id.clone(),
        provider_identity: "ollama-local".to_string(),
        adapter_identity: OllamaHttpAdapter::ADAPTER_IDENTITY.to_string(),
        endpoint: "http://127.0.0.1:11434/api/generate".to_string(),
        prompt: prompt.to_string(),
        canonical_request_body: OllamaHttpAdapter::build_request_payload("llama3.2", prompt),
    };

    let inference_record = gateway_client
        .execute_and_record(req)
        .expect("Ollama HTTP transport failed");

    let raw_output = inference_record
        .raw_model_output
        .as_deref()
        .expect("Missing raw model output from Ollama");

    let candidate =
        UntrustedProposalParser::parse(raw_output).expect("Untrusted proposal parsing failed");

    assert_eq!(candidate.target_key, "agent-test/result");

    // Construct ProposalRequest anchored by the evidence seal digest
    let raw_proposal = ProposalRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        intent: candidate.intent.clone(),
        proposed_operation: ProposedOperation::RequestReview,
        source_evidence_references: vec![inference_record.evidence_seal_digest.clone()],
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
        receipt_id: "apr_live_ollama_001".to_string(),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_quorum_leader".to_string(),
        approval_level: ApprovalLevel::Peer,
        timestamp: "2026-08-24T03:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "sig_governance_verified".to_string(),
    };

    let exec_gate = ExecutionAdmissionGate::new();
    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new(approval_receipt.receipt_id.clone()).unwrap(),
        caller_identity: "orchestrator_daemon".to_string(),
        signature: "caller_sig_verified".to_string(),
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
        vec![inference_record],
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

    let worker = LiveAgentWorker {
        target_key: candidate.target_key.into_bytes(),
        payload: candidate.payload_value,
    };

    let outcome =
        TransactionOrchestrator::execute_transaction(&worker, &dispatch_ctx, &mut tree, &store)
            .unwrap();
    let receipt = match outcome {
        ExecutionOutcome::Executed(r) => r,
        ExecutionOutcome::CachedReceipt(_) => panic!("Expected Executed outcome"),
    };

    assert_eq!(receipt.status, TerminalExecutionStatus::Committed);
    assert_eq!(tree.revision(), 1);
    assert!(tree.get(b"agent-test/result").is_some());
}
