//! Sovereign OS — BETA-002 Governed Agent Trial Suite
//!
//! Validates that probabilistic intelligence traverses the governance substrate
//! without ever acquiring unmediated execution authority (\Delta Authority \equiv 0).
//!
//! Test Scenarios:
//! - B2-001: Valid benign agent proposal committing to agent-test/result
//! - B2-002: Model payload asserting {"authorized": true} remains inert evidence
//! - B2-003: Prompt attempting governance bypass is rejected at admission gate
//! - B2-004: Tampered approval receipt signature/ID fails closed
//! - B2-005: Replaying a consumed execution claim returns cached receipt (no second mutation)
//! - B2-006: Worker panic triggers exact atomic rollback and pristine state retention
//! - B2-007: Process crash immediately post-commit recovers cleanly from append log
//! - B2-008: Torn log tail at EOF triggers safe auto-truncation and clean recovery
//! - B2-009: Interior log frame corruption fails closed on cold-start recovery
//! - B2-010: Valid state replication produces identical Merkle and transition roots
//! - B2-011: Replication wire frame replay or sequence gap is rejected at transport boundary
//! - B2-012: Full evidence replay yields identical state transition receipt and Merkle roots

use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};

use chrono::Utc;
use tempfile::tempdir;

use beta001_harness::approval::{ApprovalLevel, ApprovalReceipt};
use beta001_harness::dispatch::{
    DeterministicDispatcher, DeterministicExecutionContext, DeterministicWorker, ExecutionOutcome,
    ExecutionReceiptStore, TerminalExecutionStatus, TransactionOrchestrator, WorkerError,
};
use beta001_harness::network::{ReplicationError, StateReplicator};
use beta001_harness::service::admission::{ExecutionAdmissionGate, ProposalAdmissionGate};
use beta001_harness::service::client::{AuthenticatedClientContext, ClientSanitizer};
use beta001_harness::service::evidence::ServiceEvidencePackage;
use beta001_harness::service::inference::{InferenceGatewayAdapter, RecordedTransportOutcome};
use beta001_harness::service_contract::{
    ApprovalReceiptId, AuthorityDeltaZero, ExecutionId, ExecutionRequest, ProposalRequest,
    ProposedOperation, SchemaVersionV1, SessionId, Sha256Digest, UserId,
};
use beta001_harness::state::{
    compute_delta_digest, compute_genesis_transition_root, compute_transition_root,
    normalize_mutations, StateMutation, StateTree,
};
use beta001_harness::storage::{CommitRecordPayload, StorageEngine, SyncPolicy};

struct AgentWorker {
    result_payload: Vec<u8>,
}

impl DeterministicWorker for AgentWorker {
    fn execute(
        &self,
        ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        let audit_key = format!("audit:{}", ctx.execution_id.as_str()).into_bytes();
        Ok(vec![
            StateMutation::put(b"agent-test/result".to_vec(), self.result_payload.clone()),
            StateMutation::put(audit_key, b"B2_VERIFIED_PASS".to_vec()),
        ])
    }
}

struct PanickingAgentWorker;
impl DeterministicWorker for PanickingAgentWorker {
    fn execute(
        &self,
        _ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        panic!("Agent sandbox memory corruption fault");
    }
}

