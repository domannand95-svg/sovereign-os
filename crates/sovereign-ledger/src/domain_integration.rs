//! Integration boundary between `sovereign-ledger` and
//! `sovereign-core-asm`.
//!
//! This module owns ledger-specific orchestration policy while preserving
//! the one-way dependency direction:
//!
//! ```text
//! sovereign-core-asm
//!         ^
//!         |
//! sovereign-ledger
//! ```
//!
//! Runtime adapters and event-to-state mappings are intentionally deferred
//! until the public `EventRecord` integration contract is fully specified.
