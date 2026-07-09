//! The `sovereign-registry` crate implements the content-addressable semantic
//! graph identity substrate for Sovereign OS.

pub mod caid;
pub mod error;

pub use caid::Caid;
pub use error::RegistryError;