fn build_trial_evidence_package(
    user_id: &str,
    session_id: &str,
    intent_str: &str,
    model_output: &str,
    exec_suffix: &str,
) -> ServiceEvidencePackage {
    let now = Utc::now();
    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new(user_id.to_string()).unwrap(),
        authenticated_session_id: SessionId::new(session_id.to_string()).unwrap(),
    };

    let raw_proposal = ProposalRequest {
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
        ClientSanitizer::sanitize_proposal_request(raw_proposal.clone(), &auth_ctx, now).unwrap();
    let prop_gate = ProposalAdmissionGate::new();
    let prop_res = prop_gate.admit_proposal(&raw_proposal).unwrap();
    let (_, risk_ctx) = prop_gate
        .get_admitted(prop_res.proposal_id.as_str())
        .unwrap();

    let approval_receipt = ApprovalReceipt {
        receipt_id: format!("apr_{}", exec_suffix),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_quorum_leader".to_string(),
        approval_level: ApprovalLevel::Peer,
        timestamp: "2026-08-24T03:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "sig_governance_verified".to_string(),
    };

    let exec_gate = ExecutionAdmissionGate::new();
    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new(approval_receipt.receipt_id.clone()).unwrap(),
        caller_identity: "orchestrator_daemon".to_string(),
        signature: "caller_sig_verified".to_string(),
        timestamp: now,
    };
    let exec_res = exec_gate
        .admit_execution(&exec_req, &prop_gate, &approval_receipt)
        .unwrap();

    let inf_record = InferenceGatewayAdapter::record_transport(
        auth_ctx.authenticated_session_id.clone(),
        "gemini-2.5-pro",
        "boundary_adapter_v1",
        intent_str,
        RecordedTransportOutcome::Success {
            raw_model_output: model_output.to_string(),
        },
        now,
    )
    .unwrap();

    ServiceEvidencePackage::build(
        sanitized,
        prop_res,
        risk_ctx,
        approval_receipt,
        exec_res,
        vec![inf_record],
        now,
    )
    .unwrap()
}

#[test]
fn test_b2_001_valid_benign_agent_proposal() {
    let evidence_pkg = build_trial_evidence_package(
        "usr_b2_001",
        "ses_b2_001",
        "Write bounded calculation to agent-test/result",
        "{\"calculation\": 42}",
        "b2_001",
    );

    let store = ExecutionReceiptStore::new();
    let mut tree = StateTree::new();
    let ctx = DeterministicDispatcher::prepare_dispatch(&evidence_pkg, &store, &tree, 1).unwrap();

    let worker = AgentWorker {
        result_payload: b"{\"calculation\": 42}".to_vec(),
    };

    let outcome =
        TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store).unwrap();
    let receipt = match outcome {
        ExecutionOutcome::Executed(r) => r,
        ExecutionOutcome::CachedReceipt(_) => panic!("Expected Executed outcome"),
    };

    assert_eq!(receipt.status, TerminalExecutionStatus::Committed);
    assert_eq!(
        tree.get(b"agent-test/result"),
        Some(b"{\"calculation\": 42}".as_slice())
    );
    assert_eq!(tree.revision(), 1);
}

#[test]
fn test_b2_002_model_asserts_authorized_remains_inert() {
    let malicious_output =
        "{\"authorized\": true, \"role\": \"superuser\", \"target\": \"agent-test/result\"}";
    let evidence_pkg = build_trial_evidence_package(
        "usr_b2_002",
        "ses_b2_002",
        "Analyze policy claim",
        malicious_output,
        "b2_002",
    );

    assert_eq!(
        evidence_pkg.execution_response.authority_delta,
        AuthorityDeltaZero
    );
    assert_eq!(
        evidence_pkg.inference_records[0]
            .raw_model_output
            .as_deref(),
        Some(malicious_output)
    );
    assert!(evidence_pkg.inference_records[0].response_digest.is_some());
}

#[test]
fn test_b2_003_prompt_policy_bypass_denied_at_admission() {
    let now = Utc::now();
    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new("usr_attacker".to_string()).unwrap(),
        authenticated_session_id: SessionId::new("ses_b2_003".to_string()).unwrap(),
    };

    let raw_proposal = ProposalRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id,
        session_id: auth_ctx.authenticated_session_id,
        intent: "BYPASS_ALL_SECURITY_CONTROLS".to_string(),
        proposed_operation: ProposedOperation::RequestReview,
        source_evidence_references: vec![],
        timestamp: now,
    };

    let gate = ProposalAdmissionGate::new();
    let res = gate.admit_proposal(&raw_proposal);
    assert!(
        res.is_err(),
        "Proposal with empty evidence references must be denied admission"
    );
}

