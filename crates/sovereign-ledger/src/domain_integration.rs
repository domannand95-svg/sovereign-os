//! Deterministic integration boundary between `sovereign-ledger` and
//! `sovereign-core-asm`.
//!
//! Ledger-domain interpretation occurs before core-state execution:
//!
//! ```text
//! EventRecord
//!     -> LedgerEventMapper
//!     -> MappedLedgerWrite
//!     -> LedgerStateTransition
//!     -> StateVector
//! ```
//!
//! The mapper is invoked exactly once. Rollback relies only on the mapped
//! coordinate and the `SlotReceipt` returned during application.

use crate::record::EventRecord;
use sovereign_core_asm::state::{
    SlotReceipt, StateCoordinate, StateError, StateTransition, StateVector,
};

/// A validated ledger event translated into core-state write semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MappedLedgerWrite<'a> {
    coordinate: StateCoordinate,
    payload: &'a [u8],
}

impl<'a> MappedLedgerWrite<'a> {
    /// Creates a mapped state write from a checked coordinate and payload.
    #[inline]
    pub const fn new(coordinate: StateCoordinate, payload: &'a [u8]) -> Self {
        Self {
            coordinate,
            payload,
        }
    }

    /// Returns the mapped core-state coordinate.
    #[inline]
    pub const fn coordinate(self) -> StateCoordinate {
        self.coordinate
    }

    /// Returns the mapped payload bytes.
    #[inline]
    pub const fn payload(self) -> &'a [u8] {
        self.payload
    }
}

/// Converts ledger events into validated core-state writes.
///
/// Implementations own all ledger-domain mapping policy. The deterministic
/// core remains unaware of event types, log sequence numbers, and ledger
/// encoding rules.
pub trait LedgerEventMapper {
    /// Error returned when a ledger event cannot be mapped.
    type Error;

    /// Maps one ledger event into one validated core-state write.
    fn map<'payload>(
        &self,
        event: &EventRecord<'payload>,
    ) -> Result<MappedLedgerWrite<'payload>, Self::Error>;
}

/// Errors produced while executing an already mapped ledger transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerTransitionError {
    /// The core-state storage operation failed.
    Storage(StateError),

    /// The rollback receipt belongs to a different coordinate.
    ReceiptMismatch,
}

impl From<StateError> for LedgerTransitionError {
    #[inline]
    fn from(error: StateError) -> Self {
        Self::Storage(error)
    }
}

/// A deterministic transition built from a previously mapped ledger event.
///
/// Construction performs domain mapping once. Application and rollback do not
/// reinterpret the original `EventRecord`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LedgerStateTransition<'a> {
    write: MappedLedgerWrite<'a>,
}

impl<'a> LedgerStateTransition<'a> {
    /// Maps a ledger event once and constructs its deterministic transition.
    pub fn from_event<M>(event: &EventRecord<'a>, mapper: &M) -> Result<Self, M::Error>
    where
        M: LedgerEventMapper,
    {
        let write = mapper.map(event)?;
        Ok(Self { write })
    }

    /// Constructs a transition from an already validated mapped write.
    #[inline]
    pub const fn from_mapped(write: MappedLedgerWrite<'a>) -> Self {
        Self { write }
    }

    /// Returns the immutable mapped write used by this transition.
    #[inline]
    pub const fn mapped_write(self) -> MappedLedgerWrite<'a> {
        self.write
    }
}

