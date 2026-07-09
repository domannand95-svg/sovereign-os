//! The `sovereign-registry` crate implements the content-addressable semantic
//! graph identity substrate for Sovereign OS.

pub mod caid;
pub mod error;
pub mod graph;
pub mod node;

pub use caid::Caid;
pub use error::RegistryError;
pub use graph::RegistryGraph;
pub use node::{RegistryNode, RegistryNodeType};
