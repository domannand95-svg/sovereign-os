//! Boundary Tests for ADAM-012-C
//!
//! Validates atomic transaction processing, commit, and rollback semantics (C012-001..C012-004).

use beta001_harness::dispatch::{
    DeterministicExecutionContext, DeterministicWorker, ExecutionReservationStore,
    TransactionOrchestrator, WorkerError,
};
use beta001_harness::service_contract::{ExecutionId, ProposedOperation, Sha256Digest};
use beta001_harness::state::{StateMutation, StateTree};

struct ConfigWorker;
impl DeterministicWorker for ConfigWorker {
    fn execute(
        &self,
        ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        let execution_str = ctx.execution_id.as_str();
        Ok(vec![
            StateMutation::put(format!("log:{}", execution_str).into_bytes(), b"executing"),
            StateMutation::put(b"global:status", b"active"),
        ])
    }
}

struct FailingWorker;
impl DeterministicWorker for FailingWorker {
    fn execute(
        &self,
        _ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        Err(WorkerError::ExecutionFailure(
            "Controlled runtime failure".to_string(),
        ))
    }
}

struct PanickingWorker;
impl DeterministicWorker for PanickingWorker {
    fn execute(
        &self,
        _ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        panic!("Uncaught index out of bounds");
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
            "d3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        )
        .unwrap(),
        expected_state_root: current_root.to_string(),
        expected_revision: current_rev,
        deterministic_seed: [0u8; 32],
        logical_sequence_tick: 1,
        granted_scope: ProposedOperation::RequestReview,
    }
}

#[test]
fn test_c012_001_and_003_successful_worker_commits_atomically() {
    let mut tree = StateTree::new();
    let store = ExecutionReservationStore::new();
    store.reserve("exe_trans_001").unwrap();

    let ctx = create_mock_context("exe_trans_001", &tree.compute_state_root(), tree.revision());
    let worker = ConfigWorker;

    let res = TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store);

    assert!(res.is_ok());
    assert_eq!(tree.revision(), 1);
    assert_eq!(
        tree.get(b"log:exe_trans_001"),
        Some(b"executing".as_slice())
    );
    assert_eq!(tree.get(b"global:status"), Some(b"active".as_slice()));
}

#[test]
fn test_c012_002_worker_error_rolls_back_atomically() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"base", b"root")]);
    let root_before = tree.compute_state_root();

    let store = ExecutionReservationStore::new();
    store.reserve("exe_trans_002").unwrap();

    let ctx = create_mock_context("exe_trans_002", &root_before, tree.revision());
    let worker = FailingWorker;

    let res = TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store);

    assert!(res.is_err());
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.compute_state_root(), root_before);
    assert_eq!(tree.get(b"base"), Some(b"root".as_slice()));
}

#[test]
fn test_c012_004_worker_panic_rolls_back_atomically() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"base", b"root")]);
    let root_before = tree.compute_state_root();

    let store = ExecutionReservationStore::new();
    store.reserve("exe_trans_003").unwrap();

    let ctx = create_mock_context("exe_trans_003", &root_before, tree.revision());
    let worker = PanickingWorker;

    let res = TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store);

    assert!(res.is_err());
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.compute_state_root(), root_before);
    assert_eq!(tree.get(b"base"), Some(b"root".as_slice()));
}
