//! Legacy runtime audit projection compatibility boundary.
//!
//! This module remains temporarily available during BETA-016 migration.
//! The production implementation now lives in `adapters::audit`.

pub use crate::adapters::audit::{project_execution_receipt, ProjectionError};
