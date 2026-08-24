//! ADAM-012-C / 012-E: Transaction Orchestrator & Transition Receipt Emitter

use super::context::DeterministicExecutionContext;
use super::dispatcher::{DeterministicDispatcher, DispatchError};
use super::receipt_store::{ExecutionReceiptStore, TerminalExecutionStatus};
use super::worker::DeterministicWorker;
use crate::state::{
    compute_delta_digest, compute_transition_root, StateJournal, StateTransitionReceipt, StateTree,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Executed(StateTransitionReceipt),
    CachedReceipt(StateTransitionReceipt),
}

pub struct TransactionOrchestrator;

impl TransactionOrchestrator {
    /// Executes a transaction within the deterministic boundary and emits a verified StateTransitionReceipt.
    pub fn execute_transaction<W: DeterministicWorker>(
        worker: &W,
        ctx: &DeterministicExecutionContext,
        tree: &mut StateTree,
        store: &ExecutionReceiptStore,
    ) -> Result<ExecutionOutcome, DispatchError> {
        let exec_id_str = ctx.execution_id.as_str();

        // 1. Check for cached terminal receipt (Anti-Replay / Idempotency)
        if let Some(cached) = store.get_receipt(exec_id_str) {
            return Ok(ExecutionOutcome::CachedReceipt(cached));
        }

        let initial_root = ctx.expected_state_root.clone();
        let initial_rev = ctx.expected_revision;
        let prev_transition_root = store.current_transition_root();
        let mut journal = StateJournal::new();

        // 2. Sandboxed Worker Execution
        match DeterministicDispatcher::dispatch_sandboxed(worker, ctx, tree) {
            Ok(mutations) => {
                if let Err(e) = journal.stage_batch(mutations) {
                    journal.discard();
                    let delta_digest = compute_delta_digest(&[]);
                    let transition_root = compute_transition_root(
                        &prev_transition_root,
                        &initial_root,
                        &initial_root,
                        ctx.package_digest.as_str(),
                        exec_id_str,
                        &delta_digest,
                    );
                    let receipt = StateTransitionReceipt {
                        transition_root,
                        previous_transition_root: prev_transition_root,
                        initial_state_root: initial_root.clone(),
                        final_state_root: initial_root,
                        initial_revision: initial_rev,
                        final_revision: initial_rev,
                        execution_id: ctx.execution_id.clone(),
                        package_digest: ctx.package_digest.clone(),
                        delta_digest,
                        status: TerminalExecutionStatus::RolledBack {
                            reason: format!("Journal staging failed: {}", e),
                        },
                        sequence_tick: ctx.logical_sequence_tick,
                    };
                    store.record_terminal_receipt(receipt);
                    return Err(DispatchError::WorkerFailed(
                        super::worker::WorkerError::ExecutionFailure(format!(
                            "Journal staging failed: {}",
                            e
                        )),
                    ));
                }

                // 3. Prepare and apply transaction
                let delta_digest = match journal.prepare() {
                    Ok((_, d)) => d,
                    Err(e) => {
                        journal.discard();
                        let empty_delta = compute_delta_digest(&[]);
                        let transition_root = compute_transition_root(
                            &prev_transition_root,
                            &initial_root,
                            &initial_root,
                            ctx.package_digest.as_str(),
                            exec_id_str,
                            &empty_delta,
                        );
                        let receipt = StateTransitionReceipt {
                            transition_root,
                            previous_transition_root: prev_transition_root,
                            initial_state_root: initial_root.clone(),
                            final_state_root: initial_root,
                            initial_revision: initial_rev,
                            final_revision: initial_rev,
                            execution_id: ctx.execution_id.clone(),
                            package_digest: ctx.package_digest.clone(),
                            delta_digest: empty_delta,
                            status: TerminalExecutionStatus::RolledBack {
                                reason: format!("Journal prepare failed: {}", e),
                            },
                            sequence_tick: ctx.logical_sequence_tick,
                        };
                        store.record_terminal_receipt(receipt);
                        return Err(DispatchError::WorkerFailed(
                            super::worker::WorkerError::ExecutionFailure(format!(
                                "Journal prepare failed: {}",
                                e
                            )),
                        ));
                    }
                };

                match journal.apply(tree) {
                    Ok(_) => {
                        let final_root = tree.compute_state_root();
                        let final_rev = tree.revision();
                        let transition_root = compute_transition_root(
                            &prev_transition_root,
                            &initial_root,
                            &final_root,
                            ctx.package_digest.as_str(),
                            exec_id_str,
                            &delta_digest,
                        );
                        let receipt = StateTransitionReceipt {
                            transition_root,
                            previous_transition_root: prev_transition_root,
                            initial_state_root: initial_root,
                            final_state_root: final_root,
                            initial_revision: initial_rev,
                            final_revision: final_rev,
                            execution_id: ctx.execution_id.clone(),
                            package_digest: ctx.package_digest.clone(),
                            delta_digest,
                            status: TerminalExecutionStatus::Committed,
                            sequence_tick: ctx.logical_sequence_tick,
                        };
                        store.record_terminal_receipt(receipt.clone());
                        Ok(ExecutionOutcome::Executed(receipt))
                    }
                    Err(e) => {
                        journal.discard();
                        let transition_root = compute_transition_root(
                            &prev_transition_root,
                            &initial_root,
                            &initial_root,
                            ctx.package_digest.as_str(),
                            exec_id_str,
                            &delta_digest,
                        );
                        let receipt = StateTransitionReceipt {
                            transition_root,
                            previous_transition_root: prev_transition_root,
                            initial_state_root: initial_root.clone(),
                            final_state_root: initial_root,
                            initial_revision: initial_rev,
                            final_revision: initial_rev,
                            execution_id: ctx.execution_id.clone(),
                            package_digest: ctx.package_digest.clone(),
                            delta_digest,
                            status: TerminalExecutionStatus::RolledBack {
                                reason: format!("Journal apply failed: {}", e),
                            },
                            sequence_tick: ctx.logical_sequence_tick,
                        };
                        store.record_terminal_receipt(receipt);
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
                journal.discard();
                let delta_digest = compute_delta_digest(&[]);
                let transition_root = compute_transition_root(
                    &prev_transition_root,
                    &initial_root,
                    &initial_root,
                    ctx.package_digest.as_str(),
                    exec_id_str,
                    &delta_digest,
                );
                let receipt = StateTransitionReceipt {
                    transition_root,
                    previous_transition_root: prev_transition_root,
                    initial_state_root: initial_root.clone(),
                    final_state_root: initial_root,
                    initial_revision: initial_rev,
                    final_revision: initial_rev,
                    execution_id: ctx.execution_id.clone(),
                    package_digest: ctx.package_digest.clone(),
                    delta_digest,
                    status: TerminalExecutionStatus::RolledBack {
                        reason: format!("{:?}", e),
                    },
                    sequence_tick: ctx.logical_sequence_tick,
                };
                store.record_terminal_receipt(receipt);
                Err(e)
            }
        }
    }
}