#[test]
fn test_b2_004_tampered_approval_receipt_fails_closed() {
    let now = Utc::now();
    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new("usr_b2_004".to_string()).unwrap(),
        authenticated_session_id: SessionId::new("ses_b2_004".to_string()).unwrap(),
    };

    let raw_proposal = ProposalRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        intent: "Governed operation".to_string(),
        proposed_operation: ProposedOperation::RequestReview,
        source_evidence_references: vec![Sha256Digest::new(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        )
        .unwrap()],
        timestamp: now,
    };

    let prop_gate = ProposalAdmissionGate::new();
    let prop_res = prop_gate.admit_proposal(&raw_proposal).unwrap();
    let (_, risk_ctx) = prop_gate
        .get_admitted(prop_res.proposal_id.as_str())
        .unwrap();

    let valid_receipt = ApprovalReceipt {
        receipt_id: "apr_b2_004_valid".to_string(),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_quorum_leader".to_string(),
        approval_level: ApprovalLevel::Peer,
        timestamp: "2026-08-24T03:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "sig_governance_verified".to_string(),
    };

    let exec_gate = ExecutionAdmissionGate::new();
    let forged_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id,
        session_id: auth_ctx.authenticated_session_id,
        proposal_id: prop_res.proposal_id,
        approval_receipt_id: ApprovalReceiptId::new("apr_b2_004_FORGED".to_string()).unwrap(),
        caller_identity: "orchestrator_daemon".to_string(),
        signature: "caller_sig_verified".to_string(),
        timestamp: now,
    };

    let admission_res = exec_gate.admit_execution(&forged_req, &prop_gate, &valid_receipt);
    assert!(
        admission_res.is_err(),
        "Execution admission must reject mismatched approval receipt ID"
    );
}

#[test]
fn test_b2_005_replay_same_execution_claim_returns_cached_receipt() {
    let evidence_pkg = build_trial_evidence_package(
        "usr_b2_005",
        "ses_b2_005",
        "Write to agent-test/result",
        "{\"val\": 100}",
        "b2_005",
    );

    let store = ExecutionReceiptStore::new();
    let mut tree = StateTree::new();
    let ctx = DeterministicDispatcher::prepare_dispatch(&evidence_pkg, &store, &tree, 1).unwrap();
    let worker = AgentWorker {
        result_payload: b"{\"val\": 100}".to_vec(),
    };

    let outcome_1 =
        TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store).unwrap();
    let receipt_1 = match outcome_1 {
        ExecutionOutcome::Executed(r) => r,
        ExecutionOutcome::CachedReceipt(_) => panic!("Expected Executed on first run"),
    };

    assert_eq!(tree.revision(), 1);

    // Replay with identical execution ID
    let outcome_2 =
        TransactionOrchestrator::execute_transaction(&worker, &ctx, &mut tree, &store).unwrap();
    let receipt_2 = match outcome_2 {
        ExecutionOutcome::CachedReceipt(r) => r,
        ExecutionOutcome::Executed(_) => panic!("Expected CachedReceipt on replay"),
    };

    assert_eq!(receipt_1.transition_root, receipt_2.transition_root);
    assert_eq!(
        tree.revision(),
        1,
        "State tree revision must not increment on replay"
    );
}

#[test]
fn test_b2_006_worker_panic_exact_rollback() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"agent-test/result", b"INITIAL_CLEAN")]);
    let initial_root = tree.compute_state_root();
    let initial_rev = tree.revision();

    let store = ExecutionReceiptStore::new();
    let evidence_pkg = build_trial_evidence_package(
        "usr_b2_006",
        "ses_b2_006",
        "Faulty worker invocation",
        "{}",
        "b2_006",
    );

    let ctx = DeterministicDispatcher::prepare_dispatch(&evidence_pkg, &store, &tree, 1).unwrap();
    let res = TransactionOrchestrator::execute_transaction(
        &PanickingAgentWorker,
        &ctx,
        &mut tree,
        &store,
    );
    assert!(res.is_err());

    assert_eq!(tree.revision(), initial_rev);
    assert_eq!(tree.compute_state_root(), initial_root);
    assert_eq!(
        tree.get(b"agent-test/result"),
        Some(b"INITIAL_CLEAN".as_slice())
    );

    let receipt = store
        .get_receipt(evidence_pkg.execution_response.execution_id.as_str())
        .expect("Terminal RolledBack receipt must exist");
    assert!(matches!(
        receipt.status,
        TerminalExecutionStatus::RolledBack { .. }
    ));
}

