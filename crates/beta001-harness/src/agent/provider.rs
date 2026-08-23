//! Provider Adapter Registry & Abstraction Layer — ADAM-009-A
//!
//! Enforces provider-neutral adapter dispatch and identity isolation.
//! Invariant: Provider Identity ≠ Trust Identity (Δ Authority = 0)

use std::collections::BTreeMap;
use std::sync::Arc;

use super::external::ExternalTransportResponse;
use crate::agent::{AgentBackendError, AgentInput};

/// Metadata associated with a registered provider adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRegistration {
    pub provider_identity: String,
    pub adapter_identity: String,
    pub adapter_version: String,
    pub protocol_version: String,
}

/// Provider adapter trait isolating transport execution from governance trust.
pub trait ProviderAdapter: Send + Sync {
    fn provider_identity(&self) -> &str;
    fn adapter_identity(&self) -> &str;
    fn adapter_version(&self) -> &str;
    fn protocol_version(&self) -> &str;

    fn execute(&self, input: &AgentInput) -> Result<ExternalTransportResponse, AgentBackendError>;
}

/// Thread-safe registry for discovering and dispatching provider adapters.
pub struct ProviderAdapterRegistry {
    adapters: BTreeMap<String, Arc<dyn ProviderAdapter>>,
}

impl Default for ProviderAdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Arc<dyn ProviderAdapter>) {
        self.adapters
            .insert(adapter.provider_identity().to_string(), adapter);
    }

    pub fn resolve(&self, provider_identity: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapters.get(provider_identity).cloned()
    }

    pub fn registered_providers(&self) -> Vec<String> {
        self.adapters.keys().cloned().collect()
    }
}
