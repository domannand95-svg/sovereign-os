use crate::domain_integration::LedgerTransitionError;
use crate::lsn::Lsn;
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
    /// A canonical record was published, but its directory durability could not be proven.
    CommitAmbiguous,
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
            LedgerError::CommitAmbiguous => {
                f.write_str("ledger commit outcome is ambiguous; restart required")
            }
            LedgerError::LsnOverflow => f.write_str("ledger LSN overflow"),
        }
    }
}

impl std::error::Error for LedgerError {}

/// Errors that can occur during snapshot-coordinated ledger restoration.
///
/// Only fatal errors are exposed. Recoverable snapshot failures trigger
/// fallback to genesis replay and are recorded in restoration diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestorationError<MapperError> {
    /// The authoritative ledger is structurally invalid or inaccessible.
    Ledger(LedgerError),
    /// The domain mapper rejected a ledger event.
    Mapping(MapperError),
    /// Applying an already mapped transition failed.
    StateApplication(LedgerTransitionError),
    /// Replay did not terminate at the independently discovered ledger tail.
    ReplayTailMismatch { expected: Lsn, actual: Option<Lsn> },
}

/// Reasons a snapshot candidate can be rejected.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    Malformed,
    UnsupportedVersion,
    ChecksumMismatch,
    PayloadLengthMismatch,
    FilenameMismatch,
    DecodeFailed,
    RootMismatch,
    BeyondTail,
}

/// A snapshot rejected during discovery or coordinator validation.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RejectedSnapshot {
    pub lsn: Option<Lsn>,
    pub reason: RejectionReason,
}

/// Reason the coordinator used genesis replay instead of a snapshot.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FallbackReason {
    NoSnapshotsFound,
    NoValidSnapshotsFound,
    SnapshotDecodeFailed { lsn: Lsn },
    StateRootMismatch { lsn: Lsn },
    SnapshotBeyondTail { lsn: Lsn, tail: Lsn },
}

impl<MapperError: fmt::Display> fmt::Display for RestorationError<MapperError> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger(error) => write!(f, "ledger error: {error}"),
            Self::Mapping(error) => write!(f, "mapper error: {error}"),
            Self::StateApplication(error) => {
                write!(f, "state application error: {error:?}")
            }
            Self::ReplayTailMismatch { expected, actual } => match actual {
                Some(actual) => write!(
                    f,
                    "replay tail mismatch: expected {}, actual {}",
                    expected.get(),
                    actual.get()
                ),
                None => write!(
                    f,
                    "replay tail mismatch: expected {}, actual none",
                    expected.get()
                ),
            },
        }
    }
}

impl<MapperError> std::error::Error for RestorationError<MapperError> where
    MapperError: std::error::Error + 'static
{
}

pub type RestorationResult<T, MapperError> = Result<T, RestorationError<MapperError>>;
