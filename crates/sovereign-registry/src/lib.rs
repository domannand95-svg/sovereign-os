//! The `sovereign-registry` crate implements the content-addressable semantic
//! graph identity substrate for Sovereign OS.

pub mod caid;
pub mod error;
pub mod graph;
pub mod identity;
pub mod node;
pub mod sync;

pub use caid::Caid;
pub use error::RegistryError;
pub use graph::RegistryGraph;
pub use identity::{
    IdentityId, IdentityKind, IdentityRecord, LineageRecord, LineageRoot,
    IDENTITY_ENCODING_VERSION, LINEAGE_ENCODING_VERSION, MAX_IDENTITY_DESCRIPTOR_LEN,
    MAX_LINEAGE_PARENTS,
};
pub use node::{RegistryNode, RegistryNodeType};
pub use sync::RegistryLedgerSync;
