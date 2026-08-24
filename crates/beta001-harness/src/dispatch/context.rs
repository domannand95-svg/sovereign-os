//! ADAM-012-B: Deterministic Execution Context & Seed Derivation
//!
//! Provides strictly deterministic inputs to workers, isolating execution from ambient OS entropy.

use serde::{Deserialize, Serialize};

use crate::service_contract::{ExecutionId, ProposedOperation, Sha256Digest};

pub const EXECUTION_SEED_DOMAIN_TAG: &str = "SOVEREIGN_OS:EXECUTION_SEED:v1";

/// Immutable deterministic context supplied to workers during execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicExecutionContext {
    pub execution_id: ExecutionId,
    pub package_digest: Sha256Digest,
    pub expected_state_root: String,
    pub expected_revision: u64,
    pub deterministic_seed: [u8; 32],
    pub logical_sequence_tick: u64,
    pub granted_scope: ProposedOperation,
}

impl DeterministicExecutionContext {
    /// Derives the canonical 32-byte execution seed via domain-separated BLAKE3 hashing.
    pub fn derive_seed(
        package_digest: &Sha256Digest,
        execution_id: &ExecutionId,
        expected_revision: u64,
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        // 1. Domain Tag
        hasher.update(&(EXECUTION_SEED_DOMAIN_TAG.len() as u64).to_be_bytes());
        hasher.update(EXECUTION_SEED_DOMAIN_TAG.as_bytes());

        // 2. Package Digest
        hasher.update(&(package_digest.as_str().len() as u64).to_be_bytes());
        hasher.update(package_digest.as_str().as_bytes());

        // 3. Execution ID
        hasher.update(&(execution_id.as_str().len() as u64).to_be_bytes());
        hasher.update(execution_id.as_str().as_bytes());

        // 4. Expected Revision
        hasher.update(&8u64.to_be_bytes());
        hasher.update(&expected_revision.to_be_bytes());

        *hasher.finalize().as_bytes()
    }
}
