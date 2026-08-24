//! Boundary Tests for ADAM-012-B
//!
//! Validates acceptance criteria B012-001 through B012-007.

use chrono::Utc;

use beta001_harness::approval::{ApprovalLevel, ApprovalReceipt};
use beta001_harness::dispatch::{
    DeterministicDispatcher, DeterministicExecutionContext, DeterministicWorker, DispatchError,
    ExecutionReservationStore, WorkerError,
};
use beta001_harness::service::admission::{ExecutionAdmissionGate, ProposalAdmissionGate};
use beta001_harness::service::client::{AuthenticatedClientContext, ClientSanitizer};
use beta001_harness::service::evidence::ServiceEvidencePackage;
use beta001_harness::service::inference::{InferenceGatewayAdapter, RecordedTransportOutcome};
use beta001_harness::service_contract::{
    ApprovalReceiptId, ClaimState, DispatchState, ExecutionRequest, OutcomeState, ProposalRequest,
    ProposedOperation, SchemaVersionV1, SessionId, Sha256Digest, UserId,
};
use beta001_harness::state::{StateMutation, StateTree};

fn create_valid_evidence_package() -> ServiceEvidencePackage {
    let now = Utc::now();
    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new("usr_dispatch_test".to_string()).unwrap(),
        authenticated_session_id: SessionId::new("ses_dispatch_100".to_string()).unwrap(),
    };

    let raw_req = ProposalRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        intent: "Governed state transaction request".to_string(),
        proposed_operation: ProposedOperation::StateTransition,
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
        receipt_id: "apr_gov_dispatch_1".to_string(),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_peer_1".to_string(),
        approval_level: ApprovalLevel::Peer,
        timestamp: "2026-08-24T02:05:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "crypto_sig_dispatch".to_string(),
    };

    let exec_gate = ExecutionAdmissionGate::new();
    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new(receipt.receipt_id.clone()).unwrap(),
        caller_identity: "agent_runner_1".to_string(),
        signature: "caller_sig_xyz".to_string(),
        timestamp: now,
    };
    let exec_res = exec_gate
        .admit_execution(&exec_req, &prop_gate, &receipt)
        .unwrap();

    let inf = InferenceGatewayAdapter::record_transport(
        auth_ctx.authenticated_session_id.clone(),
        "gemini-2.5-pro",
        "adapter_v1",
        "Evaluate dispatch precondition",
        RecordedTransportOutcome::Success {
            raw_model_output: "Ready".to_string(),
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

struct MockSuccessWorker;
impl DeterministicWorker for MockSuccessWorker {
    fn execute(
        &self,
        ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        let key = format!("state:{}", ctx.execution_id.as_str()).into_bytes();
        Ok(vec![StateMutation::put(key, b"completed")])
    }
}

struct MockPanicWorker;
impl DeterministicWorker for MockPanicWorker {
    fn execute(
        &self,
        _ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        panic!("Fatal worker invariant breached");
    }
}

#[test]
fn test_b012_001_invalid_evidence_package_rejected() {
    let mut package = create_valid_evidence_package();
    package.proposal_response.provenance_digest = Sha256Digest::new(
        "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
    )
    .unwrap();

    let store = ExecutionReservationStore::new();
    let tree = StateTree::new();

    let res = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 1);
    assert!(matches!(
        res,
        Err(DispatchError::EvidenceVerificationFailed(_))
    ));
}

#[test]
fn test_b012_002_invalid_pre_dispatch_triad_rejected() {
    let mut package = create_valid_evidence_package();
    package.execution_response.dispatch_state = DispatchState::Dispatched; // Invalid pre-dispatch

    // Reseal package with modified execution response
    let resealed = ServiceEvidencePackage::build(
        package.sanitized_ingress,
        package.proposal_response,
        package.risk_context,
        package.approval_receipt,
        package.execution_response,
        package.inference_records,
        package.closed_timestamp,
    )
    .unwrap();

    let store = ExecutionReservationStore::new();
    let tree = StateTree::new();

    let res = DeterministicDispatcher::prepare_dispatch(&resealed, &store, &tree, 1);
    assert!(matches!(
        res,
        Err(DispatchError::InvalidPreDispatchState(_))
    ));
}

#[test]
fn test_b012_003_state_root_mismatch_rejected() {
    let package = create_valid_evidence_package();
    let store = ExecutionReservationStore::new();
    let tree = StateTree::new();

    let ctx = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 1).unwrap();

    // Mutate state tree after preparation to simulate drift
    let mut drifted_tree = tree.clone();
    drifted_tree.apply_raw_mutations(&[StateMutation::put(b"drift_key", b"drift_val")]);

    let worker = MockSuccessWorker;
    let res = DeterministicDispatcher::dispatch_sandboxed(&worker, &ctx, &drifted_tree);
    assert!(matches!(res, Err(DispatchError::StateRootMismatch { .. })));
}

#[test]
fn test_b012_004_atomic_reservation_cas_and_duplicate_rejection() {
    let package = create_valid_evidence_package();
    let store = ExecutionReservationStore::new();
    let tree = StateTree::new();

    let res1 = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 1);
    assert!(res1.is_ok());

    // Second reservation attempt with same execution_id must fail
    let res2 = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 2);
    assert!(matches!(
        res2,
        Err(DispatchError::DispatchAlreadyReserved(_))
    ));
}

#[test]
fn test_b012_005_deterministic_context_and_seed_derivation() {
    let package = create_valid_evidence_package();
    let store = ExecutionReservationStore::new();
    let tree = StateTree::new();

    let ctx = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 1).unwrap();

    assert_eq!(ctx.expected_revision, 0);
    assert_eq!(ctx.expected_state_root, tree.compute_state_root());
    assert_eq!(ctx.granted_scope, ProposedOperation::StateTransition);

    let expected_seed = DeterministicExecutionContext::derive_seed(
        &package.package_digest,
        &package.execution_response.execution_id,
        0,
    );
    assert_eq!(ctx.deterministic_seed, expected_seed);
}

#[test]
fn test_b012_006_worker_panic_contained_without_leak() {
    let package = create_valid_evidence_package();
    let store = ExecutionReservationStore::new();
    let tree = StateTree::new();

    let ctx = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 1).unwrap();
    let panic_worker = MockPanicWorker;

    let res = DeterministicDispatcher::dispatch_sandboxed(&panic_worker, &ctx, &tree);
    assert!(
        matches!(res, Err(DispatchError::WorkerFailed(WorkerError::ExecutionFailure(msg))) if msg.contains("PANIC_CONTAINED"))
    );

    // Tree remains completely unchanged
    assert_eq!(tree.revision(), 0);
}

#[test]
fn test_b012_007_worker_execution_purity() {
    let package = create_valid_evidence_package();
    let store = ExecutionReservationStore::new();
    let tree = StateTree::new();

    let ctx = DeterministicDispatcher::prepare_dispatch(&package, &store, &tree, 1).unwrap();
    let worker = MockSuccessWorker;

    let mutations = DeterministicDispatcher::dispatch_sandboxed(&worker, &ctx, &tree).unwrap();
    assert_eq!(mutations.len(), 1);
    assert_eq!(
        mutations[0].key(),
        format!("state:{}", ctx.execution_id.as_str()).as_bytes()
    );

    // Ensure snapshot was strictly read-only and no mutation escaped to tree
    assert_eq!(tree.len(), 0);
    assert_eq!(tree.revision(), 0);
}
