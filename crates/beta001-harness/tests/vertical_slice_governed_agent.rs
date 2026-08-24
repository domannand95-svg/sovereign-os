//! End-to-End Governed Agent Vertical Slice Integration Suite
//!
//! Executes an unprivileged governed agent task across all 14 architectural layers:
//! 1. Ingress & Client Sanitization (ADAM-001)
//! 2. Inference Gateway Containment (ADAM-002, ADAM-003)
//! 3. Proposal Admission & Policy Risk Assessment (ADAM-004, ADAM-005)
//! 4. Approval Gating & Capability Issuance (ADAM-006..011)
//! 5. Deterministic Dispatch & Transaction Rollback Integrity (ADAM-012)
//! 6. Durable Append-Only Persistence (SOVLOG01) & Fsync Barrier (ADAM-013)
//! 7. Inter-Node Wire Protocol Replication (SOVWIRE1) & Merkle Convergence (ADAM-014)
//! 8. Independent Cold-Start Crash Recovery Invariance (ADAM-013)

use chrono::Utc;
use tempfile::tempdir;

use beta001_harness::approval::{ApprovalLevel, ApprovalReceipt};
use beta001_harness::dispatch::{
    DeterministicDispatcher, DeterministicExecutionContext, DeterministicWorker, ExecutionOutcome,
    ExecutionReceiptStore, TerminalExecutionStatus, TransactionOrchestrator, WorkerError,
};
use beta001_harness::network::{
    HandshakeController, HandshakePayload, StateReplicator, WireFrame, WireMessageType,
    DEFAULT_MAX_WIRE_PAYLOAD_BYTES, HANDSHAKE_PROTOCOL_VERSION_V1,
};
use beta001_harness::service::admission::{ExecutionAdmissionGate, ProposalAdmissionGate};
use beta001_harness::service::client::{AuthenticatedClientContext, ClientSanitizer};
use beta001_harness::service::evidence::ServiceEvidencePackage;
use beta001_harness::service::inference::{InferenceGatewayAdapter, RecordedTransportOutcome};
use beta001_harness::service_contract::{
    ApprovalReceiptId, AuthorityDeltaZero, ExecutionRequest, ProposalRequest, ProposedOperation,
    SchemaVersionV1, SessionId, Sha256Digest, UserId,
};
use beta001_harness::state::{normalize_mutations, StateMutation, StateTree};
use beta001_harness::storage::{CommitRecordPayload, StorageEngine, SyncPolicy};

struct GovernedAgentWorker {
    target_key: Vec<u8>,
    target_value: Vec<u8>,
}

impl DeterministicWorker for GovernedAgentWorker {
    fn execute(
        &self,
        ctx: &DeterministicExecutionContext,
        _snapshot: &StateTree,
    ) -> Result<Vec<StateMutation>, WorkerError> {
        let audit_key = format!("audit:{}", ctx.execution_id.as_str()).into_bytes();
        Ok(vec![
            StateMutation::put(self.target_key.clone(), self.target_value.clone()),
            StateMutation::put(audit_key, b"GOVERNED_PASS".to_vec()),
        ])
    }
}

