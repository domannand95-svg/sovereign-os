//! Service Integration Layer for Sovereign OS
//!
//! Exposes governed gateway admission endpoints bridging external transport DTOs
//! to the deterministic governance kernel.

pub mod admission;
pub mod client;

pub mod inference;

pub mod evidence;

pub mod client_http;

mod intent_normalization;

// Phase A1 deliberately provides no production caller or public import API.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Inactive Phase A1 envelope boundary")
)]
mod profiled_evidence;
