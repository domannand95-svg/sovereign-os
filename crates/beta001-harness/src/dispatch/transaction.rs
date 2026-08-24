//! ADAM-012-C: Transaction Orchestrator & Two-Phase Commit
//!
//! Binds panic-contained worker execution with the CoW transaction journal
//! to provide atomic prepare, commit, and rollback semantics.

use super::context::DeterministicExecutionContext;
use super::dispatcher::{
    DeterministicDispatcher, DispatchError, ExecutionReservationStore, ReservationState,
};
use super::worker::DeterministicWorker;
use crate::state::{StateJournal, StateTree};

pub struct TransactionOrchestrator;

impl TransactionOrchestrator {
    /// Executes a deterministic worker within a transactional boundary.
    ///
    /// 1. Dispatches the sandboxed worker to generate mutations.
    /// 2. Stages mutations in a Copy-on-Write journal.
    /// 3. Upon success, atomically applies the journal (COMMITTED).
    /// 4. Upon failure or panic, discards the journal and rolls back (ROLLED_BACK).
    pub fn execute_transaction<W: DeterministicWorker>(
        worker: &W,
        ctx: &DeterministicExecutionContext,
        tree: &mut StateTree,
        reservation_store: &ExecutionReservationStore,
    ) -> Result<bool, DispatchError> {
        let exec_id = ctx.execution_id.as_str();

        let mut journal = StateJournal::new();

        // 1. Invoke panic-contained worker (B012-006, C012-002)
        match DeterministicDispatcher::dispatch_sandboxed(worker, ctx, tree) {
            Ok(mutations) => {
                // 2. Stage mutations into CoW journal (C012-001)
                if let Err(e) = journal.stage_batch(mutations) {
                    journal.discard();
                    reservation_store.transition_to_terminal(exec_id, ReservationState::RolledBack);
                    return Err(DispatchError::WorkerFailed(
                        super::worker::WorkerError::ExecutionFailure(format!(
                            "Journal staging failed: {}",
                            e
                        )),
                    ));
                }

                // 3. Atomically apply journal to state tree (C012-003)
                match journal.apply(tree) {
                    Ok(changed) => {
                        reservation_store
                            .transition_to_terminal(exec_id, ReservationState::Committed);
                        Ok(changed)
                    }
                    Err(e) => {
                        journal.discard();
                        reservation_store
                            .transition_to_terminal(exec_id, ReservationState::RolledBack);
                        Err(DispatchError::WorkerFailed(
                            super::worker::WorkerError::ExecutionFailure(format!(
                                "Journal apply failed: {}",
                                e
                            )),
                        ))
                    }
                }
            }
            Err(e) => {
                // 4. Rollback on worker failure/panic (C012-004)
                journal.discard();
                reservation_store.transition_to_terminal(exec_id, ReservationState::RolledBack);
                Err(e)
            }
        }
    }
}