#[test]
fn test_vertical_slice_full_lifecycle_governed_agent_execution() {
    // -------------------------------------------------------------------------
    // Phase 1: Storage & Cluster Initialization (ADAM-013, ADAM-014)
    // -------------------------------------------------------------------------
    let dir_primary = tempdir().expect("Failed to create primary tempdir");
    let dir_replica = tempdir().expect("Failed to create replica tempdir");
    let cluster_id = "sovereign_prime_cluster";

    let primary_engine = StorageEngine::open(dir_primary.path(), SyncPolicy::FsyncOnCommit)
        .expect("Failed to open primary storage engine");
    let replica_engine = StorageEngine::open(dir_replica.path(), SyncPolicy::FsyncOnCommit)
        .expect("Failed to open replica storage engine");

    assert_eq!(primary_engine.current_sequence_tick(), 0);
    assert_eq!(replica_engine.current_sequence_tick(), 0);
    assert_eq!(
        primary_engine.current_state_root(),
        replica_engine.current_state_root()
    );

    let handshake_payload = HandshakePayload {
        node_id: "node_beta".to_string(),
        cluster_id: cluster_id.to_string(),
        protocol_version: HANDSHAKE_PROTOCOL_VERSION_V1,
        sequence_tick: replica_engine.current_sequence_tick(),
        state_root: replica_engine.current_state_root(),
        transition_root: replica_engine.current_transition_root(),
    };
    let session = HandshakeController::verify_incoming(cluster_id, &handshake_payload)
        .expect("Handshake verification failed");
    assert_eq!(session.peer_node_id, "node_beta");

    // -------------------------------------------------------------------------
    // Phase 2: Ingress & Client Sanitization (ADAM-001)
    // -------------------------------------------------------------------------
    let now = Utc::now();
    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new("usr_agent_runner_01".to_string())
            .expect("Valid UserId"),
        authenticated_session_id: SessionId::new("ses_vs_001".to_string())
            .expect("Valid SessionId"),
    };

    let raw_proposal = ProposalRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        intent: "Set sys:service_status to ACTIVE".to_string(),
        proposed_operation: ProposedOperation::RequestReview,
        source_evidence_references: vec![Sha256Digest::new(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        )
        .expect("Valid Sha256Digest")],
        timestamp: now,
    };

    let sanitized =
        ClientSanitizer::sanitize_proposal_request(raw_proposal.clone(), &auth_ctx, now)
            .expect("Sanitization failed");

    // -------------------------------------------------------------------------
    // Phase 3: Proposal Admission & Risk Evaluation (ADAM-004, ADAM-005)
    // -------------------------------------------------------------------------
    let prop_gate = ProposalAdmissionGate::new();
    let prop_res = prop_gate
        .admit_proposal(&raw_proposal)
        .expect("Proposal admission failed");
    let (_, risk_ctx) = prop_gate
        .get_admitted(prop_res.proposal_id.as_str())
        .expect("Get admitted failed");

    // -------------------------------------------------------------------------
    // Phase 4: Multi-Party Approval Gating (ADAM-006)
    // -------------------------------------------------------------------------
    let approval_receipt = ApprovalReceipt {
        receipt_id: format!("apr_{}", prop_res.proposal_id.as_str()),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_quorum_leader".to_string(),
        approval_level: ApprovalLevel::Peer,
        timestamp: "2026-08-24T03:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "sig_governance_verified".to_string(),
    };

    // -------------------------------------------------------------------------
    // Phase 5: Execution Admission & Capability Issuance (ADAM-007..011)
    // -------------------------------------------------------------------------
    let exec_gate = ExecutionAdmissionGate::new();
    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new(approval_receipt.receipt_id.clone())
            .expect("Valid ApprovalReceiptId"),
        caller_identity: "orchestrator_daemon".to_string(),
        signature: "caller_sig_verified".to_string(),
        timestamp: now,
    };
    let exec_res = exec_gate
        .admit_execution(&exec_req, &prop_gate, &approval_receipt)
        .expect("Execution admission failed");
    assert_eq!(exec_res.authority_delta, AuthorityDeltaZero);

    // -------------------------------------------------------------------------
    // Phase 6: Inference Gateway Recording (ADAM-002, ADAM-003)
    // -------------------------------------------------------------------------
    let inf_record = InferenceGatewayAdapter::record_transport(
        auth_ctx.authenticated_session_id.clone(),
        "gemini-2.5-pro",
        "boundary_adapter_v1",
        "Evaluate state mutation preconditions",
        RecordedTransportOutcome::Success {
            raw_model_output: "PRECONDITIONS_VALIDATED".to_string(),
        },
        now,
    )
    .expect("Inference recording failed");

    let evidence_pkg = ServiceEvidencePackage::build(
        sanitized,
        prop_res,
        risk_ctx,
        approval_receipt,
        exec_res,
        vec![inf_record],
        now,
    )
    .expect("Evidence packaging failed");

    // -------------------------------------------------------------------------
    // Phase 7: Deterministic Dispatch & Transaction Execution (ADAM-012)
    // -------------------------------------------------------------------------
    let receipt_store = ExecutionReceiptStore::new();
    let mut staging_tree = StateTree::new();

    let dispatch_ctx =
        DeterministicDispatcher::prepare_dispatch(&evidence_pkg, &receipt_store, &staging_tree, 1)
            .expect("Prepare dispatch failed");

    let worker = GovernedAgentWorker {
        target_key: b"sys:service_status".to_vec(),
        target_value: b"ACTIVE".to_vec(),
    };

    let outcome = TransactionOrchestrator::execute_transaction(
        &worker,
        &dispatch_ctx,
        &mut staging_tree,
        &receipt_store,
    )
    .expect("Execute transaction failed");

    let execution_receipt = match outcome {
        ExecutionOutcome::Executed(r) => r,
        ExecutionOutcome::CachedReceipt(r) => panic!("Unexpected cached receipt: {:?}", r),
    };

    assert_eq!(execution_receipt.status, TerminalExecutionStatus::Committed);
    assert_eq!(execution_receipt.initial_revision, 0);
    assert_eq!(execution_receipt.final_revision, 1);
    assert!(execution_receipt.verify_integrity());

    // -------------------------------------------------------------------------
    // Phase 8: Commit to Primary Storage Engine (ADAM-013)
    // -------------------------------------------------------------------------
    let raw_mutations = vec![
        StateMutation::put(b"sys:service_status", b"ACTIVE"),
        StateMutation::put(
            format!(
                "audit:{}",
                evidence_pkg.execution_response.execution_id.as_str()
            )
            .into_bytes(),
            b"GOVERNED_PASS".to_vec(),
        ),
    ];
    let approved_mutations = normalize_mutations(&raw_mutations);

    let commit_payload = CommitRecordPayload {
        execution_id: evidence_pkg.execution_response.execution_id.clone(),
        package_digest: dispatch_ctx.package_digest.clone(),
        pre_revision: execution_receipt.initial_revision,
        post_revision: execution_receipt.final_revision,
        pre_state_root: execution_receipt.initial_state_root.clone(),
        post_state_root: execution_receipt.final_state_root.clone(),
        prior_transition_root: execution_receipt.previous_transition_root.clone(),
        transition_root: execution_receipt.transition_root.clone(),
        mutations: approved_mutations,
    };

    primary_engine
        .commit_record(1, commit_payload.clone())
        .expect("Primary commit record failed");
    assert_eq!(primary_engine.current_sequence_tick(), 1);
    assert_eq!(
        primary_engine.current_state_root(),
        staging_tree.compute_state_root()
    );
    assert_eq!(
        primary_engine.current_transition_root(),
        execution_receipt.transition_root
    );

    // -------------------------------------------------------------------------
    // Phase 9: Wire Protocol Replication to Replica (ADAM-014)
    // -------------------------------------------------------------------------
    let wire_frame = StateReplicator::create_commit_wire_frame(1, &commit_payload);

    let mut network_buffer = Vec::new();
    wire_frame
        .write_to(&mut network_buffer)
        .expect("Wire frame serialize failed");

    let decoded_frame =
        WireFrame::read_from(network_buffer.as_slice(), DEFAULT_MAX_WIRE_PAYLOAD_BYTES)
            .expect("Wire frame read failed");
    assert_eq!(decoded_frame.msg_type, WireMessageType::CommitFrame);

    StateReplicator::ingest_replicated_frame(&replica_engine, &decoded_frame)
        .expect("Replication ingest failed");

    // -------------------------------------------------------------------------
    // Phase 10: Multi-Node Convergence & Merkle Lineage Verification
    // -------------------------------------------------------------------------
    assert_eq!(
        replica_engine.current_sequence_tick(),
        primary_engine.current_sequence_tick()
    );
    assert_eq!(
        replica_engine.current_revision(),
        primary_engine.current_revision()
    );
    assert_eq!(
        replica_engine.current_state_root(),
        primary_engine.current_state_root()
    );
    assert_eq!(
        replica_engine.current_transition_root(),
        primary_engine.current_transition_root()
    );

    assert_eq!(
        replica_engine.get_state_value(b"sys:service_status"),
        Some(b"ACTIVE".to_vec())
    );
    assert_eq!(
        replica_engine.get_state_value(
            format!(
                "audit:{}",
                evidence_pkg.execution_response.execution_id.as_str()
            )
            .as_bytes()
        ),
        Some(b"GOVERNED_PASS".to_vec())
    );

    // -------------------------------------------------------------------------
    // Phase 11: Independent Cold-Start Recovery Invariance (ADAM-013)
    // -------------------------------------------------------------------------
    drop(primary_engine);
    drop(replica_engine);

    let primary_reopened = StorageEngine::open(dir_primary.path(), SyncPolicy::FsyncOnCommit)
        .expect("Primary re-open failed");
    let replica_reopened = StorageEngine::open(dir_replica.path(), SyncPolicy::FsyncOnCommit)
        .expect("Replica re-open failed");

    assert_eq!(primary_reopened.current_sequence_tick(), 1);
    assert_eq!(replica_reopened.current_sequence_tick(), 1);
    assert_eq!(
        primary_reopened.current_state_root(),
        replica_reopened.current_state_root()
    );
    assert_eq!(
        primary_reopened.current_transition_root(),
        replica_reopened.current_transition_root()
    );

    assert_eq!(
        primary_reopened.get_state_value(b"sys:service_status"),
        Some(b"ACTIVE".to_vec())
    );
    assert_eq!(
        replica_reopened.get_state_value(b"sys:service_status"),
        Some(b"ACTIVE".to_vec())
    );
}
