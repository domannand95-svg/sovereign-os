//! ADAM-012-D: Idempotent Execution Receipt Store & Anti-Replay Journal
//!
//! Enforces single-execution invariants across all claimed execution IDs,
//! caching terminal receipts to guarantee idempotent replay responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

use super::dispatcher::{DispatchError, ReservationState};
use crate::service_contract::{ExecutionId, Sha256Digest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalExecutionStatus {
    Committed,
    RolledBack { reason: String },
}

/// Canonical receipt recording the outcome of a dispatched transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub execution_id: ExecutionId,
    pub package_digest: Sha256Digest,
    pub initial_state_root: String,
    pub final_state_root: String,
    pub initial_revision: u64,
    pub final_revision: u64,
    pub delta_digest: Option<String>,
    pub status: TerminalExecutionStatus,
    pub sequence_tick: u64,
}

/// Unified journal managing in-flight reservations and immutable terminal receipts.
#[derive(Debug, Default)]
pub struct ExecutionReceiptStore {
    reservations: Mutex<HashMap<String, ReservationState>>,
    receipts: Mutex<HashMap<String, ExecutionReceipt>>,
}

impl ExecutionReceiptStore {
    pub fn new() -> Self {
        Self {
            reservations: Mutex::new(HashMap::new()),
            receipts: Mutex::new(HashMap::new()),
        }
    }

    /// Checks if a terminal receipt already exists for this execution_id.
    pub fn get_receipt(&self, execution_id: &str) -> Option<ExecutionReceipt> {
        let receipts = self.receipts.lock().unwrap();
        receipts.get(execution_id).cloned()
    }

    /// Atomically attempts to reserve dispatch (CLAIMED -> DISPATCH_RESERVED).
    /// If already reserved in-flight or completed, fails closed.
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

    /// Commits an immutable terminal receipt and transitions reservation status.
    pub fn record_terminal_receipt(&self, receipt: ExecutionReceipt) {
        let exec_str = receipt.execution_id.as_str().to_string();
        let target_state = match receipt.status {
            TerminalExecutionStatus::Committed => ReservationState::Committed,
            TerminalExecutionStatus::RolledBack { .. } => ReservationState::RolledBack,
        };

        let mut reservations = self.reservations.lock().unwrap();
        let mut receipts = self.receipts.lock().unwrap();

        reservations.insert(exec_str.clone(), target_state);
        receipts.insert(exec_str, receipt);
    }
}
