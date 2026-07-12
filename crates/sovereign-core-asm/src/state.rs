//! Bounded static coordinates and runtime storage allocation errors.

/// The maximum bounded allocation threshold for the execution state vector layout.
pub const STATE_VECTOR_CAPACITY: usize = 1024;

/// The maximum byte capacity of an individual state slot.
pub const STATE_SLOT_CAPACITY: usize = 64;

/// A deterministic execution address providing bounded, constant-time indexing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateCoordinate(u32);

impl StateCoordinate {
    /// Creates a coordinate within the legal vector-space boundary.
    #[inline]
    pub const fn new(value: u32) -> Result<Self, StateError> {
        if value >= STATE_VECTOR_CAPACITY as u32 {
            return Err(StateError::OutOfBoundsCoordinate);
        }

        Ok(Self(value))
    }

    /// Returns the underlying execution index.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// A fixed-size state cell with no internal heap allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateSlot {
    length: u32,
    payload: [u8; STATE_SLOT_CAPACITY],
}

impl Default for StateSlot {
    fn default() -> Self {
        Self::new()
    }
}

impl StateSlot {
    /// Creates an empty, zero-initialized state slot.
    #[inline]
    pub const fn new() -> Self {
        Self {
            length: 0,
            payload: [0; STATE_SLOT_CAPACITY],
        }
    }

    /// Replaces the slot contents after validating the payload boundary.
    ///
    /// Bytes outside the active payload window are reset to zero.
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<(), StateError> {
        if data.len() > STATE_SLOT_CAPACITY {
            return Err(StateError::PayloadTooLarge);
        }

        self.payload = [0; STATE_SLOT_CAPACITY];
        self.payload[..data.len()].copy_from_slice(data);
        self.length = data.len() as u32;

        Ok(())
    }

    /// Returns the active payload bytes.
    #[inline]
    pub fn read_bytes(&self) -> &[u8] {
        &self.payload[..self.length as usize]
    }

    /// Returns the active payload length.
    #[inline]
    pub const fn len(&self) -> usize {
        self.length as usize
    }

    /// Reports whether the slot contains no active bytes.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }
}

/// A fixed-capacity, contiguous state storage vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateVector {
    slots: [StateSlot; STATE_VECTOR_CAPACITY],
}

impl StateVector {
    /// Creates a fully zero-initialized state vector.
    #[inline]
    pub const fn new() -> Self {
        Self {
            slots: [StateSlot::new(); STATE_VECTOR_CAPACITY],
        }
    }

    /// Returns the slot at a checked coordinate.
    #[inline]
    pub fn get(&self, coordinate: StateCoordinate) -> &StateSlot {
        &self.slots[coordinate.as_u32() as usize]
    }

    /// Returns mutable access to the slot at a checked coordinate.
    #[inline]
    pub fn get_mut(&mut self, coordinate: StateCoordinate) -> &mut StateSlot {
        &mut self.slots[coordinate.as_u32() as usize]
    }

    /// Replaces the bytes at a checked coordinate.
    pub fn write(&mut self, coordinate: StateCoordinate, data: &[u8]) -> Result<(), StateError> {
        self.get_mut(coordinate).write_bytes(data)
    }
}

impl Default for StateVector {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors produced by static coordinate and storage primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateError {
    /// Coordinate falls outside the static address-space boundary.
    OutOfBoundsCoordinate,
    /// Payload exceeds the fixed slot capacity.
    PayloadTooLarge,
}

/// An opaque 32-byte commitment representing canonical state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateRoot([u8; 32]);

impl StateRoot {
    /// Creates a state commitment from an existing 32-byte value.
    #[inline]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the underlying commitment bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Applies a deterministic state mutation and returns the data required to undo it.
pub trait StateTransition {
    /// The strongly typed execution error boundary.
    type Error;

    /// The fixed-size rollback receipt type.
    type Receipt;

    /// Applies the transition to the state vector.
    fn apply(&self, vector: &mut StateVector) -> Result<Self::Receipt, Self::Error>;

    /// Restores the exact prior state using the supplied receipt.
    fn rollback(&self, vector: &mut StateVector, receipt: Self::Receipt)
        -> Result<(), Self::Error>;
}

/// Errors produced by deterministic state transitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionError {
    /// An underlying state-storage operation failed.
    Storage(StateError),

    /// The transition preconditions were not satisfied.
    InvalidExecution,

    /// The supplied rollback receipt does not belong to this transition.
    ReceiptMismatch,
}

impl From<StateError> for TransitionError {
    #[inline]
    fn from(error: StateError) -> Self {
        Self::Storage(error)
    }
}

/// A fixed-size rollback receipt for a single-coordinate mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotReceipt {
    coordinate: StateCoordinate,
    previous: StateSlot,
}

impl SlotReceipt {
    /// Creates a receipt containing the complete prior slot image.
    #[inline]
    pub const fn new(coordinate: StateCoordinate, previous: StateSlot) -> Self {
        Self {
            coordinate,
            previous,
        }
    }

    /// Returns the coordinate associated with this receipt.
    #[inline]
    pub const fn coordinate(self) -> StateCoordinate {
        self.coordinate
    }

