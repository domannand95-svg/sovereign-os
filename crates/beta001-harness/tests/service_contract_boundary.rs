//! ADAM-011-B: Typed Service Contract Boundary Tests
//!
//! Falsifies B011-001 through B011-012.

use beta001_harness::service_contract::{
    ApprovalReceiptId, DispatchState, ExecutionRequest, ExecutionResponse, InferenceGateway,
    OutcomeState, ProposalId, ProposalRequest, ProposalResponse, SessionId, Sha256Digest, UserId,
};
use serde_json::{json, Value};
use std::any::TypeId;

fn digest() -> &'static str {
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
}

fn response_digest() -> &'static str {
    "ca978112ca1bbdcaf064278e4a1f94f0ec19f70604b0e9cda0704f026f20c37c"
}

fn seal_digest() -> &'static str {
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
}

fn proposal_request() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "user_id": "usr_alpha_123",
        "session_id": "ses_beta_456",
        "intent": "Request diagnostic report generation",
        "proposed_operation": "RequestReview",
        "source_evidence_references": [digest()],
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

fn proposal_response() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "proposal_id": "prop_alpha_789",
        "session_id": "ses_beta_456",
        "lifecycle_state": "AWAITING_APPROVAL",
        "risk_context": {
            "context_id": "risk_ctx_001",
            "risk_level": "LOW",
            "blast_radius": "ISOLATED"
        },
        "required_approval_level": "PEER",
        "authority_delta": 0,
        "provenance_digest": digest(),
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

fn execution_request() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "user_id": "usr_alpha_123",
        "session_id": "ses_beta_456",
        "proposal_id": "prop_alpha_789",
        "approval_receipt_id": "apr_gamma_321",
        "caller_identity": "governance_agent_01",
        "signature": "sig_ed25519_valid",
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

fn execution_response() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "execution_id": "exe_omega_999",
        "proposal_id": "prop_alpha_789",
        "approval_receipt_id": "apr_gamma_321",
        "claim_state": "EXECUTION_CLAIMED",
        "dispatch_state": "DISPATCHED",
        "outcome_state": "SUCCEEDED",
        "authority_delta": 0,
        "evidence_package_digest": digest(),
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

fn inference_gateway() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "session_id": "ses_beta_456",
        "provider_identity": "gemini-flash",
        "adapter_identity": "adapter_v1",
        "request_digest": digest(),
        "response_digest": response_digest(),
        "transport_status": "SUCCESS",
        "raw_model_output": "{\"intent\":\"review\"}",
        "evidence_seal_digest": seal_digest(),
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

// B011-001
#[test]
fn test_b011_001_all_five_contracts_hydrate() {
    assert!(serde_json::from_value::<ProposalRequest>(proposal_request()).is_ok());
    assert!(serde_json::from_value::<ProposalResponse>(proposal_response()).is_ok());
    assert!(serde_json::from_value::<ExecutionRequest>(execution_request()).is_ok());
    assert!(serde_json::from_value::<ExecutionResponse>(execution_response()).is_ok());
    assert!(serde_json::from_value::<InferenceGateway>(inference_gateway()).is_ok());
}

// B011-002
#[test]
fn test_b011_002_unknown_fields_are_rejected() {
    let mut proposal = proposal_request();
    proposal["unauthorized_capability"] = json!("root");
    assert!(serde_json::from_value::<ProposalRequest>(proposal).is_err());

    let mut response = proposal_response();
    response["grant"] = json!(true);
    assert!(serde_json::from_value::<ProposalResponse>(response).is_err());

    let mut execution = execution_request();
    execution["bypass"] = json!(true);
    assert!(serde_json::from_value::<ExecutionRequest>(execution).is_err());

    let mut receipt = execution_response();
    receipt["capability"] = json!("admin");
    assert!(serde_json::from_value::<ExecutionResponse>(receipt).is_err());

    let mut inference = inference_gateway();
    inference["authorized"] = json!(true);
    assert!(serde_json::from_value::<InferenceGateway>(inference).is_err());
}

// B011-003
#[test]
fn test_b011_003_identifier_namespaces_are_distinct_types() {
    assert_ne!(TypeId::of::<UserId>(), TypeId::of::<SessionId>());
    assert_ne!(
        TypeId::of::<ProposalId>(),
        TypeId::of::<ApprovalReceiptId>()
    );

    assert!(serde_json::from_value::<UserId>(json!("usr_alpha")).is_ok());
    assert!(serde_json::from_value::<SessionId>(json!("ses_alpha")).is_ok());

    assert!(serde_json::from_value::<UserId>(json!("ses_alpha")).is_err());
    assert!(serde_json::from_value::<ProposalId>(json!("apr_alpha")).is_err());
}

// B011-004
#[test]
fn test_b011_004_proposal_request_is_inert() {
    let mut payload = proposal_request();
    payload["execute"] = json!(true);

    assert!(serde_json::from_value::<ProposalRequest>(payload).is_err());
}

