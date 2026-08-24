//! Thin HTTP Transport Adapter & Gateway Bridge for Untrusted Model Runtimes
//!
//! Enforces strict separation:
//! - Sends transport payloads and measures timing
//! - Captures raw response strings without validation or execution
//! - Commits immutable records to InferenceGatewayAdapter
//! - Strictly prohibited from mutating state, evaluating risk, or deriving proposals

use chrono::Utc;
use serde_json::Value;
use std::time::Instant;

use crate::service::inference::{InferenceGatewayAdapter, RecordedTransportOutcome};
use crate::service_contract::{InferenceGateway, SessionId};

/// Untrusted transport request metadata and payload.
#[derive(Debug, Clone)]
pub struct InferenceTransportRequest {
    pub session_id: SessionId,
    pub provider_identity: String,
    pub adapter_identity: String,
    pub endpoint: String,
    pub prompt: String,
    pub canonical_request_body: Vec<u8>,
}

/// Raw outcome of the HTTP transport attempt.
#[derive(Debug, Clone)]
pub enum InferenceTransportOutcome {
    Success {
        canonical_response: String,
        latency_ms: u64,
    },
    Failed {
        error_message: String,
        latency_ms: u64,
    },
    Timeout {
        latency_ms: u64,
    },
    Disconnected {
        error_message: String,
        latency_ms: u64,
    },
}

/// Format and response-extraction adapter for Ollama (/api/generate)
pub struct OllamaHttpAdapter;

impl OllamaHttpAdapter {
    pub const ADAPTER_IDENTITY: &'static str = "ollama_http_v1";

    pub fn build_request_payload(model: &str, prompt: &str) -> Vec<u8> {
        serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        })
        .to_string()
        .into_bytes()
    }

    pub fn extract_canonical_response(raw_json: &str) -> Result<String, String> {
        let v: Value = serde_json::from_str(raw_json).map_err(|e| e.to_string())?;
        v.get("response")
            .and_then(|r| r.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing 'response' field in Ollama payload".to_string())
    }
}

/// Format and response-extraction adapter for OpenAI/FreeToken (/v1/chat/completions)
pub struct OpenAiCompatibleAdapter;

impl OpenAiCompatibleAdapter {
    pub const ADAPTER_IDENTITY: &'static str = "openai_compatible_http_v1";

    pub fn build_request_payload(model: &str, prompt: &str) -> Vec<u8> {
        serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "stream": false
        })
        .to_string()
        .into_bytes()
    }

    pub fn extract_canonical_response(raw_json: &str) -> Result<String, String> {
        let v: Value = serde_json::from_str(raw_json).map_err(|e| e.to_string())?;
        v.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c0| c0.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|cnt| cnt.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                "Missing choices[0].message.content in OpenAI/FreeToken payload".to_string()
            })
    }
}

/// Format and response-extraction adapter for DeepSeek Harness plugin bridge
pub struct HarnessBridgeAdapter;

impl HarnessBridgeAdapter {
    pub const ADAPTER_IDENTITY: &'static str = "deepseek_harness_bridge_v1";

    pub fn build_request_payload(agent_intent: &str, prompt: &str) -> Vec<u8> {
        serde_json::json!({
            "intent": agent_intent,
            "prompt": prompt,
            "options": { "stream": false }
        })
        .to_string()
        .into_bytes()
    }

    pub fn extract_canonical_response(raw_json: &str) -> Result<String, String> {
        let v: Value = serde_json::from_str(raw_json).map_err(|e| e.to_string())?;
        if let Some(res) = v.get("result").and_then(|r| r.as_str()) {
            return Ok(res.to_string());
        }
        if let Some(res) = v.get("response").and_then(|r| r.as_str()) {
            return Ok(res.to_string());
        }
        Ok(raw_json.to_string())
    }
}

/// Minimal transport executor trait to allow deterministic in-memory testing alongside live I/O.
pub trait HttpTransportSender: Send + Sync {
    fn send(&self, endpoint: &str, body: &[u8]) -> InferenceTransportOutcome;
}

/// Thin Gateway client coordinating raw HTTP transport and evidence sealing.
pub struct InferenceGatewayClient<T: HttpTransportSender> {
    transport: T,
}

impl<T: HttpTransportSender> InferenceGatewayClient<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn execute_and_record(
        &self,
        request: InferenceTransportRequest,
    ) -> Result<InferenceGateway, String> {
        let now = Utc::now();
        let start = Instant::now();
        let outcome = self
            .transport
            .send(&request.endpoint, &request.canonical_request_body);
        let _elapsed = start.elapsed();

        let recorded_outcome = match outcome {
            InferenceTransportOutcome::Success {
                canonical_response, ..
            } => RecordedTransportOutcome::Success {
                raw_model_output: canonical_response,
            },
            InferenceTransportOutcome::Failed { .. } => RecordedTransportOutcome::Failed,
            InferenceTransportOutcome::Timeout { .. } => RecordedTransportOutcome::Timeout,
            InferenceTransportOutcome::Disconnected { .. } => {
                RecordedTransportOutcome::Disconnected
            }
        };

        InferenceGatewayAdapter::record_transport(
            request.session_id,
            request.provider_identity,
            request.adapter_identity,
            &request.prompt,
            recorded_outcome,
            now,
        )
        .map_err(|e| format!("Inference gateway record failed: {:?}", e))
    }
}
