//! Sovereign OS governed agent runtime.
//!
//! This crate manages agent lifecycle state, observation intake,
//! proposal formation, capability negotiation, and execution attempts.
//!
//! It does not grant authority.
//! It does not bypass policy.
//! It does not mutate audit history.

pub mod capability;
mod encoding;
pub mod execution;
pub mod identity;
pub mod observation;
pub mod proposal;
pub mod replay;

pub use capability::*;
pub use execution::*;
pub use identity::*;
pub use observation::*;
pub use proposal::*;
pub use replay::*;
