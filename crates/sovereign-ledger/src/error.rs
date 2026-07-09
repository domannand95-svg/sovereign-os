use core::fmt;

/// Fail-closed error domain for the sovereign-ledger crate.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LedgerError {
    InvalidChecksum,
    LsnSequenceGap,
    SegmentCorrupted,
    StorageExhausted,
    UnsupportedVersion,
    WriteViolation,
    LsnOverflow,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::InvalidChecksum => f.write_str("ledger checksum validation failed"),
            LedgerError::LsnSequenceGap => f.write_str("ledger LSN sequence gap detected"),
            LedgerError::SegmentCorrupted => f.write_str("ledger segment corruption detected"),
            LedgerError::StorageExhausted => f.write_str("ledger storage exhausted"),
            LedgerError::UnsupportedVersion => f.write_str("unsupported ledger format version"),
            LedgerError::WriteViolation => f.write_str("ledger write violation"),
            LedgerError::LsnOverflow => f.write_str("ledger LSN overflow"),
        }
    }
}

impl std::error::Error for LedgerError {}
