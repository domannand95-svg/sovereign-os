//! State-root computation for ledger snapshots.
//!
//! This module implements the normative algorithm defined by ADR 0001:
//! BLAKE3("SOVEREIGN_STATE_V1" || core_asm::snapshot::encode(state_vector)).

use sovereign_core_asm::state::StateVector;

/// Domain separator for state-root computation.
///
/// The array type makes the exact 18-byte length a compile-time invariant.
pub const DOMAIN_SEPARATOR: &[u8; 18] = b"SOVEREIGN_STATE_V1";

/// Computes the state root for a canonical state vector.
pub fn compute_state_root(vector: &StateVector) -> [u8; 32] {
    let encoded = sovereign_core_asm::snapshot::encode(vector);
    compute_state_root_from_encoded(&encoded)
}

/// Computes the state root from an already encoded state-vector payload.
pub fn compute_state_root_from_encoded(encoded: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(DOMAIN_SEPARATOR);
    hasher.update(encoded);
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_root_fixed_vector() {
        let expected = [
            172, 249, 28, 122, 155, 8, 216, 10, 202, 224, 144, 91, 91, 237, 15, 5, 191, 69, 4, 79,
            100, 60, 119, 97, 209, 134, 205, 24, 165, 171, 237, 112,
        ];

        assert_eq!(compute_state_root(&StateVector::new()), expected);
    }

    #[test]
    fn domain_separator_is_exact() {
        assert_eq!(DOMAIN_SEPARATOR, b"SOVEREIGN_STATE_V1");
        assert_eq!(DOMAIN_SEPARATOR.len(), 18);
    }

    #[test]
    fn vector_and_encoded_paths_match() {
        let vector = StateVector::new();
        let encoded = sovereign_core_asm::snapshot::encode(&vector);

        assert_eq!(
            compute_state_root(&vector),
            compute_state_root_from_encoded(&encoded)
        );
    }
}