impl StateTransition for LedgerStateTransition<'_> {
    type Error = LedgerTransitionError;
    type Receipt = SlotReceipt;

    fn apply(&self, vector: &mut StateVector) -> Result<Self::Receipt, Self::Error> {
        let coordinate = self.write.coordinate();
        let previous = *vector.get(coordinate);

        vector.write(coordinate, self.write.payload())?;

        Ok(SlotReceipt::new(coordinate, previous))
    }

    fn rollback(
        &self,
        vector: &mut StateVector,
        receipt: Self::Receipt,
    ) -> Result<(), Self::Error> {
        if receipt.coordinate() != self.write.coordinate() {
            return Err(LedgerTransitionError::ReceiptMismatch);
        }

        *vector.get_mut(receipt.coordinate()) = receipt.previous();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsn::Lsn;
    use crate::record::EventType;
    use sovereign_core_asm::state::{StateSlot, STATE_SLOT_CAPACITY};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestMappingError {
        Rejected,
    }

    struct FixedMapper {
        coordinate: StateCoordinate,
    }

    impl LedgerEventMapper for FixedMapper {
        type Error = TestMappingError;

        fn map<'payload>(
            &self,
            event: &EventRecord<'payload>,
        ) -> Result<MappedLedgerWrite<'payload>, Self::Error> {
            Ok(MappedLedgerWrite::new(self.coordinate, event.payload))
        }
    }

    struct RejectingMapper;

    impl LedgerEventMapper for RejectingMapper {
        type Error = TestMappingError;

        fn map<'payload>(
            &self,
            _event: &EventRecord<'payload>,
        ) -> Result<MappedLedgerWrite<'payload>, Self::Error> {
            Err(TestMappingError::Rejected)
        }
    }

    fn event(payload: &[u8]) -> EventRecord<'_> {
        EventRecord {
            lsn: Lsn::from(42),
            event_type: EventType::RegistryMutation,
            payload,
            checksum: 0,
        }
    }

    #[test]
    fn mapper_preserves_coordinate_and_payload() {
        let coordinate = StateCoordinate::new(10).unwrap();
        let mapper = FixedMapper { coordinate };
        let record = event(b"mapped-payload");

        let mapped = mapper.map(&record).unwrap();

        assert_eq!(mapped.coordinate(), coordinate);
        assert_eq!(mapped.payload(), b"mapped-payload");
    }

    #[test]
    fn transition_apply_and_rollback_round_trip() {
        let coordinate = StateCoordinate::new(20).unwrap();
        let mapper = FixedMapper { coordinate };
        let record = event(b"ledger-state");
        let transition = LedgerStateTransition::from_event(&record, &mapper).unwrap();

        let mut vector = StateVector::new();
        vector.write(coordinate, b"prior-state").unwrap();

        let receipt = transition.apply(&mut vector).unwrap();

        assert_eq!(vector.get(coordinate).read_bytes(), b"ledger-state");

        transition.rollback(&mut vector, receipt).unwrap();

        assert_eq!(vector.get(coordinate).read_bytes(), b"prior-state");
    }

    #[test]
    fn oversized_payload_fails_without_mutating_state() {
        let coordinate = StateCoordinate::new(30).unwrap();
        let oversized = [9u8; STATE_SLOT_CAPACITY + 1];
        let record = event(&oversized);
        let mapper = FixedMapper { coordinate };
        let transition = LedgerStateTransition::from_event(&record, &mapper).unwrap();

        let mut vector = StateVector::new();
        vector.write(coordinate, b"preserved").unwrap();

        assert_eq!(
            transition.apply(&mut vector),
            Err(LedgerTransitionError::Storage(StateError::PayloadTooLarge))
        );

        assert_eq!(vector.get(coordinate).read_bytes(), b"preserved");
    }

    #[test]
    fn rollback_rejects_receipt_for_another_coordinate() {
        let coordinate = StateCoordinate::new(40).unwrap();
        let other_coordinate = StateCoordinate::new(41).unwrap();
        let mapper = FixedMapper { coordinate };
        let record = event(b"replacement");
        let transition = LedgerStateTransition::from_event(&record, &mapper).unwrap();

        let mut vector = StateVector::new();
        vector.write(coordinate, b"original").unwrap();

        let mismatched = SlotReceipt::new(other_coordinate, StateSlot::new());

        assert_eq!(
            transition.rollback(&mut vector, mismatched),
            Err(LedgerTransitionError::ReceiptMismatch)
        );

        assert_eq!(vector.get(coordinate).read_bytes(), b"original");
        assert!(vector.get(other_coordinate).is_empty());
    }

    #[test]
    fn mapper_failure_occurs_before_state_execution() {
        let coordinate = StateCoordinate::new(50).unwrap();
        let record = event(b"rejected");

        let result = LedgerStateTransition::from_event(&record, &RejectingMapper);

        assert_eq!(result, Err(TestMappingError::Rejected));

        let vector = StateVector::new();
        assert!(vector.get(coordinate).is_empty());
    }
}
