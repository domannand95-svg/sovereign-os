//! Legacy runtime audit projection compatibility boundary.
//!
//! Production implementation now lives in `adapters::audit`.

pub use crate::adapters::audit::{project_execution_receipt, ProjectionError};
