//! The `sovereign-registry` crate implements the content-addressable semantic
//! graph identity substrate for Sovereign OS.

pub mod caid;
pub mod edge;
pub mod encoding;
pub mod error;
pub mod graph;
pub mod identity;
pub mod node;
pub mod sync;

pub use caid::Caid;
pub use edge::{ObjectClass, RegistryEdge, RelationType};
pub use encoding::{
    deserialize_edge_v2, serialize_edge_v2, VersionedRegistryNode, REGISTRY_EDGE_MAGIC_V2,
    REGISTRY_ENCODING_VERSION_V2, REGISTRY_NODE_MAGIC_V2,
};
pub use error::RegistryError;
pub use graph::RegistryGraph;
pub use identity::{
    IdentityId, IdentityKind, IdentityRecord, LineageRecord, LineageRoot,
    IDENTITY_ENCODING_VERSION, LINEAGE_ENCODING_VERSION, MAX_IDENTITY_DESCRIPTOR_LEN,
    MAX_LINEAGE_PARENTS,
};
pub use node::{RegistryNode, RegistryNodeType};
pub use sync::{RegistryLedgerSync, RegistryWireRecord};
