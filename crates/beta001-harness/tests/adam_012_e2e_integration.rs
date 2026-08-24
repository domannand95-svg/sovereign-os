//! End-to-End Integration Suite for ADAM-012
//!
//! Validates the complete pipeline: admission gates, evidence packaging,
//! deterministic dispatch, transaction isolation, transition lineage, and authority invariance.

use chrono::Utc;

use beta001_harness::approval::{ApprovalLevel, ApprovalReceipt};
use beta001_harness::dispatch::{
    DeterministicDispatcher, DeterministicExecutionContext, DeterministicWorker, ExecutionOutcome,
    ExecutionReceiptStore, TerminalExecutionStatus, TransactionOrchestrator, WorkerError,
};
use beta001_harness::service::admission::{ExecutionAdmissionGate, ProposalAdmissionGate};
use beta001_harness::service::client::{AuthenticatedClientContext, ClientSanitizer};
use beta001_harness::service::evidence::ServiceEvidencePackage;
use beta001_harness::service::inference::{InferenceGatewayAdapter, RecordedTransportOutcome};
use beta001_harness::service_contract::{
    ApprovalReceiptId, AuthorityDeltaZero, ExecutionRequest, ProposalRequest, ProposedOperation,
    SchemaVersionV1, SessionId, Sha256Digest, UserId,
};
use beta001_harness::state::{compute_genesis_transition_root, StateMutation, StateTree};

fn build_governed_evidence_package(
    user_name: &str,
    session_str: &str,
    intent_str: &str,
) -> ServiceEvidencePackage {
    let now = Utc::now();
    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new(user_name.to_string()).unwrap(),
        authenticated_session_id: SessionId::new(session_str.to_string()).unwrap(),
    };

    let raw_req = ProposalRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        intent: intent_str.to_string(),
        proposed_operation: ProposedOperation::RequestReview,
        source_evidence_references: vec![Sha256Digest::new(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        )
        .unwrap()],
        timestamp: now,
    };

    let sanitized =
        ClientSanitizer::sanitize_proposal_request(raw_req.clone(), &auth_ctx, now).unwrap();
    let prop_gate = ProposalAdmissionGate::new();
    let prop_res = prop_gate.admit_proposal(&raw_req).unwrap();
    let (_, risk_ctx) = prop_gate
        .get_admitted(prop_res.proposal_id.as_str())
        .unwrap();

    let receipt = ApprovalReceipt {
        receipt_id: format!("apr_{}", prop_res.proposal_id.as_str()),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_quorum_leader".to_string(),
        approval_level: ApprovalLevel::Peer,
        timestamp: "2026-08-24T03:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "crypto_sig_adam_012_e2e".to_string(),
    };

    let exec_gate = ExecutionAdmissionGate::new();
    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new(receipt.receipt_id.clone()).unwrap(),
        caller_identity: "orchestrator_daemon".to_string(),
        signature: "caller_sig_verified".to_string(),
        timestamp: now,
    };
    let exec_res = exec_gate
        .admit_execution(&exec_req, &prop_gate, &receipt)
        .unwrap();

    let inf = InferenceGatewayAdapter::record_transport(
        auth_ctx.authenticated_session_id.clone(),
        "gemini-2.5-pro",
        "boundary_adapter_v1",
        "Deterministic evaluation of state mutation preconditions",
        RecordedTransportOutcome::Success {
            raw_model_output: "PRECONDITIONS_VALIDATED".to_string(),
        },
        now,
    )
    .unwrap();

    ServiceEvidencePackage::build(
        sanitized,
        prop_res,
        risk_ctx,
        receipt,
        exec_res,
        vec![inf],
        now,
    )
    .unwrap()
}

struct LedgerWorker {
    account: String,
    balance: u64,
}

impl DeterministicWorker for LedgerWorker {
    fn execute(
        &self,
        ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        let acct_key = format!("account:{}", self.account).into_bytes();
        let audit_key = format!("audit:{}", ctx.execution_id.as_str()).into_bytes();

        Ok(vec![
            StateMutation::put(acct_key, self.balance.to_string().into_bytes()),
            StateMutation::put(audit_key, b"COMMITTED".to_vec()),
        ])
    }
}

struct CorruptWorker;
impl DeterministicWorker for CorruptWorker {
    fn execute(
        &self,
        _ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        panic!("Memory isolation boundary violation simulation");
    }
}

