//! Sovereign Ledger
//!
//! Deterministic append-only event ledger primitives.
//!
//! This crate implements the foundational chronological substrate defined by
//! SPEC-EVT-001.

pub mod checksum;
pub mod error;
pub mod lsn;
pub mod record;

pub use checksum::crc32c;
pub use error::LedgerError;
pub use lsn::Lsn;
pub use record::{
    EventRecord, EventType, EVENT_TYPE_OFFSET, LSN_OFFSET, PAYLOAD_LEN_OFFSET, PAYLOAD_OFFSET,
    RECORD_CHECKSUM_LEN, RECORD_HEADER_LEN,
};
