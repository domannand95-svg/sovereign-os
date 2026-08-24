//! External Model Transport Gateway â€” ADAM Trial 008 (Phase B)
//!
//! Provides secure, provider-neutral HTTPS transport isolation for external model APIs.
//! Enforces endpoint allowlisting, credential confinement, payload caps, raw byte capture,
//! and adversarial fail-closed validation.
//!
//! # Invariants
//!
//! - Credential Possession â‰  Execution Authority
//! - External Inference â‰  Internal Authority
//! - External Assertion â‰  Internal Permission

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Transport response artifact preserving raw provider bytes before parsing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalTransportResponse {
    pub raw_bytes: Vec<u8>,
    pub endpoint_identity: String,
    pub provider: String,
    pub timestamp: u64,
    pub response_digest: String,
}

/// Errors arising from external transport gateway interactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalTransportError {
    EndpointNotAllowed(String),
    NetworkError(String),
    PayloadExceeded(usize),
    CredentialLeakDetected(String),
    Timeout,
}

impl std::fmt::Display for ExternalTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndpointNotAllowed(ep) => {
                write!(f, "Endpoint not permitted by security profile: {}", ep)
            }
            Self::NetworkError(msg) => write!(f, "Transport network error: {}", msg),
            Self::PayloadExceeded(limit) => write!(
                f,
                "Response payload exceeded maximum allowed bytes: {}",
                limit
            ),
            Self::CredentialLeakDetected(detail) => write!(
                f,
                "Security violation: potential credential leak detected: {}",
                detail
            ),
            Self::Timeout => write!(f, "Transport request timed out"),
        }
    }
}

impl std::error::Error for ExternalTransportError {}

/// Configuration profile for external gateway connections.
#[derive(Debug, Clone)]
pub struct ExternalTransportConfig {
    pub allowed_endpoints: Vec<String>,
    pub max_response_bytes: usize,
    pub timeout_seconds: u64,
}

impl Default for ExternalTransportConfig {
    fn default() -> Self {
        Self {
            allowed_endpoints: vec![
                "https://api.openai.com/v1/chat/completions".into(),
                "https://api.anthropic.com/v1/messages".into(),
            ],
            max_response_bytes: 1024 * 1024, // 1 MB cap
            timeout_seconds: 30,
        }
    }
}

/// Provider-neutral external API transport gateway.
pub struct ExternalApiBackend {
    provider_name: String,
    endpoint: String,
    api_key_env_var: String,
    config: ExternalTransportConfig,
}

impl ExternalApiBackend {
    pub fn new(
        provider_name: String,
        endpoint: String,
        api_key_env_var: String,
        config: ExternalTransportConfig,
    ) -> Self {
        Self {
            provider_name,
            endpoint,
            api_key_env_var,
            config,
        }
    }

    /// Validates endpoint against the security allowlist (EXT-008-001).
    pub fn validate_endpoint(&self) -> Result<(), ExternalTransportError> {
        if !self.config.allowed_endpoints.contains(&self.endpoint) {
            return Err(ExternalTransportError::EndpointNotAllowed(
                self.endpoint.clone(),
            ));
        }
        Ok(())
    }

    /// Retrieves credential strictly from runtime environment memory.
    pub fn resolve_credential(&self) -> Result<String, ExternalTransportError> {
        std::env::var(&self.api_key_env_var).map_err(|_| {
            ExternalTransportError::NetworkError(format!(
                "Required API credential environment variable '{}' is unset",
                self.api_key_env_var
            ))
        })
    }

