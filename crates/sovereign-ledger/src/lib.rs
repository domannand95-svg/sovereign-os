//! The `sovereign-ledger` crate implements the absolute chronological sequence
//! substrate for Sovereign OS.

pub mod append;
pub mod checksum;
pub mod config;
pub mod domain_integration;
pub mod error;
pub mod lsn;
pub mod record;
pub mod replay;
pub mod segment;
pub mod snapshot;

pub use append::LedgerAppendEngine;
pub use checksum::crc32c;
pub use config::LedgerConfig;
pub use error::LedgerError;
pub use lsn::Lsn;
pub use record::{
    EventRecord, EventType, EVENT_TYPE_OFFSET, LSN_OFFSET, PAYLOAD_LEN_OFFSET, PAYLOAD_OFFSET,
    RECORD_CHECKSUM_LEN, RECORD_HEADER_LEN,
};
pub use replay::ReplayIterator;
pub use segment::LedgerSegment;

pub use snapshot::{
    LedgerSnapshotManager, SnapshotHeader, SNAPSHOT_CHECKSUM_LEN, SNAPSHOT_HEADER_LEN,
};

pub use error::{
    FallbackReason, RejectedSnapshot, RejectionReason, RestorationError, RestorationResult,
};

pub mod state_root;

pub use state_root::{compute_state_root, compute_state_root_from_encoded, DOMAIN_SEPARATOR};
