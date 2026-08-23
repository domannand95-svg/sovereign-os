//! Boundary Tests for ADAM-011-C
//!
//! Validates criteria C011-001 through C011-010 covering proposal admission,
//! deterministic risk evaluation, relational binding, and anti-replay execution claim acquisition.

use beta001_harness::approval::{ApprovalLevel as CoreApprovalLevel, ApprovalReceipt};
use beta001_harness::service::admission::{
    AdmissionError, ExecutionAdmissionGate, ProposalAdmissionGate,
};
use beta001_harness::service_contract::{
    ApprovalReceiptId, BlastRadius, ClaimState, DispatchState, ExecutionRequest, LifecycleState,
    OutcomeState, ProposalId, ProposalRequest, ProposedOperation, RequiredApprovalLevel, RiskLevel,
    SchemaVersionV1, SessionId, Sha256Digest, UserId,
};
use chrono::Utc;

fn sample_proposal_request(op: ProposedOperation) -> ProposalRequest {
    ProposalRequest {
        schema_version: SchemaVersionV1,
        user_id: UserId::new("usr_alice_001".to_string()).unwrap(),
        session_id: SessionId::new("ses_portal_123".to_string()).unwrap(),
        intent: "Perform routine diagnostic verification".to_string(),
        proposed_operation: op,
        source_evidence_references: vec![Sha256Digest::new(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        )
        .unwrap()],
        timestamp: Utc::now(),
    }
}

#[test]
fn test_c011_001_to_003_proposal_admission_and_deterministic_risk() {
    let gate = ProposalAdmissionGate::new();
    let req = sample_proposal_request(ProposedOperation::RequestStateMutation);

    let res = gate
        .admit_proposal(&req)
        .expect("Proposal admission failed");

    assert_eq!(res.lifecycle_state, LifecycleState::AwaitingApproval);
    assert_eq!(res.risk_context.risk_level, RiskLevel::High);
    assert_eq!(res.risk_context.blast_radius, BlastRadius::System);
    assert_eq!(
        res.required_approval_level,
        RequiredApprovalLevel::Governance
    );
    assert_eq!(res.authority_delta.value(), 0);

    let (proposal, risk_ctx) = gate
        .get_admitted(res.proposal_id.as_str())
        .expect("Admitted proposal not stored");
    assert_eq!(proposal.proposal_id, res.proposal_id.as_str());
    assert_eq!(risk_ctx.context_id, res.risk_context.context_id);
}

#[test]
fn test_c011_proposal_not_found_in_gate() {
    let prop_gate = ProposalAdmissionGate::new();
    let exec_gate = ExecutionAdmissionGate::new();

    let receipt = ApprovalReceipt {
        receipt_id: "apr_valid_123".to_string(),
        proposal_id: "prop_nonexistent_123".to_string(),
        risk_context_id: "risk_ctx_any".to_string(),
        approver_identity: "governance_peer_1".to_string(),
        approval_level: CoreApprovalLevel::Peer,
        timestamp: "2026-08-24T02:05:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "crypto_sig_abc".to_string(),
    };

    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: UserId::new("usr_alice_001".to_string()).unwrap(),
        session_id: SessionId::new("ses_portal_123".to_string()).unwrap(),
        proposal_id: ProposalId::new("prop_nonexistent_123".to_string()).unwrap(),
        approval_receipt_id: ApprovalReceiptId::new("apr_valid_123".to_string()).unwrap(),
        caller_identity: "exec_agent_alpha".to_string(),
        signature: "caller_sig_xyz".to_string(),
        timestamp: Utc::now(),
    };

    let result = exec_gate.admit_execution(&exec_req, &prop_gate, &receipt);
    assert_eq!(result, Err(AdmissionError::ProposalNotFound));
}

#[test]
fn test_c011_004_relational_mismatch_rejected() {
    let prop_gate = ProposalAdmissionGate::new();
    let exec_gate = ExecutionAdmissionGate::new();

    let req = sample_proposal_request(ProposedOperation::EmitNotification);
    let prop_res = prop_gate.admit_proposal(&req).unwrap();
    let (_, risk_ctx) = prop_gate
        .get_admitted(prop_res.proposal_id.as_str())
        .unwrap();

    let receipt = ApprovalReceipt {
        receipt_id: "apr_valid_123".to_string(),
        proposal_id: "prop_other_different_id".to_string(), // Mismatch with proposal
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_peer_1".to_string(),
        approval_level: CoreApprovalLevel::Peer,
        timestamp: "2026-08-24T02:05:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "crypto_sig_abc".to_string(),
    };

    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: UserId::new("usr_alice_001".to_string()).unwrap(),
        session_id: SessionId::new("ses_portal_123".to_string()).unwrap(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new("apr_valid_123".to_string()).unwrap(),
        caller_identity: "exec_agent_alpha".to_string(),
        signature: "caller_sig_xyz".to_string(),
        timestamp: Utc::now(),
    };

    let result = exec_gate.admit_execution(&exec_req, &prop_gate, &receipt);
    assert!(matches!(result, Err(AdmissionError::RelationalMismatch(_))));
}

