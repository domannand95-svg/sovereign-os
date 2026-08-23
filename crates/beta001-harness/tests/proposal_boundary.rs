//! Negative Boundary Tests for ADAM-010-A
//! 
//! Proves that the Governed Action Proposal strictly enforces data contracts 
//! and refuses invalid or execution-escalating states.

use beta001_harness::proposal::{GovernedActionProposal, ProposedOperation, ProposalValidationError};

fn valid_base_proposal() -> GovernedActionProposal {
    GovernedActionProposal {
        schema_version: "v1.0.0".to_string(),
        proposal_id: "prop-010-a-001".to_string(),
        proposer_identity: "evaluated-intelligence-source".to_string(),
        source_evidence_references: vec!["evidence-digest-001".to_string()],
        intent: "Propose routine diagnostic scan".to_string(),
        proposed_operation: ProposedOperation::RequestReview,
        risk_evaluation_reference: None,
        created_timestamp: "2026-08-24T01:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        provenance_digest: "provenance-hash-xyz".to_string(),
    }
}

#[test]
fn test_p010_a_001_missing_proposal_identity_rejected() {
    let mut prop = valid_base_proposal();
    prop.proposer_identity = "".to_string();
    assert_eq!(prop.validate(), Err(ProposalValidationError::InvalidIdentity));
}

#[test]
fn test_p010_a_002_missing_evidence_provenance_rejected() {
    let mut prop = valid_base_proposal();
    prop.source_evidence_references.clear();
    assert_eq!(prop.validate(), Err(ProposalValidationError::MissingEvidence));
}

#[test]
fn test_p010_a_003_unknown_operation_type_rejected() {
    // Proves that deserialization fails entirely if the operation is outside the constrained vocabulary
    let json_payload = r#"{
        "schema_version": "v1.0.0",
        "proposal_id": "prop-010-a-invalid",
        "proposer_identity": "rogue-agent",
        "source_evidence_references": ["hash"],
        "intent": "Perform unauthorized action",
        "proposed_operation": "ExecuteDirectMutation",
        "created_timestamp": "2026-08-24T01:00:00Z",
        "expiration_timestamp": "2099-12-31T23:59:59Z",
        "provenance_digest": "hash"
    }"#;
    
    let result: Result<GovernedActionProposal, _> = serde_json::from_str(json_payload);
    assert!(result.is_err(), "Proposal with unknown operation type must fail strictly at the Serde boundary");
}

#[test]
fn test_p010_a_004_expired_proposal_rejected() {
    let mut prop = valid_base_proposal();
    prop.expiration_timestamp = "2020-01-01T00:00:00Z".to_string(); // Date in the past
    assert_eq!(prop.validate(), Err(ProposalValidationError::ExpiredProposal));
}

#[test]
fn test_p010_a_005_proposal_containing_execution_authority_rejected() {
    let mut prop = valid_base_proposal();
    prop.intent = "Attempt to grant_authority to bypass policy constraints".to_string();
    assert_eq!(prop.validate(), Err(ProposalValidationError::ContainsExecutionAuthority));
}