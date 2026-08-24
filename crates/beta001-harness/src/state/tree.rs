//! ADAM-012-A / 013-C: Canonical In-Memory State Tree & Root Derivation

use super::mutation::StateMutation;
use std::collections::BTreeMap;

pub const STATE_ROOT_DOMAIN: &str = "SOVEREIGN_OS:STATE_ROOT:v1";

/// Canonical state tree storing key-value pairs sorted lexicographically by key.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StateTree {
    records: BTreeMap<Vec<u8>, Vec<u8>>,
    revision: u64,
}

impl StateTree {
    pub fn new() -> Self {
        Self {
            records: BTreeMap::new(),
            revision: 0,
        }
    }

    pub fn from_entries(records: BTreeMap<Vec<u8>, Vec<u8>>, revision: u64) -> Self {
        Self { records, revision }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.records.get(key).map(|v| v.as_slice())
    }

    pub fn entries(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Applies a batch of raw mutations. Returns true if any mutation caused state drift.
    pub fn apply_raw_mutations(&mut self, mutations: &[StateMutation]) -> bool {
        let mut mutated = false;

        for m in mutations {
            match m {
                StateMutation::Put { key, value } => {
                    let prev = self.records.insert(key.clone(), value.clone());
                    if prev.as_ref() != Some(value) {
                        mutated = true;
                    }
                }
                StateMutation::Delete { key } => {
                    if self.records.remove(key).is_some() {
                        mutated = true;
                    }
                }
            }
        }

        if mutated {
            self.revision += 1;
        }

        mutated
    }

    /// Computes the canonical BLAKE3 state root over lexicographically sorted entries.
    pub fn compute_state_root(&self) -> String {
        let mut hasher = blake3::Hasher::new();

        // 1. Domain tag
        hasher.update(&(STATE_ROOT_DOMAIN.len() as u64).to_be_bytes());
        hasher.update(STATE_ROOT_DOMAIN.as_bytes());

        // 2. Count of entries
        hasher.update(&(self.records.len() as u64).to_be_bytes());

        // 3. Length-prefixed entries
        for (k, v) in &self.records {
            hasher.update(&(k.len() as u64).to_be_bytes());
            hasher.update(k);
            hasher.update(&(v.len() as u64).to_be_bytes());
            hasher.update(v);
        }

        hasher.finalize().to_hex().to_string()
    }
}
