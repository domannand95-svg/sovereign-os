//! Boundary Tests for ADAM-012-E
//!
//! Validates transition roots, causal lineage, and transition receipts (E012-001..E012-005).

use beta001_harness::dispatch::{
    DeterministicExecutionContext, DeterministicWorker, ExecutionOutcome, ExecutionReceiptStore,
    TerminalExecutionStatus, TransactionOrchestrator, WorkerError,
};
use beta001_harness::service_contract::{ExecutionId, ProposedOperation, Sha256Digest};
use beta001_harness::state::{
    compute_genesis_transition_root, compute_transition_root, StateMutation, StateTree,
    TRANSITION_GENESIS_DOMAIN_TAG,
};

struct MutationWorker {
    key: Vec<u8>,
    val: Vec<u8>,
}

impl DeterministicWorker for MutationWorker {
    fn execute(
        &self,
        _ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        Ok(vec![StateMutation::put(self.key.clone(), self.val.clone())])
    }
}

fn create_mock_context(
    exec_id: &str,
    pkg_digest: &str,
    current_root: &str,
    current_rev: u64,
) -> DeterministicExecutionContext {
    DeterministicExecutionContext {
        execution_id: ExecutionId::new(exec_id.to_string()).unwrap(),
        package_digest: Sha256Digest::new(pkg_digest.to_string()).unwrap(),
        expected_state_root: current_root.to_string(),
        expected_revision: current_rev,
        deterministic_seed: [7u8; 32],
        logical_sequence_tick: 1,
        granted_scope: ProposedOperation::RequestReview,
    }
}

#[test]
fn test_e012_001_genesis_transition_root_stability() {
    let mut expected_hasher = blake3::Hasher::new();
    expected_hasher.update(TRANSITION_GENESIS_DOMAIN_TAG.as_bytes());
    expected_hasher.update(&0u64.to_be_bytes());
    let expected = expected_hasher.finalize().to_hex().to_string();

    assert_eq!(compute_genesis_transition_root(), expected);
}

#[test]
fn test_e012_002_and_005_transition_receipt_cryptographic_self_verification() {
    let mut tree = StateTree::new();
    let store = ExecutionReceiptStore::new();
    let exec_id = "exe_proof_001";
    let pkg_digest = "b1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";

    store.reserve(exec_id).unwrap();
    let ctx = create_mock_context(
        exec_id,
        pkg_digest,
        &tree.compute_state_root(),
        tree.revision(),
    );
    let worker = MutationWorker {
        key: b"config:node_id".to_vec(),
        val: b"node_alpha".to_vec(),
    };

    let outcome =
        TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store).unwrap();
    let receipt = match outcome {
        ExecutionOutcome::Executed(r) => r,
        _ => panic!("Expected executed receipt"),
    };

    assert_eq!(receipt.status, TerminalExecutionStatus::Committed);
    assert_eq!(
        receipt.previous_transition_root,
        compute_genesis_transition_root()
    );
    assert!(receipt.verify_integrity());
}

#[test]
fn test_e012_003_divergent_package_digest_changes_transition_root() {
    let prev_root = compute_genesis_transition_root();
    let init_state = "state_root_initial";
    let final_state = "state_root_final";
    let delta = "delta_digest_1";

    let t_root1 = compute_transition_root(
        &prev_root,
        init_state,
        final_state,
        "1111111111111111111111111111111111111111111111111111111111111111",
        "exe_trans_1",
        delta,
    );

    let t_root2 = compute_transition_root(
        &prev_root,
        init_state,
        final_state,
        "2222222222222222222222222222222222222222222222222222222222222222",
        "exe_trans_1",
        delta,
    );

    assert_ne!(t_root1, t_root2);
}

#[test]
fn test_e012_004_identical_state_root_via_different_paths_preserves_distinct_transition_roots() {
    // Both paths reach identical StateRoot { k: "v" } from empty state, but via different executions/packages
    let mut tree1 = StateTree::new();
    let store1 = ExecutionReceiptStore::new();
    let exec1 = "exe_path_a_001";
    store1.reserve(exec1).unwrap();
    let ctx1 = create_mock_context(
        exec1,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        &tree1.compute_state_root(),
        0,
    );
    let worker1 = MutationWorker {
        key: b"k".to_vec(),
        val: b"v".to_vec(),
    };

    let r1 =
        match TransactionOrchestrator::execute_transaction(&worker1, &ctx1, &mut tree1, &store1)
            .unwrap()
        {
            ExecutionOutcome::Executed(r) => r,
            _ => panic!(),
        };

    let mut tree2 = StateTree::new();
    let store2 = ExecutionReceiptStore::new();
    let exec2 = "exe_path_b_002";
    store2.reserve(exec2).unwrap();
    let ctx2 = create_mock_context(
        exec2,
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        &tree2.compute_state_root(),
        0,
    );
    let worker2 = MutationWorker {
        key: b"k".to_vec(),
        val: b"v".to_vec(),
    };

    let r2 =
        match TransactionOrchestrator::execute_transaction(&worker2, &ctx2, &mut tree2, &store2)
            .unwrap()
        {
            ExecutionOutcome::Executed(r) => r,
            _ => panic!(),
        };

    // State roots are identical (content commitment)
    assert_eq!(tree1.compute_state_root(), tree2.compute_state_root());
    assert_eq!(r1.final_state_root, r2.final_state_root);

    // Transition roots are distinct (causal provenance commitment)
    assert_ne!(r1.transition_root, r2.transition_root);
}
