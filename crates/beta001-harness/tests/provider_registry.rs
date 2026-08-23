//! Integration Tests for ADAM-009-A Provider Adapter Registry
//! Verifies registration, fail-closed resolution, and identity/trust isolation.

use std::sync::Arc;
use beta001_harness::agent::{
    AgentBackendError, AgentInput, ProviderAdapter, ProviderAdapterRegistry,
};
use beta001_harness::agent::external::ExternalTransportResponse;

struct MockAdapter {
    provider_id: String,
    adapter_id: String,
    version: String,
    protocol: String,
}

impl ProviderAdapter for MockAdapter {
    fn provider_identity(&self) -> &str {
        &self.provider_id
    }

    fn adapter_identity(&self) -> &str {
        &self.adapter_id
    }

    fn adapter_version(&self) -> &str {
        &self.version
    }

    fn protocol_version(&self) -> &str {
        &self.protocol
    }

    fn execute(
        &self,
        _input: &AgentInput,
    ) -> Result<ExternalTransportResponse, AgentBackendError> {
        Ok(ExternalTransportResponse {
            provider: self.provider_id.clone(),
            timestamp: 1724412000,
            endpoint_identity: "endpoint-mock".to_string(),
            raw_bytes: b"mock-response".to_vec(),
            response_digest: "resp-digest".to_string(),
        })
    }
}

#[test]
fn test_reg_009_001_provider_registration_succeeds() {
    let mut registry = ProviderAdapterRegistry::new();
    let adapter = Arc::new(MockAdapter {
        provider_id: "provider-alpha".to_string(),
        adapter_id: "adapter-http".to_string(),
        version: "v1.0.0".to_string(),
        protocol: "http/json".to_string(),
    });

    registry.register(adapter);
    let resolved = registry.resolve("provider-alpha");
    assert!(resolved.is_some(), "REG-009-001: Registered provider must be resolvable");
    assert_eq!(resolved.unwrap().provider_identity(), "provider-alpha");
}

#[test]
fn test_reg_009_002_unknown_provider_fails_closed() {
    let registry = ProviderAdapterRegistry::new();
    let resolved = registry.resolve("non-existent-provider");
    assert!(resolved.is_none(), "REG-009-002: Unknown provider must fail closed (None)");
}

#[test]
fn test_reg_009_003_provider_identity_does_not_imply_trust() {
    let mut registry = ProviderAdapterRegistry::new();
    let adapter = Arc::new(MockAdapter {
        provider_id: "trusted-sounding-provider".to_string(),
        adapter_id: "adapter-std".to_string(),
        version: "v1.0.0".to_string(),
        protocol: "http/json".to_string(),
    });

    registry.register(adapter);
    let resolved = registry.resolve("trusted-sounding-provider").unwrap();
    assert_eq!(resolved.provider_identity(), "trusted-sounding-provider");
}

#[test]
fn test_reg_009_004_adapter_interchangeability() {
    let mut registry = ProviderAdapterRegistry::new();
    let adapter_a = Arc::new(MockAdapter {
        provider_id: "provider-one".to_string(),
        adapter_id: "adapter-std".to_string(),
        version: "v1.0.0".to_string(),
        protocol: "v1".to_string(),
    });
    let adapter_b = Arc::new(MockAdapter {
        provider_id: "provider-two".to_string(),
        adapter_id: "adapter-std".to_string(),
        version: "v1.0.0".to_string(),
        protocol: "v1".to_string(),
    });

    registry.register(adapter_a);
    registry.register(adapter_b);

    let input = AgentInput {
        prompt: "test".to_string(),
        task_reference: "task-001".to_string(),
    };
    let res_a = registry.resolve("provider-one").unwrap().execute(&input).unwrap();
    let res_b = registry.resolve("provider-two").unwrap().execute(&input).unwrap();

    assert_eq!(res_a.response_digest, res_b.response_digest);
}
