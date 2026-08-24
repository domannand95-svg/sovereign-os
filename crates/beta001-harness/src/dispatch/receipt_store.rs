//! ADAM-012-D / 012-E: Execution Receipt Store & Causal Lineage Journal

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

use super::dispatcher::{DispatchError, ReservationState};
use crate::state::{compute_genesis_transition_root, StateTransitionReceipt};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalExecutionStatus {
    Committed,
    RolledBack { reason: String },
}

/// Unified journal managing in-flight reservations, lineage roots, and immutable transition receipts.
#[derive(Debug)]
pub struct ExecutionReceiptStore {
    reservations: Mutex<HashMap<String, ReservationState>>,
    receipts: Mutex<HashMap<String, StateTransitionReceipt>>,
    current_transition_root: Mutex<String>,
}

impl Default for ExecutionReceiptStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionReceiptStore {
    pub fn new() -> Self {
        Self {
            reservations: Mutex::new(HashMap::new()),
            receipts: Mutex::new(HashMap::new()),
            current_transition_root: Mutex::new(compute_genesis_transition_root()),
        }
    }

    /// Retrieves the current causal transition root.
    pub fn current_transition_root(&self) -> String {
        self.current_transition_root.lock().unwrap().clone()
    }

    /// Checks if a terminal transition receipt already exists for this execution_id.
    pub fn get_receipt(&self, execution_id: &str) -> Option<StateTransitionReceipt> {
        let receipts = self.receipts.lock().unwrap();
        receipts.get(execution_id).cloned()
    }

    /// Atomically attempts to reserve dispatch (CLAIMED -> DISPATCH_RESERVED).
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

    /// Commits an immutable terminal transition receipt, advances transition root, and transitions reservation status.
    pub fn record_terminal_receipt(&self, receipt: StateTransitionReceipt) {
        let exec_str = receipt.execution_id.as_str().to_string();
        let target_state = match receipt.status {
            TerminalExecutionStatus::Committed => ReservationState::Committed,
            TerminalExecutionStatus::RolledBack { .. } => ReservationState::RolledBack,
        };

        let mut reservations = self.reservations.lock().unwrap();
        let mut receipts = self.receipts.lock().unwrap();
        let mut root_guard = self.current_transition_root.lock().unwrap();

        *root_guard = receipt.transition_root.clone();
        reservations.insert(exec_str.clone(), target_state);
        receipts.insert(exec_str, receipt);
    }
}
