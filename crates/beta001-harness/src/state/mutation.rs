//! ADAM-012-A: State Mutations & Normalization Protocol
//!
//! Provides deterministic mutation encoding, sequence-preserving normalization (last-write-wins),
//! and canonical delta digest derivation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DELTA_DOMAIN_TAG: &str = "SOVEREIGN_OS:DELTA:v1";

/// Fundamental atomic mutation operations supported by the state store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateMutation {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

impl StateMutation {
    pub fn put(key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Self {
        Self::Put {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn delete(key: impl Into<Vec<u8>>) -> Self {
        Self::Delete { key: key.into() }
    }

    pub fn key(&self) -> &[u8] {
        match self {
            Self::Put { key, .. } => key.as_slice(),
            Self::Delete { key } => key.as_slice(),
        }
    }
}

/// Normalizes a sequence of raw mutations:
/// 1. Enforces last-write-wins based on explicit sequential appearance index.
/// 2. Sorts retained final operations by raw key bytes in ascending lexicographical order.
pub fn normalize_mutations(mutations: &[StateMutation]) -> Vec<StateMutation> {
    let mut latest_per_key: HashMap<&[u8], (usize, &StateMutation)> = HashMap::new();

    for (seq_idx, mutation) in mutations.iter().enumerate() {
        latest_per_key.insert(mutation.key(), (seq_idx, mutation));
    }

    let mut retained: Vec<(usize, &StateMutation)> = latest_per_key.into_values().collect();

    // Sort by key lexicographical bytes for canonical deterministic ordering
    retained.sort_by(|a, b| a.1.key().cmp(b.1.key()));

    retained.into_iter().map(|(_, m)| m.clone()).collect()
}

/// Derives the canonical domain-separated digest for a normalized sequence of mutations.
pub fn compute_delta_digest(normalized_mutations: &[StateMutation]) -> String {
    let mut hasher = blake3::Hasher::new();

    // Domain tag with length prefix
    hasher.update(&(DELTA_DOMAIN_TAG.len() as u64).to_be_bytes());
    hasher.update(DELTA_DOMAIN_TAG.as_bytes());

    // Mutation count
    hasher.update(&(normalized_mutations.len() as u64).to_be_bytes());

    for mutation in normalized_mutations {
        match mutation {
            StateMutation::Put { key, value } => {
                hasher.update(&1u8.to_be_bytes()); // Opcode 0x01: Put
                hasher.update(&(key.len() as u64).to_be_bytes());
                hasher.update(key);
                hasher.update(&(value.len() as u64).to_be_bytes());
                hasher.update(value);
            }
            StateMutation::Delete { key } => {
                hasher.update(&2u8.to_be_bytes()); // Opcode 0x02: Delete
                hasher.update(&(key.len() as u64).to_be_bytes());
                hasher.update(key);
            }
        }
    }

    hasher.finalize().to_hex().to_string()
}
