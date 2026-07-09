use crate::LedgerError;
use core::fmt;

/// Strongly typed Log Sequence Number.
///
/// LSN values are scalar, monotonic identifiers assigned to committed
/// ledger records.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Lsn(pub u64);

impl Lsn {
    /// First valid sequence value.
    pub const GENESIS: Self = Self(0);

    /// Returns the underlying scalar value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next sequential LSN or fails closed on overflow.
    pub fn next(self) -> Result<Self, LedgerError> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(LedgerError::LsnOverflow),
        }
    }

    /// Verifies that `next` is exactly one step after `self`.
    pub fn ensure_next(self, next: Self) -> Result<(), LedgerError> {
        match self.next() {
            Ok(expected) if expected == next => Ok(()),
            _ => Err(LedgerError::LsnSequenceGap),
        }
    }
}

impl fmt::Debug for Lsn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Lsn").field(&self.0).finish()
    }
}

impl fmt::Display for Lsn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<u64> for Lsn {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Lsn> for u64 {
    fn from(value: Lsn) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_is_zero() {
        assert_eq!(Lsn::GENESIS.get(), 0);
    }

    #[test]
    fn next_increments_monotonically() {
        assert_eq!(Lsn(41).next(), Ok(Lsn(42)));
    }

    #[test]
    fn next_fails_closed_on_overflow() {
        assert_eq!(Lsn(u64::MAX).next(), Err(LedgerError::LsnOverflow));
    }

    #[test]
    fn ensure_next_accepts_dense_sequence() {
        assert_eq!(Lsn(7).ensure_next(Lsn(8)), Ok(()));
    }

    #[test]
    fn ensure_next_rejects_gap() {
        assert_eq!(Lsn(7).ensure_next(Lsn(9)), Err(LedgerError::LsnSequenceGap));
    }

    #[test]
    fn ensure_next_rejects_duplicate() {
        assert_eq!(Lsn(7).ensure_next(Lsn(7)), Err(LedgerError::LsnSequenceGap));
    }
}
