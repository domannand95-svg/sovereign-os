//! The `sovereign-registry` crate implements the content-addressable semantic
//! graph identity substrate for Sovereign OS.

pub mod caid;
pub mod edge;
pub mod encoding;
pub mod error;
pub mod graph;
pub mod identity;
mod identity_resolution;
pub mod node;
pub mod sync;
mod validation;

pub use caid::Caid;
pub use edge::{ObjectClass, RegistryEdge, RelationType};
pub use encoding::{
    deserialize_edge_v2, serialize_edge_v2, CapabilityPayloadV1, ExecutionBudgetV1,
    FilesystemReadScopeV1, FilesystemWriteScopeV1, NetworkScopeV1, OperationCodeV1,
    RegistryGenesisPayloadV1, ResourceConstraintsV1, TargetScopeV1, VersionedRegistryNode,
    CAPABILITY_PAYLOAD_MAX_LEN_V1, CAPABILITY_PAYLOAD_MIN_LEN_V1,
    CAPABILITY_PAYLOAD_SCHEMA_VERSION_V1, REGISTRY_EDGE_MAGIC_V2, REGISTRY_ENCODING_VERSION_V2,
    REGISTRY_NODE_MAGIC_V2,
};
pub use error::RegistryError;
pub use graph::{RegistryBootstrapConfig, RegistryGraph};
pub use identity::{
    IdentityId, IdentityKind, IdentityRecord, LineageRecord, LineageRoot,
    IDENTITY_ENCODING_VERSION, LINEAGE_ENCODING_VERSION, MAX_IDENTITY_DESCRIPTOR_LEN,
    MAX_LINEAGE_PARENTS,
};
pub use identity_resolution::{IdentityResolver, IdentityStateRef, ResolvedIdentity};
pub use node::{RegistryNode, RegistryNodeType};
pub use sync::{RegistryLedgerSync, RegistryWireRecord};
pub use validation::{
    validate_capability_identities, validate_capability_reference, validate_capability_references,
    validate_governed_reference,
};
