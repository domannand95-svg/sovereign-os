//! ADAM-012-B: Deterministic Dispatcher & Reservation Gate
//!
//! Validates service evidence packages, enforces lifecycle state prerequisites,
//! and executes atomic CAS dispatch reservations.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::service::evidence::{EvidenceClosureError, ServiceEvidencePackage};
use crate::service_contract::{ClaimState, DispatchState, OutcomeState};
use crate::state::StateTree;

use super::context::DeterministicExecutionContext;
use super::worker::{invoke_sandboxed_worker, DeterministicWorker, WorkerError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationState {
    DispatchReserved,
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    EvidenceVerificationFailed(String),
    InvalidPreDispatchState(String),
    StateRootMismatch { expected: String, actual: String },
    RevisionMismatch { expected: u64, actual: u64 },
    DispatchAlreadyReserved(String),
    TerminalStateAlreadyExists(String),
    WorkerFailed(WorkerError),
}

impl From<EvidenceClosureError> for DispatchError {
    fn from(err: EvidenceClosureError) -> Self {
        Self::EvidenceVerificationFailed(format!("{:?}", err))
    }
}

impl From<WorkerError> for DispatchError {
    fn from(err: WorkerError) -> Self {
        Self::WorkerFailed(err)
    }
}

/// In-memory execution journal enforcing single-reservation CAS semantics.
#[derive(Debug, Default)]
pub struct ExecutionReservationStore {
    reservations: Mutex<HashMap<String, ReservationState>>,
}

impl ExecutionReservationStore {
    pub fn new() -> Self {
        Self {
            reservations: Mutex::new(HashMap::new()),
        }
    }

    /// Atomically attempts to reserve an execution_id (CLAIMED -> DISPATCH_RESERVED).
    pub fn reserve(&self, execution_id: &str) -> Result<(), DispatchError> {
        let mut guard = self.reservations.lock().unwrap();
        if let Some(state) = guard.get(execution_id) {
            return match state {
                ReservationState::DispatchReserved => Err(DispatchError::DispatchAlreadyReserved(
                    execution_id.to_string(),
                )),
                ReservationState::Committed | ReservationState::RolledBack => Err(
                    DispatchError::TerminalStateAlreadyExists(execution_id.to_string()),
                ),
            };
        }
        guard.insert(execution_id.to_string(), ReservationState::DispatchReserved);
        Ok(())
    }

    /// Updates reservation state post-execution.
    pub fn transition_to_terminal(&self, execution_id: &str, terminal_state: ReservationState) {
        let mut guard = self.reservations.lock().unwrap();
        guard.insert(execution_id.to_string(), terminal_state);
    }
}

pub struct DeterministicDispatcher;

impl DeterministicDispatcher {
    /// Prepares, verifies, and reserves execution for a ServiceEvidencePackage against current state.
    pub fn prepare_dispatch(
        package: &ServiceEvidencePackage,
        reservation_store: &ExecutionReservationStore,
        current_tree: &StateTree,
        logical_sequence_tick: u64,
    ) -> Result<DeterministicExecutionContext, DispatchError> {
        // 1. Cryptographic chain recomputation (B012-001)
        package
            .verify_chain()
            .map_err(|e| DispatchError::EvidenceVerificationFailed(format!("{:?}", e)))?;

        // 2. Pre-dispatch lifecycle triad check (B012-002)
        let claim_state = package.execution_response.claim_state;
        let dispatch_state = package.execution_response.dispatch_state;
        let outcome_state = package.execution_response.outcome_state;

        if claim_state != ClaimState::ExecutionClaimed
            || dispatch_state != DispatchState::NotDispatched
            || outcome_state != OutcomeState::Pending
        {
            return Err(DispatchError::InvalidPreDispatchState(format!(
                "Invalid pre-dispatch triad: Claim={:?}, Dispatch={:?}, Outcome={:?}",
                claim_state, dispatch_state, outcome_state
            )));
        }

        let exec_id = &package.execution_response.execution_id;

        // 3. Atomic CAS Reservation (B012-004)
        reservation_store.reserve(exec_id.as_str())?;

        // 4. Derive deterministic seed and execution context (B012-005)
        let current_root = current_tree.compute_state_root();
        let current_rev = current_tree.revision();

        let seed = DeterministicExecutionContext::derive_seed(
            &package.package_digest,
            exec_id,
            current_rev,
        );

        Ok(DeterministicExecutionContext {
            execution_id: exec_id.clone(),
            package_digest: package.package_digest.clone(),
            expected_state_root: current_root,
            expected_revision: current_rev,
            deterministic_seed: seed,
            logical_sequence_tick,
            granted_scope: package
                .sanitized_ingress
                .original_request
                .proposed_operation,
        })
    }

    /// Dispatches a sandboxed worker, enforcing panic containment (B012-006, B012-007).
    pub fn dispatch_sandboxed<W: DeterministicWorker>(
        worker: &W,
        ctx: &DeterministicExecutionContext,
        snapshot: &StateTree,
    ) -> Result<Vec<crate::state::StateMutation>, DispatchError> {
        // Verify state root has not drifted prior to worker invocation
        let current_root = snapshot.compute_state_root();
        if current_root != ctx.expected_state_root {
            return Err(DispatchError::StateRootMismatch {
                expected: ctx.expected_state_root.clone(),
                actual: current_root,
            });
        }

        let current_rev = snapshot.revision();
        if current_rev != ctx.expected_revision {
            return Err(DispatchError::RevisionMismatch {
                expected: ctx.expected_revision,
                actual: current_rev,
            });
        }

        // Invoke worker inside panic containment boundary
        let mutations = invoke_sandboxed_worker(worker, ctx, snapshot)?;
        Ok(mutations)
    }
}
