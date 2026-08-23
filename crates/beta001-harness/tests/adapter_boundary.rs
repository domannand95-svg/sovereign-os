//! Negative Boundary Tests for ADAM-010-D
//! 
//! Proves that the Execution Adapter Gate strictly enforces integrated governance 
//! proofs and refuses bypassed, replayed, or scope-mismatched actions.

use beta001_harness::proposal::{GovernedActionProposal, ProposedOperation};
use beta001_harness::risk::{RiskEvaluator, RiskEvaluationContext};
use beta001_harness::approval::{ApprovalReceipt, ApprovalLevel, ApprovalValidationError};
use beta001_harness::adapter::{ExecutionAdapterGate, AdapterIsolationError};

fn setup_fixtures(op: ProposedOperation, approval_level: ApprovalLevel) -> (GovernedActionProposal, RiskEvaluationContext, ApprovalReceipt) {
    let prop = GovernedActionProposal {
        schema_version: "v1.0.0".to_string(),
        proposal_id: "prop-010-d-001".to_string(),
        proposer_identity: "evaluated-intelligence-source".to_string(),
        source_evidence_references: vec!["evidence-1".to_string()],
        intent: "Execute standard workload".to_string(),
        proposed_operation: op,
        risk_evaluation_reference: None,
        created_timestamp: "2026-08-24T01:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        provenance_digest: "digest".to_string(),
    };

    let ctx = RiskEvaluator::evaluate_proposal(&prop);

    let receipt = ApprovalReceipt {
        receipt_id: "receipt-001".to_string(),
        proposal_id: prop.proposal_id.clone(),
        risk_context_id: ctx.context_id.clone(),
        approver_identity: "governance-node".to_string(),
        approval_level,
        timestamp: "2026-08-24T01:05:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "valid-signature".to_string(),
    };

    (prop, ctx, receipt)
}

#[test]
fn test_e010_d_001_missing_or_invalid_approval_rejected() {
    let (prop, ctx, mut receipt) = setup_fixtures(ProposedOperation::EmitNotification, ApprovalLevel::Peer);
    receipt.signature = "unsigned".to_string(); // Invalid signature

    let gate = ExecutionAdapterGate::new();
    assert_eq!(
        gate.verify_execution_readiness(&prop, &ctx, &receipt),
        Err(AdapterIsolationError::MissingOrInvalidApproval(ApprovalValidationError::InvalidSignature))
    );
}

#[test]
fn test_e010_d_002_wrong_scope_rejected() {
    // High risk (RequestStateMutation) structurally maps to System BlastRadius and requires Governance approval. 
    // We attempt to bypass by providing Peer approval.
    let (prop, ctx, receipt) = setup_fixtures(ProposedOperation::RequestStateMutation, ApprovalLevel::Peer);

    let gate = ExecutionAdapterGate::new();
    assert_eq!(
        gate.verify_execution_readiness(&prop, &ctx, &receipt),
        Err(AdapterIsolationError::MissingOrInvalidApproval(ApprovalValidationError::InsufficientApprovalScope))
    );
}

#[test]
fn test_e010_d_003_replay_rejected() {
    let (prop, ctx, receipt) = setup_fixtures(ProposedOperation::RequestReview, ApprovalLevel::Operator);

    let gate = ExecutionAdapterGate::new();
    
    // First execution authorization attempt must succeed and burn the token
    assert_eq!(gate.verify_execution_readiness(&prop, &ctx, &receipt), Ok(()));
    
    // Immediate replay of the identical evidence package must be rejected
    assert_eq!(
        gate.verify_execution_readiness(&prop, &ctx, &receipt),
        Err(AdapterIsolationError::ReplayDetected)
    );
}

#[test]
fn test_e010_d_004_proposal_mismatch_rejected() {
    let (prop, ctx, mut receipt) = setup_fixtures(ProposedOperation::EmitNotification, ApprovalLevel::Peer);
    
    // Receipt points to an alternate proposal identifier
    receipt.proposal_id = "rogue-proposal-xyz".to_string();

    let gate = ExecutionAdapterGate::new();
    assert_eq!(
        gate.verify_execution_readiness(&prop, &ctx, &receipt),
        Err(AdapterIsolationError::ProposalMismatch)
    );
}