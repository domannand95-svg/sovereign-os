//! Sovereign OS governed agent runtime.
//!
//! This crate manages agent lifecycle state, observation intake,
//! proposal formation, capability negotiation, and execution attempts.
//!
//! It does not grant authority.
//! It does not bypass policy.
//! It does not mutate audit history.

pub mod identity;
pub mod observation;
pub mod proposal;
pub mod capability;
pub mod execution;
pub mod replay;
mod encoding;

pub use identity::*;
pub use observation::*;
pub use proposal::*;
pub use capability::*;
pub use execution::*;
pub use replay::*;
