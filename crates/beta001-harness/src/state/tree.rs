//! ADAM-012-A / 013-C: Canonical Content-Addressed State Tree
//!
//! Provides B-Tree backed deterministic key-value storage with explicit StateRoot derivation
//! using length-prefixed big-endian encoding.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::mutation::{normalize_mutations, StateMutation};

pub const STATE_ROOT_DOMAIN: &str = "SOVEREIGN_OS:STATE_ROOT:v1";

/// Committed state store maintaining lexicographical key order and state revision tracking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTree {
    entries: BTreeMap<Vec<u8>, Vec<u8>>,
    revision: u64,
}

impl Default for StateTree {
    fn default() -> Self {
        Self::new()
    }
}

impl StateTree {
    /// Initializes a new empty state tree at revision 0.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            revision: 0,
        }
    }

    /// Hydrates a state tree directly from verified entries and revision.
    pub fn from_entries(entries: BTreeMap<Vec<u8>, Vec<u8>>, revision: u64) -> Self {
        Self { entries, revision }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.entries.get(key).map(|v| v.as_slice())
    }

    pub fn entries(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.entries
    }

    /// Computes the canonical BLAKE3 StateRoot over domain-separated, length-prefixed entries.
    pub fn compute_state_root(&self) -> String {
        let mut hasher = blake3::Hasher::new();

        // 1. Domain tag
        hasher.update(STATE_ROOT_DOMAIN.as_bytes());

        // 2. Total entry count (u64 big-endian)
        hasher.update(&(self.entries.len() as u64).to_be_bytes());

        // 3. Length-prefixed keys and values in lexicographical order
        for (key, value) in &self.entries {
            hasher.update(&(key.len() as u64).to_be_bytes());
            hasher.update(key);
            hasher.update(&(value.len() as u64).to_be_bytes());
            hasher.update(value);
        }

        hasher.finalize().to_hex().to_string()
    }

    /// Applies normalized mutations directly to the state tree.
    ///
    /// Revision semantics:
    /// - If one or more mutations result in an effective state change, revision is incremented by 1.
    /// - If the mutations result in zero effective state changes (e.g. deleting absent key, writing identical value),
    ///   the state and revision remain unchanged.
    ///
    /// Returns true if an effective change was committed.
    pub fn apply_normalized_mutations(&mut self, normalized_mutations: &[StateMutation]) -> bool {
        let mut has_effective_change = false;

        for mutation in normalized_mutations {
            match mutation {
                StateMutation::Put { key, value } => {
                    let prev = self.entries.get(key);
                    if prev != Some(value) {
                        self.entries.insert(key.clone(), value.clone());
                        has_effective_change = true;
                    }
                }
                StateMutation::Delete { key } => {
                    if self.entries.remove(key).is_some() {
                        has_effective_change = true;
                    }
                }
            }
        }

        if has_effective_change {
            self.revision += 1;
        }

        has_effective_change
    }

    /// Convenience helper to normalize and apply raw mutations in a single step.
    pub fn apply_raw_mutations(&mut self, mutations: &[StateMutation]) -> bool {
        let normalized = normalize_mutations(mutations);
        self.apply_normalized_mutations(&normalized)
    }
}