    /// Returns the complete prior slot image.
    #[inline]
    pub const fn previous(self) -> StateSlot {
        self.previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_constructor_accepts_valid_bounds() {
        let value = (STATE_VECTOR_CAPACITY - 1) as u32;
        let coordinate = StateCoordinate::new(value).unwrap();

        assert_eq!(coordinate.as_u32(), value);
    }

    #[test]
    fn coordinate_constructor_rejects_out_of_bounds() {
        let value = STATE_VECTOR_CAPACITY as u32;

        assert_eq!(
            StateCoordinate::new(value),
            Err(StateError::OutOfBoundsCoordinate)
        );
    }

    #[test]
    fn slot_write_and_read_succeeds() {
        let mut slot = StateSlot::new();
        let data = b"sovereign-core-payload";

        assert_eq!(slot.write_bytes(data), Ok(()));
        assert_eq!(slot.len(), data.len());
        assert!(!slot.is_empty());
        assert_eq!(slot.read_bytes(), data);
    }

    #[test]
    fn slot_accepts_exact_capacity() {
        let mut slot = StateSlot::new();
        let data = [7; STATE_SLOT_CAPACITY];

        assert_eq!(slot.write_bytes(&data), Ok(()));
        assert_eq!(slot.read_bytes(), &data);
    }

    #[test]
    fn oversized_write_preserves_existing_payload() {
        let mut slot = StateSlot::new();
        slot.write_bytes(b"valid").unwrap();

        let oversized = [1; STATE_SLOT_CAPACITY + 1];

        assert_eq!(
            slot.write_bytes(&oversized),
            Err(StateError::PayloadTooLarge)
        );
        assert_eq!(slot.read_bytes(), b"valid");
    }

    #[test]
    fn shorter_overwrite_updates_active_window() {
        let mut slot = StateSlot::new();
        slot.write_bytes(&[5; STATE_SLOT_CAPACITY]).unwrap();
        slot.write_bytes(b"new").unwrap();

        assert_eq!(slot.len(), 3);
        assert_eq!(slot.read_bytes(), b"new");
        assert_eq!(&slot.payload[3..], &[0; STATE_SLOT_CAPACITY - 3]);
    }

    #[test]
    fn default_slot_is_empty() {
        let slot = StateSlot::default();

        assert!(slot.is_empty());
        assert_eq!(slot.read_bytes(), b"");
    }

    #[test]
    fn vector_new_initializes_all_slots_empty() {
        let vector = StateVector::new();

        for index in 0..STATE_VECTOR_CAPACITY {
            let coordinate = StateCoordinate::new(index as u32).unwrap();
            assert!(vector.get(coordinate).is_empty());
        }
    }

    #[test]
    fn vector_write_and_get_round_trip() {
        let mut vector = StateVector::new();
        let coordinate = StateCoordinate::new(42).unwrap();
        let data = b"vector-test";

        assert_eq!(vector.write(coordinate, data), Ok(()));
        assert_eq!(vector.get(coordinate).read_bytes(), data);
    }

    #[test]
    fn mutations_at_distinct_coordinates_remain_isolated() {
        let mut vector = StateVector::new();
        let coordinate_a = StateCoordinate::new(10).unwrap();
        let coordinate_b = StateCoordinate::new(20).unwrap();

        vector.write(coordinate_a, b"payload-a").unwrap();
        vector.write(coordinate_b, b"payload-b").unwrap();

        assert_eq!(vector.get(coordinate_a).read_bytes(), b"payload-a");
        assert_eq!(vector.get(coordinate_b).read_bytes(), b"payload-b");
    }

    #[test]
    fn failed_oversized_write_preserves_existing_slot_value() {
        let mut vector = StateVector::new();
        let coordinate = StateCoordinate::new(7).unwrap();

        vector.write(coordinate, b"initial").unwrap();

        let oversized = [9u8; STATE_SLOT_CAPACITY + 1];

        assert_eq!(
            vector.write(coordinate, &oversized),
            Err(StateError::PayloadTooLarge)
        );
        assert_eq!(vector.get(coordinate).read_bytes(), b"initial");
    }

    #[test]
    fn default_vector_matches_new_vector() {
        assert_eq!(StateVector::default(), StateVector::new());
    }

    #[test]
    fn root_construction_preserves_bytes() {
        let bytes = [0xAAu8; 32];
        let root = StateRoot::new(bytes);

        assert_eq!(root.as_bytes(), &bytes);
    }

    #[test]
    fn root_equality_and_ordering_are_structural() {
        let root_a = StateRoot::new([0u8; 32]);
        let root_b = StateRoot::new([0u8; 32]);

        let mut larger_bytes = [0u8; 32];
        larger_bytes[31] = 1;
        let root_c = StateRoot::new(larger_bytes);

        assert_eq!(root_a, root_b);
        assert_ne!(root_a, root_c);
        assert!(root_a < root_c);
    }

    #[test]
    fn equal_roots_hash_identically() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let root_a = StateRoot::new([5u8; 32]);
        let root_b = StateRoot::new([5u8; 32]);

        let mut hasher_a = DefaultHasher::new();
        let mut hasher_b = DefaultHasher::new();

        root_a.hash(&mut hasher_a);
        root_b.hash(&mut hasher_b);

        assert_eq!(hasher_a.finish(), hasher_b.finish());
    }

