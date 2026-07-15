use core::fmt;

use crate::state::{StateCoordinate, StateVector, STATE_SLOT_CAPACITY, STATE_VECTOR_CAPACITY};

pub const STATE_VECTOR_SNAPSHOT_VERSION: u32 = 1;

const VERSION_BYTES: usize = core::mem::size_of::<u32>();
const SLOT_LENGTH_BYTES: usize = core::mem::size_of::<u32>();
const ENCODED_SLOT_SIZE: usize = SLOT_LENGTH_BYTES + STATE_SLOT_CAPACITY;

pub const STATE_VECTOR_SNAPSHOT_SIZE: usize =
    VERSION_BYTES + (STATE_VECTOR_CAPACITY * ENCODED_SLOT_SIZE);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotDecodeError {
    InvalidLength { expected: usize, actual: usize },
    UnsupportedVersion { expected: u32, actual: u32 },
    InvalidSlotLength { slot: usize, length: usize },
    NonZeroPadding { slot: usize, offset: usize },
}

impl fmt::Display for SnapshotDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { expected, actual } => write!(
                formatter,
                "invalid state-vector snapshot length: expected {expected} bytes, received {actual}"
            ),
            Self::UnsupportedVersion { expected, actual } => write!(
                formatter,
                "unsupported state-vector snapshot version: expected {expected}, received {actual}"
            ),
            Self::InvalidSlotLength { slot, length } => write!(
                formatter,
                "invalid encoded length {length} for state slot {slot}"
            ),
            Self::NonZeroPadding { slot, offset } => write!(
                formatter,
                "non-zero padding in state slot {slot} at payload offset {offset}"
            ),
        }
    }
}

impl std::error::Error for SnapshotDecodeError {}

pub fn encode(vector: &StateVector) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(STATE_VECTOR_SNAPSHOT_SIZE);

    encoded.extend_from_slice(&STATE_VECTOR_SNAPSHOT_VERSION.to_be_bytes());

    for index in 0..STATE_VECTOR_CAPACITY {
        let coordinate =
            StateCoordinate::new(index as u32).expect("bounded snapshot coordinate must be valid");
        let slot = vector.get(coordinate);
        let active_bytes = slot.read_bytes();

        encoded.extend_from_slice(&(active_bytes.len() as u32).to_be_bytes());
        encoded.extend_from_slice(active_bytes);
        encoded.resize(
            encoded.len() + (STATE_SLOT_CAPACITY - active_bytes.len()),
            0,
        );
    }

    debug_assert_eq!(encoded.len(), STATE_VECTOR_SNAPSHOT_SIZE);
    encoded
}

pub fn decode(bytes: &[u8]) -> Result<StateVector, SnapshotDecodeError> {
    if bytes.len() != STATE_VECTOR_SNAPSHOT_SIZE {
        return Err(SnapshotDecodeError::InvalidLength {
            expected: STATE_VECTOR_SNAPSHOT_SIZE,
            actual: bytes.len(),
        });
    }

    let version = u32::from_be_bytes(
        bytes[..VERSION_BYTES]
            .try_into()
            .expect("validated version field width"),
    );

    if version != STATE_VECTOR_SNAPSHOT_VERSION {
        return Err(SnapshotDecodeError::UnsupportedVersion {
            expected: STATE_VECTOR_SNAPSHOT_VERSION,
            actual: version,
        });
    }

    let mut restored = StateVector::new();
    let mut cursor = VERSION_BYTES;

    for index in 0..STATE_VECTOR_CAPACITY {
        let length_end = cursor + SLOT_LENGTH_BYTES;
        let slot_length = u32::from_be_bytes(
            bytes[cursor..length_end]
                .try_into()
                .expect("validated slot-length field width"),
        ) as usize;
        cursor = length_end;

        if slot_length > STATE_SLOT_CAPACITY {
            return Err(SnapshotDecodeError::InvalidSlotLength {
                slot: index,
                length: slot_length,
            });
        }

        let payload_end = cursor + STATE_SLOT_CAPACITY;
        let encoded_payload = &bytes[cursor..payload_end];
        let active_payload = &encoded_payload[..slot_length];

        for (offset, byte) in encoded_payload[slot_length..].iter().enumerate() {
            if *byte != 0 {
                return Err(SnapshotDecodeError::NonZeroPadding {
                    slot: index,
                    offset: slot_length + offset,
                });
            }
        }

        let coordinate =
            StateCoordinate::new(index as u32).expect("bounded snapshot coordinate must be valid");

        restored
            .write(coordinate, active_payload)
            .expect("validated snapshot slot length must be writable");

        cursor = payload_end;
    }

    debug_assert_eq!(cursor, STATE_VECTOR_SNAPSHOT_SIZE);
    Ok(restored)
}