#[test]
fn test_f012_001_end_to_end_governed_state_execution_lifecycle() {
    let mut tree = StateTree::new();
    let store = ExecutionReceiptStore::new();

    // 1. Ingress & Admission Pipeline
    let package = build_governed_evidence_package(
        "usr_alice",
        "ses_e2e_001",
        "Initialize node config ledger",
    );
    let exec_id = package.execution_response.execution_id.clone();

    // Verify zero authority expansion at boundary
    assert_eq!(
        package.execution_response.authority_delta,
        AuthorityDeltaZero
    );

    // 2. Prepare Dispatch
    let ctx = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 1).unwrap();
    assert_eq!(ctx.execution_id, exec_id);
    assert_eq!(ctx.expected_revision, 0);

    // 3. Worker Execution and Two-Phase Atomic Apply
    let worker = LedgerWorker {
        account: "alice".to_string(),
        balance: 1000,
    };
    let outcome =
        TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store).unwrap();

    // 4. Verify Receipt and State Evolution
    let receipt = match outcome {
        ExecutionOutcome::Executed(r) => r,
        _ => panic!("Expected newly executed transaction"),
    };

    assert_eq!(receipt.status, TerminalExecutionStatus::Committed);
    assert_eq!(receipt.initial_revision, 0);
    assert_eq!(receipt.final_revision, 1);
    assert_eq!(
        receipt.previous_transition_root,
        compute_genesis_transition_root()
    );
    assert!(receipt.verify_integrity());

    assert_eq!(tree.get(b"account:alice"), Some(b"1000".as_slice()));
    assert_eq!(
        tree.get(format!("audit:{}", exec_id.as_str()).as_bytes()),
        Some(b"COMMITTED".as_slice())
    );
}

#[test]
fn test_f012_002_multi_step_causal_transition_lineage() {
    let mut tree = StateTree::new();
    let store = ExecutionReceiptStore::new();

    // Step 1: Genesis -> T_1
    let pkg1 =
        build_governed_evidence_package("usr_system", "ses_e2e_step1", "Set config:epoch to 1");
    let ctx1 = DeterministicDispatcher::prepare_dispatch(&pkg1, &store, &tree, 1).unwrap();
    let worker1 = LedgerWorker {
        account: "system_epoch".to_string(),
        balance: 1,
    };
    let r1 = match TransactionOrchestrator::execute_transaction(&worker1, &ctx1, &mut tree, &store)
        .unwrap()
    {
        ExecutionOutcome::Executed(r) => r,
        _ => panic!(),
    };

    assert_eq!(
        r1.previous_transition_root,
        compute_genesis_transition_root()
    );
    assert_eq!(tree.revision(), 1);
    assert_eq!(store.current_transition_root(), r1.transition_root);

    // Step 2: T_1 -> T_2
    let pkg2 =
        build_governed_evidence_package("usr_system", "ses_e2e_step2", "Set config:epoch to 2");
    let ctx2 = DeterministicDispatcher::prepare_dispatch(&pkg2, &store, &tree, 2).unwrap();
    let worker2 = LedgerWorker {
        account: "system_epoch".to_string(),
        balance: 2,
    };
    let r2 = match TransactionOrchestrator::execute_transaction(&worker2, &ctx2, &mut tree, &store)
        .unwrap()
    {
        ExecutionOutcome::Executed(r) => r,
        _ => panic!(),
    };

    assert_eq!(r2.previous_transition_root, r1.transition_root);
    assert_eq!(r2.initial_state_root, r1.final_state_root);
    assert_eq!(tree.revision(), 2);
    assert_eq!(store.current_transition_root(), r2.transition_root);

    // Verify cryptographic integrity of both transition receipts
    assert!(r1.verify_integrity());
    assert!(r2.verify_integrity());
}

#[test]
fn test_f012_003_replay_idempotency_in_governed_pipeline() {
    let mut tree = StateTree::new();
    let store = ExecutionReceiptStore::new();

    let package =
        build_governed_evidence_package("usr_bob", "ses_e2e_replay", "Credit Bob 500 units");
    let ctx = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 10).unwrap();
    let worker = LedgerWorker {
        account: "bob".to_string(),
        balance: 500,
    };

    // Initial Execution
    let res1 =
        TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store).unwrap();
    let receipt1 = match res1 {
        ExecutionOutcome::Executed(r) => r,
        _ => panic!(),
    };

    assert_eq!(tree.get(b"account:bob"), Some(b"500".as_slice()));
    assert_eq!(tree.revision(), 1);

    // Replay Execution
    let res2 =
        TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store).unwrap();
    let receipt2 = match res2 {
        ExecutionOutcome::CachedReceipt(r) => r,
        _ => panic!("Expected cached receipt on replay"),
    };

    assert_eq!(receipt1, receipt2);
    assert_eq!(tree.revision(), 1);
}

#[test]
fn test_f012_004_worker_panic_fail_closed_and_rollback_integrity() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"account:vital", b"1000000")]);
    let initial_root = tree.compute_state_root();
    let initial_rev = tree.revision();

    let store = ExecutionReceiptStore::new();
    let package =
        build_governed_evidence_package("usr_attacker", "ses_e2e_panic", "Malicious panic payload");
    let ctx = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 99).unwrap();

    let res = TransactionOrchestrator::execute_transaction(&CorruptWorker, &ctx, &mut tree, &store);
    assert!(res.is_err());

    // Verify tree remained completely pristine
    assert_eq!(tree.revision(), initial_rev);
    assert_eq!(tree.compute_state_root(), initial_root);
    assert_eq!(tree.get(b"account:vital"), Some(b"1000000".as_slice()));

    // Verify terminal receipt was recorded as RolledBack
    let receipt = store
        .get_receipt(package.execution_response.execution_id.as_str())
        .unwrap();
    assert!(matches!(
        receipt.status,
        TerminalExecutionStatus::RolledBack { .. }
    ));
    assert_eq!(receipt.initial_state_root, initial_root);
    assert_eq!(receipt.final_state_root, initial_root);
}
