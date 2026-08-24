//! ADAM-012-E: Cryptographic Transition Roots & Causal Lineage Receipts
//!
//! Separates state content commitments (StateRoot) from chronological provenance commitments (TransitionRoot).

use serde::{Deserialize, Serialize};

use crate::dispatch::TerminalExecutionStatus;
use crate::service_contract::{ExecutionId, Sha256Digest};

pub const TRANSITION_ROOT_DOMAIN_TAG: &str = "SOVEREIGN_OS:STATE_TRANSITION:v1";
pub const TRANSITION_GENESIS_DOMAIN_TAG: &str = "SOVEREIGN_OS:TRANSITION_GENESIS:v1";

/// Canonical receipt recording the cryptographic state transition proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransitionReceipt {
    pub transition_root: String,
    pub previous_transition_root: String,
    pub initial_state_root: String,
    pub final_state_root: String,
    pub initial_revision: u64,
    pub final_revision: u64,
    pub execution_id: ExecutionId,
    pub package_digest: Sha256Digest,
    pub delta_digest: String,
    pub status: TerminalExecutionStatus,
    pub sequence_tick: u64,
}

impl StateTransitionReceipt {
    /// Verifies the internal cryptographic consistency of the transition receipt.
    pub fn verify_integrity(&self) -> bool {
        let expected_root = compute_transition_root(
            &self.previous_transition_root,
            &self.initial_state_root,
            &self.final_state_root,
            self.package_digest.as_str(),
            self.execution_id.as_str(),
            &self.delta_digest,
        );
        self.transition_root == expected_root
    }
}

/// Computes the deterministic Genesis Transition Root (T_0).
pub fn compute_genesis_transition_root() -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRANSITION_GENESIS_DOMAIN_TAG.as_bytes());
    hasher.update(&0u64.to_be_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Computes the causal TransitionRoot_{n+1} across domain-separated length-prefixed inputs.
pub fn compute_transition_root(
    previous_transition_root: &str,
    initial_state_root: &str,
    final_state_root: &str,
    package_digest: &str,
    execution_id: &str,
    delta_digest: &str,
) -> String {
    let mut hasher = blake3::Hasher::new();

    // 1. Domain Tag
    hasher.update(&(TRANSITION_ROOT_DOMAIN_TAG.len() as u64).to_be_bytes());
    hasher.update(TRANSITION_ROOT_DOMAIN_TAG.as_bytes());

    // 2. Length-prefixed Previous Transition Root
    hasher.update(&(previous_transition_root.len() as u64).to_be_bytes());
    hasher.update(previous_transition_root.as_bytes());

    // 3. Length-prefixed Initial State Root
    hasher.update(&(initial_state_root.len() as u64).to_be_bytes());
    hasher.update(initial_state_root.as_bytes());

    // 4. Length-prefixed Final State Root
    hasher.update(&(final_state_root.len() as u64).to_be_bytes());
    hasher.update(final_state_root.as_bytes());

    // 5. Length-prefixed Package Digest
    hasher.update(&(package_digest.len() as u64).to_be_bytes());
    hasher.update(package_digest.as_bytes());

    // 6. Length-prefixed Execution ID
    hasher.update(&(execution_id.len() as u64).to_be_bytes());
    hasher.update(execution_id.as_bytes());

    // 7. Length-prefixed Delta Digest
    hasher.update(&(delta_digest.len() as u64).to_be_bytes());
    hasher.update(delta_digest.as_bytes());

    hasher.finalize().to_hex().to_string()
}