    #[test]
    fn transition_apply_and_rollback_with_receipt() {
        struct MockReplaceTransition {
            coordinate: StateCoordinate,
            target_payload: [u8; 4],
        }

        impl StateTransition for MockReplaceTransition {
            type Error = TransitionError;
            type Receipt = SlotReceipt;

            fn apply(&self, vector: &mut StateVector) -> Result<Self::Receipt, Self::Error> {
                let previous = *vector.get(self.coordinate);

                vector.write(self.coordinate, &self.target_payload)?;

                Ok(SlotReceipt::new(self.coordinate, previous))
            }

            fn rollback(
                &self,
                vector: &mut StateVector,
                receipt: Self::Receipt,
            ) -> Result<(), Self::Error> {
                if receipt.coordinate() != self.coordinate {
                    return Err(TransitionError::ReceiptMismatch);
                }

                *vector.get_mut(self.coordinate) = receipt.previous();

                Ok(())
            }
        }

        let mut vector = StateVector::new();
        let coordinate = StateCoordinate::new(100).unwrap();

        vector.write(coordinate, &[1u8, 2u8, 3u8, 4u8]).unwrap();

        let transition = MockReplaceTransition {
            coordinate,
            target_payload: [9u8, 9u8, 9u8, 9u8],
        };

        let receipt = transition.apply(&mut vector).unwrap();

        assert_eq!(vector.get(coordinate).read_bytes(), &[9u8, 9u8, 9u8, 9u8]);

        transition.rollback(&mut vector, receipt).unwrap();

        assert_eq!(vector.get(coordinate).read_bytes(), &[1u8, 2u8, 3u8, 4u8]);
    }

    #[test]
    fn transition_rejects_mismatched_receipt_coordinate() {
        struct MockReplaceTransition {
            coordinate: StateCoordinate,
            target_payload: [u8; 4],
        }

        impl StateTransition for MockReplaceTransition {
            type Error = TransitionError;
            type Receipt = SlotReceipt;

            fn apply(&self, vector: &mut StateVector) -> Result<Self::Receipt, Self::Error> {
                let previous = *vector.get(self.coordinate);

                vector.write(self.coordinate, &self.target_payload)?;

                Ok(SlotReceipt::new(self.coordinate, previous))
            }

            fn rollback(
                &self,
                vector: &mut StateVector,
                receipt: Self::Receipt,
            ) -> Result<(), Self::Error> {
                if receipt.coordinate() != self.coordinate {
                    return Err(TransitionError::ReceiptMismatch);
                }

                *vector.get_mut(self.coordinate) = receipt.previous();

                Ok(())
            }
        }

        let coordinate_a = StateCoordinate::new(10).unwrap();
        let coordinate_b = StateCoordinate::new(20).unwrap();

        let transition = MockReplaceTransition {
            coordinate: coordinate_a,
            target_payload: [9u8, 9u8, 9u8, 9u8],
        };

        let receipt = SlotReceipt::new(coordinate_b, StateSlot::new());

        let mut vector = StateVector::new();

        assert!(vector.get(coordinate_a).is_empty());
        assert!(vector.get(coordinate_b).is_empty());

        assert_eq!(
            transition.rollback(&mut vector, receipt),
            Err(TransitionError::ReceiptMismatch)
        );

        assert!(vector.get(coordinate_a).is_empty());
        assert!(vector.get(coordinate_b).is_empty());
    }

    #[test]
    fn transition_fails_closed_on_precondition_violation() {
        struct MockGuardedTransition {
            coordinate: StateCoordinate,
            magic_value: u8,
        }

        impl StateTransition for MockGuardedTransition {
            type Error = TransitionError;
            type Receipt = SlotReceipt;

            fn apply(&self, vector: &mut StateVector) -> Result<Self::Receipt, Self::Error> {
                let current = vector.get(self.coordinate);

                if current.is_empty() || current.read_bytes()[0] != self.magic_value {
                    return Err(TransitionError::InvalidExecution);
                }

                let previous = *current;

                vector.write(self.coordinate, b"updated")?;

                Ok(SlotReceipt::new(self.coordinate, previous))
            }

            fn rollback(
                &self,
                vector: &mut StateVector,
                receipt: Self::Receipt,
            ) -> Result<(), Self::Error> {
                if receipt.coordinate() != self.coordinate {
                    return Err(TransitionError::ReceiptMismatch);
                }

                *vector.get_mut(self.coordinate) = receipt.previous();

                Ok(())
            }
        }

        let mut vector = StateVector::new();
        let coordinate = StateCoordinate::new(50).unwrap();

        vector.write(coordinate, b"initial").unwrap();

        let transition = MockGuardedTransition {
            coordinate,
            magic_value: 0xFF,
        };

        assert_eq!(
            transition.apply(&mut vector),
            Err(TransitionError::InvalidExecution)
        );

        assert_eq!(vector.get(coordinate).read_bytes(), b"initial");
    }
}