#[test]
fn test_c011_005_risk_context_mismatch_rejected() {
    let prop_gate = ProposalAdmissionGate::new();
    let exec_gate = ExecutionAdmissionGate::new();

    let req = sample_proposal_request(ProposedOperation::RequestReview);
    let prop_res = prop_gate.admit_proposal(&req).unwrap();

    let receipt = ApprovalReceipt {
        receipt_id: "apr_valid_123".to_string(),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: "risk_ctx_stale_different".to_string(), // Mismatch
        approver_identity: "governance_peer_1".to_string(),
        approval_level: CoreApprovalLevel::Peer,
        timestamp: "2026-08-24T02:05:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "crypto_sig_abc".to_string(),
    };

    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: UserId::new("usr_alice_001".to_string()).unwrap(),
        session_id: SessionId::new("ses_portal_123".to_string()).unwrap(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new("apr_valid_123".to_string()).unwrap(),
        caller_identity: "exec_agent_alpha".to_string(),
        signature: "caller_sig_xyz".to_string(),
        timestamp: Utc::now(),
    };

    let result = exec_gate.admit_execution(&exec_req, &prop_gate, &receipt);
    assert_eq!(result, Err(AdmissionError::RiskContextMismatch));
}

#[test]
fn test_c011_006_insufficient_approval_scope_rejected() {
    let prop_gate = ProposalAdmissionGate::new();
    let exec_gate = ExecutionAdmissionGate::new();

    // High risk (RequestStateMutation) requires Governance approval
    let req = sample_proposal_request(ProposedOperation::RequestStateMutation);
    let prop_res = prop_gate.admit_proposal(&req).unwrap();
    let (_, risk_ctx) = prop_gate
        .get_admitted(prop_res.proposal_id.as_str())
        .unwrap();

    let receipt = ApprovalReceipt {
        receipt_id: "apr_valid_123".to_string(),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_peer_1".to_string(),
        approval_level: CoreApprovalLevel::Peer, // Insufficient for High Risk
        timestamp: "2026-08-24T02:05:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "crypto_sig_abc".to_string(),
    };

    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: UserId::new("usr_alice_001".to_string()).unwrap(),
        session_id: SessionId::new("ses_portal_123".to_string()).unwrap(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new("apr_valid_123".to_string()).unwrap(),
        caller_identity: "exec_agent_alpha".to_string(),
        signature: "caller_sig_xyz".to_string(),
        timestamp: Utc::now(),
    };

    let result = exec_gate.admit_execution(&exec_req, &prop_gate, &receipt);
    assert_eq!(result, Err(AdmissionError::InsufficientApprovalScope));
}

#[test]
fn test_c011_007_to_010_atomic_claim_and_anti_replay_lockout() {
    let prop_gate = ProposalAdmissionGate::new();
    let exec_gate = ExecutionAdmissionGate::new();

    let req = sample_proposal_request(ProposedOperation::RequestApproval);
    let prop_res = prop_gate.admit_proposal(&req).unwrap();
    let (_, risk_ctx) = prop_gate
        .get_admitted(prop_res.proposal_id.as_str())
        .unwrap();

    let receipt = ApprovalReceipt {
        receipt_id: "apr_valid_123".to_string(),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_op_1".to_string(),
        approval_level: CoreApprovalLevel::Operator,
        timestamp: "2026-08-24T02:05:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "crypto_sig_abc".to_string(),
    };

    let exec_req = ExecutionRequest {
        schema_version: SchemaVersionV1,
        user_id: UserId::new("usr_alice_001".to_string()).unwrap(),
        session_id: SessionId::new("ses_portal_123".to_string()).unwrap(),
        proposal_id: prop_res.proposal_id.clone(),
        approval_receipt_id: ApprovalReceiptId::new("apr_valid_123".to_string()).unwrap(),
        caller_identity: "exec_agent_alpha".to_string(),
        signature: "caller_sig_xyz".to_string(),
        timestamp: Utc::now(),
    };

    // First execution claim must succeed with pre-dispatch state
    let exec_res = exec_gate
        .admit_execution(&exec_req, &prop_gate, &receipt)
        .expect("Execution admission failed");

    assert!(exec_res.execution_id.as_str().starts_with("exe_"));
    assert_eq!(exec_res.claim_state, ClaimState::ExecutionClaimed);
    assert_eq!(exec_res.dispatch_state, DispatchState::NotDispatched);
    assert_eq!(exec_res.outcome_state, OutcomeState::Pending);
    assert_eq!(exec_res.authority_delta.value(), 0);

    // Second execution claim on identical authorization binding must fail closed
    let replay_result = exec_gate.admit_execution(&exec_req, &prop_gate, &receipt);
    assert_eq!(replay_result, Err(AdmissionError::ReplayDetected));

    // Duplicate proposal intake replay must also fail closed
    let prop_replay = prop_gate.admit_proposal(&req);
    assert_eq!(prop_replay, Err(AdmissionError::ReplayDetected));
}

#[test]
fn test_c011_anti_poisoning_on_validation_failure() {
    let prop_gate = ProposalAdmissionGate::new();

    let mut invalid_req = sample_proposal_request(ProposedOperation::RequestReview);
    invalid_req.source_evidence_references = vec![]; // Triggers ProposalValidationError::EmptyEvidence

    let first_attempt = prop_gate.admit_proposal(&invalid_req);
    assert!(matches!(
        first_attempt,
        Err(AdmissionError::ProposalValidationFailed(_))
    ));

    // Fix the evidence on the same request payload; admission must proceed and NOT trigger ReplayDetected
    invalid_req.source_evidence_references = vec![Sha256Digest::new(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
    )
    .unwrap()];
    let second_attempt = prop_gate.admit_proposal(&invalid_req);
    assert!(
        second_attempt.is_ok(),
        "Replay set was poisoned by previous failed validation attempt"
    );
}