pub fn restore(bytes: &[u8], destination: &mut StateVector) -> Result<(), SnapshotDecodeError> {
    let restored = decode(bytes)?;
    *destination = restored;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coordinate(index: u32) -> StateCoordinate {
        StateCoordinate::new(index).unwrap()
    }

    fn populated_vector() -> StateVector {
        let mut vector = StateVector::new();
        vector.write(coordinate(0), b"alpha").unwrap();
        vector.write(coordinate(17), b"beta").unwrap();
        vector
            .write(coordinate((STATE_VECTOR_CAPACITY - 1) as u32), &[0xAB; 64])
            .unwrap();
        vector
    }

    #[test]
    fn zero_vector_round_trip() {
        let original = StateVector::new();
        let restored = decode(&encode(&original)).unwrap();

        assert_eq!(restored, original);
    }

    #[test]
    fn populated_vector_round_trip() {
        let original = populated_vector();
        let restored = decode(&encode(&original)).unwrap();

        assert_eq!(restored, original);
    }

    #[test]
    fn identical_state_has_deterministic_encoding() {
        let first = populated_vector();
        let second = populated_vector();

        assert_eq!(encode(&first), encode(&second));
    }

    #[test]
    fn encoded_snapshot_has_exact_fixed_size() {
        assert_eq!(
            encode(&StateVector::new()).len(),
            STATE_VECTOR_SNAPSHOT_SIZE
        );
        assert_eq!(STATE_VECTOR_SNAPSHOT_SIZE, 69_636);
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let mut encoded = encode(&StateVector::new());
        encoded[..VERSION_BYTES].copy_from_slice(&2_u32.to_be_bytes());

        assert_eq!(
            decode(&encoded),
            Err(SnapshotDecodeError::UnsupportedVersion {
                expected: STATE_VECTOR_SNAPSHOT_VERSION,
                actual: 2,
            })
        );
    }

    #[test]
    fn truncated_payload_is_rejected() {
        let mut encoded = encode(&StateVector::new());
        encoded.pop();

        assert_eq!(
            decode(&encoded),
            Err(SnapshotDecodeError::InvalidLength {
                expected: STATE_VECTOR_SNAPSHOT_SIZE,
                actual: STATE_VECTOR_SNAPSHOT_SIZE - 1,
            })
        );
    }

    #[test]
    fn trailing_byte_is_rejected() {
        let mut encoded = encode(&StateVector::new());
        encoded.push(0);

        assert_eq!(
            decode(&encoded),
            Err(SnapshotDecodeError::InvalidLength {
                expected: STATE_VECTOR_SNAPSHOT_SIZE,
                actual: STATE_VECTOR_SNAPSHOT_SIZE + 1,
            })
        );
    }

    #[test]
    fn oversized_slot_length_is_rejected() {
        let mut encoded = encode(&StateVector::new());
        encoded[VERSION_BYTES..VERSION_BYTES + SLOT_LENGTH_BYTES]
            .copy_from_slice(&((STATE_SLOT_CAPACITY + 1) as u32).to_be_bytes());

        assert_eq!(
            decode(&encoded),
            Err(SnapshotDecodeError::InvalidSlotLength {
                slot: 0,
                length: STATE_SLOT_CAPACITY + 1,
            })
        );
    }

    #[test]
    fn non_zero_padding_is_rejected() {
        let mut encoded = encode(&StateVector::new());
        let first_payload_byte = VERSION_BYTES + SLOT_LENGTH_BYTES;
        encoded[first_payload_byte] = 1;

        assert_eq!(
            decode(&encoded),
            Err(SnapshotDecodeError::NonZeroPadding { slot: 0, offset: 0 })
        );
    }

    #[test]
    fn failed_restoration_preserves_destination() {
        let mut destination = populated_vector();
        let original_destination = destination.clone();

        let mut encoded = encode(&StateVector::new());
        encoded[..VERSION_BYTES].copy_from_slice(&2_u32.to_be_bytes());

        assert!(restore(&encoded, &mut destination).is_err());
        assert_eq!(destination, original_destination);
    }
}
