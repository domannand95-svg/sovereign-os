//! Sovereign OS governed agent runtime.
//!
//! This crate manages agent lifecycle state, observation intake,
//! proposal formation, capability negotiation, execution attempts,
//! and verified evidence projection.
//!
//! It does not grant authority.
//! It does not bypass policy.
//! It does not mutate audit history.

pub mod adapters;
pub mod audit_projection;
pub mod capability;
mod encoding;
pub mod execution;
pub mod identity;
pub mod observation;
pub mod proposal;
pub mod replay;

pub use audit_projection::*;
pub use capability::*;
pub use execution::*;
pub use identity::*;
pub use observation::*;
pub use proposal::*;
pub use replay::*;