//! ADAM-012-B: Sandboxed Worker Contract & Panic Containment Boundary
//!
//! Enforces pure mutation proposals with strict panic isolation.

use std::panic::{catch_unwind, AssertUnwindSafe, RefUnwindSafe};

use super::context::DeterministicExecutionContext;
use crate::state::{StateMutation, StateTree};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerError {
    ExecutionFailure(String),
    ScopeViolation(String),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutionFailure(msg) => write!(f, "Worker execution failure: {}", msg),
            Self::ScopeViolation(msg) => write!(f, "Worker scope violation: {}", msg),
        }
    }
}

impl std::error::Error for WorkerError {}

/// Trait defining sandboxed, deterministic mutation logic.
pub trait DeterministicWorker: Send + Sync + RefUnwindSafe {
    fn execute(
        &self,
        ctx: &DeterministicExecutionContext,
        snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError>;
}

/// Invokes a worker within a strict panic-containment sandbox.
/// Contained panics are caught and translated into Result::Err without unwinding the caller.
pub fn invoke_sandboxed_worker<W: DeterministicWorker>(
    worker: &W,
    ctx: &DeterministicExecutionContext,
    snapshot: &StateTree,
) -> Result<Vec<StateMutation>, WorkerError> {
    let result = catch_unwind(AssertUnwindSafe(|| worker.execute(ctx, snapshot)));

    match result {
        Ok(exec_result) => exec_result,
        Err(panic_payload) => {
            let reason = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown worker panic".to_string()
            };
            Err(WorkerError::ExecutionFailure(format!(
                "PANIC_CONTAINED: {}",
                reason
            )))
        }
    }
}