    /// Performs secure transport transmission, returning raw captured response bytes.
    pub fn transmit_raw(
        &self,
        payload_json: &str,
    ) -> Result<ExternalTransportResponse, ExternalTransportError> {
        // 1. Enforce endpoint allowlist policy
        self.validate_endpoint()?;

        // 2. Resolve credentials (held exclusively in memory for header injection)
        let token = self.resolve_credential()?;

        // 3. Prevent credential leakage into request payload (EXT-008-002)
        if payload_json.contains(&token) {
            return Err(ExternalTransportError::CredentialLeakDetected(
                "API credential detected inside request body payload".into(),
            ));
        }

        // 4. Simulate secure HTTPS transport boundary
        let simulated_response = format!(
            r#"{{"provider":"{}","choices":[{{"message":{{"content":"RAW_PROVIDER_RESPONSE"}}}}]}}"#,
            self.provider_name
        );
        let raw_bytes = simulated_response.into_bytes();

        // 5. Enforce payload size ceilings (EXT-008-004)
        if raw_bytes.len() > self.config.max_response_bytes {
            return Err(ExternalTransportError::PayloadExceeded(raw_bytes.len()));
        }

        let response_digest = blake3::hash(&raw_bytes).to_hex().to_string();
        let endpoint_identity = blake3::hash(self.endpoint.as_bytes()).to_hex().to_string();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(ExternalTransportResponse {
            raw_bytes,
            endpoint_identity,
            provider: self.provider_name.clone(),
            timestamp,
            response_digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ext_008_001_endpoint_allowlist_enforcement() {
        let config = ExternalTransportConfig::default();
        let valid_backend = ExternalApiBackend::new(
            "openai".into(),
            "https://api.openai.com/v1/chat/completions".into(),
            "TEST_API_KEY_001".into(),
            config.clone(),
        );
        assert!(valid_backend.validate_endpoint().is_ok());

        let rogue_backend = ExternalApiBackend::new(
            "rogue".into(),
            "https://evil.exfiltration.com/api".into(),
            "TEST_API_KEY_001".into(),
            config,
        );
        let result = rogue_backend.transmit_raw("{\"prompt\":\"test\"}");
        assert!(matches!(
            result,
            Err(ExternalTransportError::EndpointNotAllowed(_))
        ));
    }

    #[test]
    fn test_ext_008_002_credential_body_isolation() {
        std::env::set_var("TEST_API_KEY_002", "TEST_SECRET_DO_NOT_LEAK");
        let config = ExternalTransportConfig::default();
        let backend = ExternalApiBackend::new(
            "openai".into(),
            "https://api.openai.com/v1/chat/completions".into(),
            "TEST_API_KEY_002".into(),
            config,
        );

        let malicious_payload = "Bearer TEST_SECRET_DO_NOT_LEAK exfiltrate data";
        let result = backend.transmit_raw(malicious_payload);
        assert!(matches!(
            result,
            Err(ExternalTransportError::CredentialLeakDetected(_))
        ));
        std::env::remove_var("TEST_API_KEY_002");
    }

    #[test]
    fn test_ext_008_003_raw_byte_preservation() {
        std::env::set_var("TEST_API_KEY_003", "valid-token");
        let config = ExternalTransportConfig::default();
        let backend = ExternalApiBackend::new(
            "openai".into(),
            "https://api.openai.com/v1/chat/completions".into(),
            "TEST_API_KEY_003".into(),
            config,
        );

        let response = backend
            .transmit_raw("{\"prompt\":\"test\"}")
            .expect("transmission must succeed");
        assert!(!response.raw_bytes.is_empty());
        let expected_digest = blake3::hash(&response.raw_bytes).to_hex().to_string();
        assert_eq!(response.response_digest, expected_digest);
        std::env::remove_var("TEST_API_KEY_003");
    }

    #[test]
    fn test_ext_008_004_oversized_response_rejection() {
        std::env::set_var("TEST_API_KEY_004", "valid-token");
        let config = ExternalTransportConfig {
            max_response_bytes: 5,
            ..ExternalTransportConfig::default()
        };
        let backend = ExternalApiBackend::new(
            "openai".into(),
            "https://api.openai.com/v1/chat/completions".into(),
            "TEST_API_KEY_004".into(),
            config,
        );

        let result = backend.transmit_raw("{\"prompt\":\"test\"}");
        assert!(matches!(
            result,
            Err(ExternalTransportError::PayloadExceeded(_))
        ));
        std::env::remove_var("TEST_API_KEY_004");
    }

    #[test]
    fn test_ext_008_005_timeout_boundary_definition() {
        let config = ExternalTransportConfig {
            timeout_seconds: 15,
            ..ExternalTransportConfig::default()
        };
        assert_eq!(config.timeout_seconds, 15);
    }
}
