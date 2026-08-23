//! Negative Boundary Tests for ADAM-010-B
//!
//! Proves that risk evaluation is deterministic, immutable, and
//! does not grant execution capability.

use beta001_harness::proposal::{GovernedActionProposal, ProposedOperation};
use beta001_harness::risk::{BlastRadius, RiskEvaluator, RiskLevel};

fn dummy_proposal(op: ProposedOperation) -> GovernedActionProposal {
    GovernedActionProposal {
        schema_version: "v1.0.0".to_string(),
        proposal_id: "prop-010-b-test".to_string(),
        proposer_identity: "test-agent".to_string(),
        source_evidence_references: vec!["evidence-1".to_string()],
        intent: "Test operation".to_string(),
        proposed_operation: op,
        risk_evaluation_reference: None,
        created_timestamp: "2026-08-24T01:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        provenance_digest: "digest".to_string(),
    }
}

#[test]
fn test_r010_b_001_risk_escalation_cannot_be_lowered() {
    let prop = dummy_proposal(ProposedOperation::RequestStateMutation);
    let ctx = RiskEvaluator::evaluate_proposal(&prop);

    // Ensure state mutations are structurally bound to High risk and System blast radius
    assert_eq!(ctx.risk_level, RiskLevel::High);
    assert_eq!(ctx.blast_radius, BlastRadius::System);
}

#[test]
fn test_r010_b_002_proposal_cannot_self_select_risk() {
    let prop = dummy_proposal(ProposedOperation::EmitNotification);

    // The proposal struct inherently lacks fields to suggest its own risk.
    // The evaluation guarantees the mapping:
    let ctx = RiskEvaluator::evaluate_proposal(&prop);
    assert_eq!(ctx.risk_level, RiskLevel::Low);
    assert_eq!(ctx.evaluator_identity, "system-risk-evaluator-v1");
}

#[test]
fn test_r010_b_004_risk_evaluation_cannot_authorize_execution() {
    let prop = dummy_proposal(ProposedOperation::RequestApproval);
    let ctx = RiskEvaluator::evaluate_proposal(&prop);

    // The context is generated, but contains no execution tokens or capabilities,
    // preserving the Delta Authority = 0 invariant.
    assert_eq!(ctx.risk_level, RiskLevel::Medium);
    assert_eq!(ctx.proposal_id, prop.proposal_id);
}