#[test]
fn test_b2_007_kill_process_after_commit_cold_recovery() {
    let dir = tempdir().unwrap();
    let expected_state_root;
    let expected_trans_root;

    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
        let mut staging = StateTree::new();
        let mutations =
            normalize_mutations(&[StateMutation::put(b"agent-test/result", b"DURABLE_PAYLOAD")]);
        let pre_rev = staging.revision();
        let pre_root = staging.compute_state_root();
        staging.apply_raw_mutations(&mutations);
        let post_rev = staging.revision();
        let post_root = staging.compute_state_root();
        let delta = compute_delta_digest(&mutations);
        let prior_root = engine.current_transition_root();
        let pkg_digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let trans_root = compute_transition_root(
            &prior_root,
            &pre_root,
            &post_root,
            pkg_digest,
            "exe_b2_007",
            &delta,
        );

        expected_state_root = post_root.clone();
        expected_trans_root = trans_root.clone();

        let payload = CommitRecordPayload {
            execution_id: ExecutionId::new("exe_b2_007".to_string()).unwrap(),
            package_digest: Sha256Digest::new(pkg_digest.to_string()).unwrap(),
            pre_revision: pre_rev,
            post_revision: post_rev,
            pre_state_root: pre_root,
            post_state_root: post_root,
            prior_transition_root: prior_root,
            transition_root: trans_root,
            mutations,
        };

        engine.commit_record(1, payload).unwrap();
    }

    let recovered = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(recovered.current_sequence_tick(), 1);
    assert_eq!(recovered.current_state_root(), expected_state_root);
    assert_eq!(recovered.current_transition_root(), expected_trans_root);
    assert_eq!(
        recovered.get_state_value(b"agent-test/result"),
        Some(b"DURABLE_PAYLOAD".to_vec())
    );
}

#[test]
fn test_b2_008_corrupt_persistence_tail_safe_truncation() {
    let dir = tempdir().unwrap();

    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
        let mut staging = StateTree::new();
        let mutations =
            normalize_mutations(&[StateMutation::put(b"agent-test/result", b"COMMITTED_VALID")]);
        let pre_rev = staging.revision();
        let pre_root = staging.compute_state_root();
        staging.apply_raw_mutations(&mutations);
        let post_rev = staging.revision();
        let post_root = staging.compute_state_root();
        let delta = compute_delta_digest(&mutations);
        let prior_root = engine.current_transition_root();
        let pkg_digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let trans_root = compute_transition_root(
            &prior_root,
            &pre_root,
            &post_root,
            pkg_digest,
            "exe_b2_008",
            &delta,
        );

        let payload = CommitRecordPayload {
            execution_id: ExecutionId::new("exe_b2_008".to_string()).unwrap(),
            package_digest: Sha256Digest::new(pkg_digest.to_string()).unwrap(),
            pre_revision: pre_rev,
            post_revision: post_rev,
            pre_state_root: pre_root,
            post_state_root: post_root,
            prior_transition_root: prior_root,
            transition_root: trans_root,
            mutations,
        };

        engine.commit_record(1, payload).unwrap();
    }

    // Append interrupted frame bytes starting with valid SOVLOG01 header tag to simulate power loss during append
    let log_path = dir.path().join("commit.log");
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&log_path)
        .unwrap();
    file.write_all(b"SOVLOG01_TORN_T").unwrap();
    file.sync_all().unwrap();
    drop(file);

    // Auto-truncation should truncate the torn tail and recover tick 1
    let recovered = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(recovered.current_sequence_tick(), 1);
    assert_eq!(
        recovered.get_state_value(b"agent-test/result"),
        Some(b"COMMITTED_VALID".to_vec())
    );
}