// B011-005
#[test]
fn test_b011_005_execution_requires_proposal_and_approval_identity() {
    let mut missing_proposal = execution_request();
    missing_proposal
        .as_object_mut()
        .unwrap()
        .remove("proposal_id");

    assert!(serde_json::from_value::<ExecutionRequest>(missing_proposal).is_err());

    let mut missing_approval = execution_request();
    missing_approval
        .as_object_mut()
        .unwrap()
        .remove("approval_receipt_id");

    assert!(serde_json::from_value::<ExecutionRequest>(missing_approval).is_err());
}

// B011-006
#[test]
fn test_b011_006_execution_state_triad_is_preserved() {
    let hydrated = serde_json::from_value::<ExecutionResponse>(execution_response()).unwrap();

    assert_eq!(hydrated.dispatch_state, DispatchState::Dispatched);
    assert_eq!(hydrated.outcome_state, OutcomeState::Succeeded);
}

// B011-007
#[test]
fn test_b011_007_illegal_dispatch_outcome_combinations_fail_hydration() {
    for outcome in ["SUCCEEDED", "FAILED", "OUTCOME_UNKNOWN"] {
        let mut payload = execution_response();
        payload["dispatch_state"] = json!("NOT_DISPATCHED");
        payload["outcome_state"] = json!(outcome);

        assert!(
            serde_json::from_value::<ExecutionResponse>(payload).is_err(),
            "NOT_DISPATCHED + {outcome} must fail"
        );
    }

    let mut valid_pending = execution_response();
    valid_pending["dispatch_state"] = json!("NOT_DISPATCHED");
    valid_pending["outcome_state"] = json!("PENDING");

    assert!(serde_json::from_value::<ExecutionResponse>(valid_pending).is_ok());

    for outcome in ["PENDING", "SUCCEEDED", "FAILED", "OUTCOME_UNKNOWN"] {
        let mut payload = execution_response();
        payload["dispatch_state"] = json!("DISPATCHED");
        payload["outcome_state"] = json!(outcome);

        assert!(
            serde_json::from_value::<ExecutionResponse>(payload).is_ok(),
            "DISPATCHED + {outcome} must hydrate"
        );
    }
}

// B011-008
#[test]
fn test_b011_008_success_inference_requires_response_evidence() {
    let mut missing_digest = inference_gateway();
    missing_digest["response_digest"] = json!(null);

    assert!(serde_json::from_value::<InferenceGateway>(missing_digest).is_err());

    let mut missing_output = inference_gateway();
    missing_output["raw_model_output"] = json!(null);

    assert!(serde_json::from_value::<InferenceGateway>(missing_output).is_err());
}

// B011-009
#[test]
fn test_b011_009_non_success_inference_cannot_carry_model_output() {
    for state in ["FAILED", "TIMEOUT", "DISCONNECTED"] {
        let mut valid = inference_gateway();
        valid["transport_status"] = json!(state);
        valid["response_digest"] = json!(null);
        valid["raw_model_output"] = json!(null);

        assert!(
            serde_json::from_value::<InferenceGateway>(valid).is_ok(),
            "{state} with null evidence must hydrate"
        );

        let mut smuggled = inference_gateway();
        smuggled["transport_status"] = json!(state);

        assert!(
            serde_json::from_value::<InferenceGateway>(smuggled).is_err(),
            "{state} must reject response evidence"
        );
    }
}

// B011-010
#[test]
fn test_b011_010_sha256_is_canonical_lowercase_hex() {
    assert!(serde_json::from_value::<Sha256Digest>(json!(digest())).is_ok());

    assert!(serde_json::from_value::<Sha256Digest>(json!(
        "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855"
    ))
    .is_err());

    assert!(serde_json::from_value::<Sha256Digest>(json!("e3b0c442")).is_err());
}

// B011-011
#[test]
fn test_b011_011_cross_plane_identifier_substitution_fails() {
    let mut swapped_proposal = execution_request();
    swapped_proposal["proposal_id"] = json!("apr_gamma_321");

    assert!(serde_json::from_value::<ExecutionRequest>(swapped_proposal).is_err());

    let mut swapped_approval = execution_request();
    swapped_approval["approval_receipt_id"] = json!("prop_alpha_789");

    assert!(serde_json::from_value::<ExecutionRequest>(swapped_approval).is_err());

    let mut swapped_user = execution_request();
    swapped_user["user_id"] = json!("ses_beta_456");

    assert!(serde_json::from_value::<ExecutionRequest>(swapped_user).is_err());
}

// B011-012
#[test]
fn test_b011_012_authority_delta_is_structurally_zero() {
    let mut proposal = proposal_response();
    proposal["authority_delta"] = json!(1);

    assert!(serde_json::from_value::<ProposalResponse>(proposal).is_err());

    let mut execution = execution_response();
    execution["authority_delta"] = json!(-1);

    assert!(serde_json::from_value::<ExecutionResponse>(execution).is_err());
}
