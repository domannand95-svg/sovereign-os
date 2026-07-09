//! Sovereign Ledger
//!
//! Deterministic append-only event ledger primitives.
//!
//! This crate implements the foundational chronological substrate defined by
//! SPEC-EVT-001.

pub mod error;
pub mod lsn;

pub use error::LedgerError;
pub use lsn::Lsn;