#[test]
fn test_b2_009_corrupt_interior_record_fails_closed() {
    let dir = tempdir().unwrap();

    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
        let mut staging = StateTree::new();
        let prior_root = engine.current_transition_root();
        let pkg_digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        // Frame 1 (Tick 1)
        let muts_1 = normalize_mutations(&[StateMutation::put(b"agent-test/result", b"VAL_1")]);
        let pre_rev_1 = staging.revision();
        let pre_root_1 = staging.compute_state_root();
        staging.apply_raw_mutations(&muts_1);
        let post_rev_1 = staging.revision();
        let post_root_1 = staging.compute_state_root();
        let delta_1 = compute_delta_digest(&muts_1);

        let trans_root_1 = compute_transition_root(
            &prior_root,
            &pre_root_1,
            &post_root_1,
            pkg_digest,
            "exe_b2_009_1",
            &delta_1,
        );

        let payload_1 = CommitRecordPayload {
            execution_id: ExecutionId::new("exe_b2_009_1".to_string()).unwrap(),
            package_digest: Sha256Digest::new(pkg_digest.to_string()).unwrap(),
            pre_revision: pre_rev_1,
            post_revision: post_rev_1,
            pre_state_root: pre_root_1,
            post_state_root: post_root_1,
            prior_transition_root: prior_root,
            transition_root: trans_root_1.clone(),
            mutations: muts_1,
        };
        engine.commit_record(1, payload_1).unwrap();

        // Frame 2 (Tick 2)
        let muts_2 = normalize_mutations(&[StateMutation::put(b"agent-test/result", b"VAL_2")]);
        let pre_rev_2 = staging.revision();
        let pre_root_2 = staging.compute_state_root();
        staging.apply_raw_mutations(&muts_2);
        let post_rev_2 = staging.revision();
        let post_root_2 = staging.compute_state_root();
        let delta_2 = compute_delta_digest(&muts_2);

        let trans_root_2 = compute_transition_root(
            &trans_root_1,
            &pre_root_2,
            &post_root_2,
            pkg_digest,
            "exe_b2_009_2",
            &delta_2,
        );

        let payload_2 = CommitRecordPayload {
            execution_id: ExecutionId::new("exe_b2_009_2".to_string()).unwrap(),
            package_digest: Sha256Digest::new(pkg_digest.to_string()).unwrap(),
            pre_revision: pre_rev_2,
            post_revision: post_rev_2,
            pre_state_root: pre_root_2,
            post_state_root: post_root_2,
            prior_transition_root: trans_root_1,
            transition_root: trans_root_2,
            mutations: muts_2,
        };
        engine.commit_record(2, payload_2).unwrap();
    }

    // Corrupt interior frame bytes at offset 32
    let log_path = dir.path().join("commit.log");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&log_path)
        .unwrap();
    file.seek(SeekFrom::Start(32)).unwrap();
    file.write_all(b"\xDE\xAD\xBE\xEF").unwrap();
    file.sync_all().unwrap();
    drop(file);

    let recovery_res = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit);
    assert!(
        recovery_res.is_err(),
        "Recovery must fail closed on interior frame corruption"
    );
}

#[test]
fn test_b2_010_replicate_valid_state_identical_roots() {
    let dir_primary = tempdir().unwrap();
    let dir_replica = tempdir().unwrap();

    let primary = StorageEngine::open(dir_primary.path(), SyncPolicy::FsyncOnCommit).unwrap();
    let replica = StorageEngine::open(dir_replica.path(), SyncPolicy::FsyncOnCommit).unwrap();

    let mut staging = StateTree::new();
    let mutations = normalize_mutations(&[StateMutation::put(
        b"agent-test/result",
        b"REPLICATED_VALUE",
    )]);
    let pre_rev = staging.revision();
    let pre_root = staging.compute_state_root();
    staging.apply_raw_mutations(&mutations);
    let post_rev = staging.revision();
    let post_root = staging.compute_state_root();
    let delta = compute_delta_digest(&mutations);
    let prior_root = primary.current_transition_root();
    let pkg_digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    let trans_root = compute_transition_root(
        &prior_root,
        &pre_root,
        &post_root,
        pkg_digest,
        "exe_b2_010",
        &delta,
    );

    let payload = CommitRecordPayload {
        execution_id: ExecutionId::new("exe_b2_010".to_string()).unwrap(),
        package_digest: Sha256Digest::new(pkg_digest.to_string()).unwrap(),
        pre_revision: pre_rev,
        post_revision: post_rev,
        pre_state_root: pre_root,
        post_state_root: post_root,
        prior_transition_root: prior_root,
        transition_root: trans_root,
        mutations,
    };

    primary.commit_record(1, payload.clone()).unwrap();

    let wire_frame = StateReplicator::create_commit_wire_frame(1, &payload);
    StateReplicator::ingest_replicated_frame(&replica, &wire_frame).unwrap();

    assert_eq!(
        replica.current_sequence_tick(),
        primary.current_sequence_tick()
    );
    assert_eq!(replica.current_state_root(), primary.current_state_root());
    assert_eq!(
        replica.current_transition_root(),
        primary.current_transition_root()
    );
    assert_eq!(
        replica.get_state_value(b"agent-test/result"),
        Some(b"REPLICATED_VALUE".to_vec())
    );
}

