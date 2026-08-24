//! Boundary Tests for ADAM-012-D
//!
//! Validates execution anti-replay, in-flight reservation locking, and idempotent receipt caching (D012-001..D012-005).

use beta001_harness::dispatch::{
    DeterministicExecutionContext, DeterministicWorker, DispatchError, ExecutionOutcome,
    ExecutionReceiptStore, TerminalExecutionStatus, TransactionOrchestrator, WorkerError,
};
use beta001_harness::service_contract::{ExecutionId, ProposedOperation, Sha256Digest};
use beta001_harness::state::{StateMutation, StateTree};

struct CountWorker;
impl DeterministicWorker for CountWorker {
    fn execute(
        &self,
        ctx: &DeterministicExecutionContext,
        snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        let current_count = snapshot
            .get(b"counter")
            .and_then(|b| std::str::from_utf8(b).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let next_count = current_count + 1;
        Ok(vec![
            StateMutation::put(b"counter", next_count.to_string().into_bytes()),
            StateMutation::put(
                format!("audit:{}", ctx.execution_id.as_str()).into_bytes(),
                b"done",
            ),
        ])
    }
}

fn create_mock_context(
    exec_id: &str,
    current_root: &str,
    current_rev: u64,
) -> DeterministicExecutionContext {
    DeterministicExecutionContext {
        execution_id: ExecutionId::new(exec_id.to_string()).unwrap(),
        package_digest: Sha256Digest::new(
            "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string(),
        )
        .unwrap(),
        expected_state_root: current_root.to_string(),
        expected_revision: current_rev,
        deterministic_seed: [1u8; 32],
        logical_sequence_tick: 42,
        granted_scope: ProposedOperation::RequestReview,
    }
}

#[test]
fn test_d012_001_and_003_first_execution_succeeds_and_subsequent_replay_returns_cached_receipt() {
    let mut tree = StateTree::new();
    let store = ExecutionReceiptStore::new();
    let exec_id = "exe_replay_001";

    store.reserve(exec_id).unwrap();
    let ctx = create_mock_context(exec_id, &tree.compute_state_root(), tree.revision());
    let worker = CountWorker;

    // 1. First execution
    let res1 =
        TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store).unwrap();
    let receipt1 = match res1 {
        ExecutionOutcome::Executed(r) => r,
        _ => panic!("Expected first run to execute"),
    };
    assert_eq!(receipt1.status, TerminalExecutionStatus::Committed);
    assert_eq!(tree.get(b"counter"), Some(b"1".as_slice()));
    assert_eq!(tree.revision(), 1);

    // 2. Replay with identical execution_id
    let res2 =
        TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store).unwrap();
    let receipt2 = match res2 {
        ExecutionOutcome::CachedReceipt(r) => r,
        _ => panic!("Expected replay to return cached receipt"),
    };

    // Receipts must be byte/value identical
    assert_eq!(receipt1, receipt2);

    // Tree must NOT have incremented counter (worker did not re-execute)
    assert_eq!(tree.get(b"counter"), Some(b"1".as_slice()));
    assert_eq!(tree.revision(), 1);
}

#[test]
fn test_d012_002_duplicate_in_flight_reservation_fails_closed() {
    let store = ExecutionReceiptStore::new();
    let exec_id = "exe_inflight_001";

    // Initial reservation
    assert!(store.reserve(exec_id).is_ok());

    // Concurrent / Duplicate in-flight reservation attempt
    let res = store.reserve(exec_id);
    assert!(matches!(res, Err(DispatchError::DispatchAlreadyReserved(id)) if id == exec_id));
}

#[test]
fn test_d012_004_and_005_replay_after_failure_returns_cached_failure_without_mutating_state() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"base", b"stable_data")]);
    let root_before = tree.compute_state_root();

    let store = ExecutionReceiptStore::new();
    let exec_id = "exe_fail_replay_001";
    store.reserve(exec_id).unwrap();

    let ctx = create_mock_context(exec_id, &root_before, tree.revision());

    struct BuggyWorker;
    impl DeterministicWorker for BuggyWorker {
        fn execute(
            &self,
            _ctx: &DeterministicExecutionContext,
            _snapshot: &StateTree,
        ) -> Result<Vec<StateMutation>, WorkerError> {
            Err(WorkerError::ExecutionFailure(
                "Deterministic database constraint violation".to_string(),
            ))
        }
    }

    // 1. First execution fails and rolls back
    let res1 = TransactionOrchestrator::execute_transaction(&BuggyWorker, &ctx, &mut tree, &store);
    assert!(res1.is_err());
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.compute_state_root(), root_before);

    // 2. Replay returns cached rolled-back outcome without re-running or mutating
    let res2 = TransactionOrchestrator::execute_transaction(&BuggyWorker, &ctx, &mut tree, &store)
        .unwrap();
    match res2 {
        ExecutionOutcome::CachedReceipt(receipt) => {
            assert!(matches!(
                receipt.status,
                TerminalExecutionStatus::RolledBack { .. }
            ));
            assert_eq!(receipt.initial_state_root, root_before);
            assert_eq!(receipt.final_state_root, root_before);
        }
        _ => panic!("Expected cached receipt on replay of failed transaction"),
    }

    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.compute_state_root(), root_before);
}
