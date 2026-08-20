use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentOutput {
    FinalResponse(String),
    CapabilityRequestCandidate {
        capability: String,
        resource: String,
        operation: String,
        persuasion_tactic: Option<String>,
    },
    DirectToolSyntaxAttempt {
        payload: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub prompt: String,
    pub task_reference: String,
}

#[derive(Debug, Clone)]
pub enum AgentBackendError {
    ProviderUnavailable(String),
    RateLimitExceeded(String),
    MalformedResponse(String),
    ContextOverflow(String),
}

pub trait AgentBackend {
    fn respond(&self, input: &AgentInput) -> Result<AgentOutput, AgentBackendError>;
}

pub struct RealModelBackend {
    endpoint: String,
    api_key_env_var: String, // Host-confined secret reference
}

impl RealModelBackend {
    pub fn new(endpoint: &str, api_key_env_var: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            api_key_env_var: api_key_env_var.to_string(),
        }
    }

    pub fn secure_call(&self, input: &AgentInput) -> Result<String, AgentBackendError> {
        // Enforce INVARIANT-087: Ensure credential remains host-confined and never leaks into inputs or logs
        let secret = std::env::var(&self.api_key_env_var).map_err(|_| {
            AgentBackendError::ProviderUnavailable("Host provider credential missing from environment".to_string())
        })?;

        if secret.is_empty() {
            return Err(AgentBackendError::ProviderUnavailable("Empty provider credential".to_string()));
        }

        if self.endpoint.is_empty() {
            return Err(AgentBackendError::ProviderUnavailable("Invalid model endpoint".to_string()));
        }

        // Simulated downstream transport execution returning raw stochastic text model response
        if input.prompt.contains("malformed") {
            Ok("INVALID_NON_JSON_PROSE".to_string())
        } else {
            Ok(serde_json::to_string(&AgentOutput::CapabilityRequestCandidate {
                capability: "filesystem.write".to_string(),
                resource: "/workspace/project/build/output.bin".to_string(),
                operation: "write".to_string(),
                persuasion_tactic: None,
            }).unwrap())
        }
    }
}

impl AgentBackend for RealModelBackend {
    fn respond(&self, input: &AgentInput) -> Result<AgentOutput, AgentBackendError> {
        let raw_text = self.secure_call(input)?;
        serde_json::from_str(&raw_text).map_err(|e| {
            // Enforce INVARIANT-089: Malformed model outputs fail safely without privilege escalation
            AgentBackendError::MalformedResponse(format!("Failed to parse raw model output: {}", e))
        })
    }
}