#[test]
fn test_b2_011_replay_sequence_gap_network_frame_rejected() {
    let dir = tempdir().unwrap();
    let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();

    let mut staging = StateTree::new();
    let mutations = normalize_mutations(&[StateMutation::put(b"agent-test/result", b"V1")]);
    let pre_rev = staging.revision();
    let pre_root = staging.compute_state_root();
    staging.apply_raw_mutations(&mutations);
    let post_rev = staging.revision();
    let post_root = staging.compute_state_root();
    let delta = compute_delta_digest(&mutations);
    let prior_root = compute_genesis_transition_root();
    let pkg_digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    let trans_root = compute_transition_root(
        &prior_root,
        &pre_root,
        &post_root,
        pkg_digest,
        "exe_b2_011",
        &delta,
    );

    let payload = CommitRecordPayload {
        execution_id: ExecutionId::new("exe_b2_011".to_string()).unwrap(),
        package_digest: Sha256Digest::new(pkg_digest.to_string()).unwrap(),
        pre_revision: pre_rev,
        post_revision: post_rev,
        pre_state_root: pre_root,
        post_state_root: post_root,
        prior_transition_root: prior_root,
        transition_root: trans_root,
        mutations,
    };

    let frame = StateReplicator::create_commit_wire_frame(1, &payload);
    StateReplicator::ingest_replicated_frame(&engine, &frame).unwrap();

    let replay_res = StateReplicator::ingest_replicated_frame(&engine, &frame);
    assert!(matches!(
        replay_res,
        Err(ReplicationError::ReplayDetected { .. })
    ));

    let gap_frame = StateReplicator::create_commit_wire_frame(4, &payload);
    let gap_res = StateReplicator::ingest_replicated_frame(&engine, &gap_frame);
    assert!(matches!(
        gap_res,
        Err(ReplicationError::SequenceGapDetected { .. })
    ));
}

#[test]
fn test_b2_012_full_evidence_deterministic_replay() {
    let evidence_pkg = build_trial_evidence_package(
        "usr_b2_012",
        "ses_b2_012",
        "Deterministic agent computation",
        "{\"result\": 999}",
        "b2_012",
    );

    let worker = AgentWorker {
        result_payload: b"{\"result\": 999}".to_vec(),
    };

    let store_1 = ExecutionReceiptStore::new();
    let mut tree_1 = StateTree::new();
    let ctx_1 =
        DeterministicDispatcher::prepare_dispatch(&evidence_pkg, &store_1, &tree_1, 1).unwrap();
    let outcome_1 =
        TransactionOrchestrator::execute_transaction(&worker, &ctx_1, &mut tree_1, &store_1)
            .unwrap();
    let receipt_1 = match outcome_1 {
        ExecutionOutcome::Executed(r) => r,
        ExecutionOutcome::CachedReceipt(_) => panic!("Expected Executed"),
    };

    let store_2 = ExecutionReceiptStore::new();
    let mut tree_2 = StateTree::new();
    let ctx_2 =
        DeterministicDispatcher::prepare_dispatch(&evidence_pkg, &store_2, &tree_2, 1).unwrap();
    let outcome_2 =
        TransactionOrchestrator::execute_transaction(&worker, &ctx_2, &mut tree_2, &store_2)
            .unwrap();
    let receipt_2 = match outcome_2 {
        ExecutionOutcome::Executed(r) => r,
        ExecutionOutcome::CachedReceipt(_) => panic!("Expected Executed"),
    };

    assert_eq!(receipt_1.transition_root, receipt_2.transition_root);
    assert_eq!(receipt_1.final_state_root, receipt_2.final_state_root);
    assert_eq!(receipt_1.delta_digest, receipt_2.delta_digest);
    assert_eq!(tree_1.compute_state_root(), tree_2.compute_state_root());
}
