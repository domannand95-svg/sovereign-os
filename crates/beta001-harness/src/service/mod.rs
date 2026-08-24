//! Service Integration Layer for Sovereign OS
//!
//! Exposes governed gateway admission endpoints bridging external transport DTOs
//! to the deterministic governance kernel.

pub mod admission;
pub mod client;

pub mod inference;

pub mod evidence;

pub mod client_http;
