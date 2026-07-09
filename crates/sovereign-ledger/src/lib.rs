//! The `sovereign-ledger` crate implements the absolute chronological sequence
//! substrate for Sovereign OS. It guarantees absolute, deterministic persistence,
//! linear sequence ordering, and out-of-band crash recovery.

pub mod checksum;
pub mod config;
pub mod error;
pub mod lsn;
pub mod record;
pub mod segment;

pub use checksum::crc32c;
pub use config::LedgerConfig;
pub use error::LedgerError;
pub use lsn::Lsn;
pub use record::{
    EventRecord, EventType, EVENT_TYPE_OFFSET, LSN_OFFSET, PAYLOAD_LEN_OFFSET, PAYLOAD_OFFSET,
    RECORD_CHECKSUM_LEN, RECORD_HEADER_LEN,
};
pub use segment::LedgerSegment;
