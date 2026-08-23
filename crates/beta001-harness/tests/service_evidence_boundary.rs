//! Boundary Tests for ADAM-011-F
//!
//! Validates criteria F011-001 through F011-007 covering end-to-end evidence aggregation,
//! domain-separated package sealing, relational integrity, digest recomputation, and anti-tamper falsification.

use chrono::Utc;
use beta001_harness::service::client::{ClientSanitizer, AuthenticatedClientContext};
use beta001_harness::service::admission::{ProposalAdmissionGate, ExecutionAdmissionGate};
use beta001_harness::service::inference::{InferenceGatewayAdapter, RecordedTransportOutcome};
use beta001_harness::service::evidence::{ServiceEvidencePackage, EvidenceClosureError, SERVICE_EVIDENCE_DOMAIN_TAG};
use beta001_harness::service_contract::{
    ProposalRequest, ExecutionRequest, ProposedOperation, UserId, SessionId,
    ApprovalReceiptId, Sha256Digest, SchemaVersionV1,
};
use beta001_harness::approval::{ApprovalReceipt, ApprovalLevel};

fn setup_end_to_end_fixtures() -> (
    beta001_harness::service::client::SanitizedProposalRequest,
    beta001_harness::service_contract::ProposalResponse,
    beta001_harness::risk::RiskEvaluationContext,
    ApprovalReceipt,
    beta001_harness::service_contract::ExecutionResponse,
    Vec<beta001_harness::service_contract::InferenceGateway>,
) {
    let now = Utc::now();
    let auth_ctx = AuthenticatedClientContext {
        authenticated_user_id: UserId::new("usr_alice_001".to_string()).unwrap(),
        authenticated_session_id: SessionId::new("ses_portal_123".to_string()).unwrap(),
    };

    let raw_req = ProposalRequest {
        schema_version: SchemaVersionV1,
        user_id: auth_ctx.authenticated_user_id.clone(),
        session_id: auth_ctx.authenticated_session_id.clone(),
        intent: "Execute controlled diagnostic query".to_string(),
        proposed_operation: ProposedOperation::RequestReview,
        source_evidence_references: vec![
            Sha256Digest::new("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string()).unwrap(),
        ],
        timestamp: now,
    };

    // 011-D Sanitized Ingress
    let sanitized_ingress = ClientSanitizer::sanitize_proposal_request(raw_req.clone(), &auth_ctx, now)
        .expect("Sanitization failed");

    // 011-C Proposal Admission
    let prop_gate = ProposalAdmissionGate::new();
    let prop_res = prop_gate.admit_proposal(&raw_req).expect("Proposal admission failed");
    let (_, risk_ctx) = prop_gate.get_admitted(prop_res.proposal_id.as_str()).unwrap();

    // 010 Approval Receipt
    let receipt = ApprovalReceipt {
        receipt_id: "apr_gov_001".to_string(),
        proposal_id: prop_res.proposal_id.as_str().to_string(),
        risk_context_id: risk_ctx.context_id.clone(),
        approver_identity: "governance_peer_1".to_string(),
        approval_level: ApprovalLevel::Peer,
        timestamp: "2026-08-24T02:05:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        signature: "crypto_sig_abc".to_string(),
    };

    // 011-C Execution Admission
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
    let exec_res = exec_gate.admit_execution(&exec_req, &prop_gate, &receipt)
        .expect("Execution claim failed");

    // 011-E Inference Records
    let inf1 = InferenceGatewayAdapter::record_transport(
        auth_ctx.authenticated_session_id.clone(),
        "gemini-2.5-pro",
        "adapter_v1",
        "Prompt: analyze risk",
        RecordedTransportOutcome::Success {
            raw_model_output: "Analysis: Low risk.".to_string(),
        },
        now,
    ).expect("Inference capture 1 failed");

    let inf2 = InferenceGatewayAdapter::record_transport(
        auth_ctx.authenticated_session_id.clone(),
        "gemini-2.5-pro",
        "adapter_v1",
        "Prompt: finalize report",
        RecordedTransportOutcome::Success {
            raw_model_output: "Report finalized.".to_string(),
        },
        now,
    ).expect("Inference capture 2 failed");

    (sanitized_ingress, prop_res, risk_ctx, receipt, exec_res, vec![inf1, inf2])
}

#[test]
fn test_f011_001_and_002_build_package_and_verify_seal() {
    let (sanitized, prop, risk, receipt, exec, inferences) = setup_end_to_end_fixtures();
    let closed_time = Utc::now();

    let package = ServiceEvidencePackage::build(
        sanitized,
        prop,
        risk,
        receipt,
        exec,
        inferences,
        closed_time,
    ).expect("Service evidence package build failed");

    assert_eq!(package.closed_timestamp, closed_time);
    assert_eq!(SERVICE_EVIDENCE_DOMAIN_TAG, "SOVEREIGN_OS:SERVICE_EVIDENCE:v1");
    assert!(package.verify_chain().is_ok());
}

#[test]
fn test_f011_003_relational_mismatches_fail_verification() {
    let (sanitized, prop, risk, receipt, exec, inferences) = setup_end_to_end_fixtures();
    let closed_time = Utc::now();

    let mut bad_risk = risk.clone();
    bad_risk.proposal_id = "prop_forged_mismatched".to_string();

    let result = ServiceEvidencePackage::build(
        sanitized,
        prop,
        bad_risk,
        receipt,
        exec,
        inferences,
        closed_time,
    );

    assert!(matches!(result, Err(EvidenceClosureError::RelationalMismatch(_))));
}

#[test]
fn test_f011_004_raw_intent_tampering_fails_verification() {
    let (mut sanitized, prop, risk, receipt, exec, inferences) = setup_end_to_end_fixtures();
    let closed_time = Utc::now();

    // Tamper original raw intent string without updating raw_intent_digest
    sanitized.original_request.intent = "Tampered unauthorized intent".to_string();

    let result = ServiceEvidencePackage::build(
        sanitized,
        prop,
        risk,
        receipt,
        exec,
        inferences,
        closed_time,
    );

    assert!(matches!(result, Err(EvidenceClosureError::IntegrityViolation(_))));
}

#[test]
fn test_f011_004_inference_output_tampering_fails_verification() {
    let (sanitized, prop, risk, receipt, exec, mut inferences) = setup_end_to_end_fixtures();
    let closed_time = Utc::now();

    // Tamper raw model output in first inference envelope
    inferences[0].raw_model_output = Some("Tampered model output".to_string());

    let result = ServiceEvidencePackage::build(
        sanitized,
        prop,
        risk,
        receipt,
        exec,
        inferences,
        closed_time,
    );

    assert!(matches!(result, Err(EvidenceClosureError::IntegrityViolation(_))));
}

#[test]
fn test_f011_005_zero_inference_records_builds_and_verifies() {
    let (sanitized, prop, risk, receipt, exec, _) = setup_end_to_end_fixtures();
    let closed_time = Utc::now();

    let package = ServiceEvidencePackage::build(
        sanitized,
        prop,
        risk,
        receipt,
        exec,
        vec![], // inference_count = 0
        closed_time,
    ).expect("Zero-inference evidence package build failed");

    assert_eq!(package.inference_records.len(), 0);
    assert!(package.verify_chain().is_ok());
}

#[test]
fn test_f011_005_inference_permutation_fails_package_verification() {
    let (sanitized, prop, risk, receipt, exec, inferences) = setup_end_to_end_fixtures();
    let closed_time = Utc::now();

    let mut package = ServiceEvidencePackage::build(
        sanitized,
        prop,
        risk,
        receipt,
        exec,
        inferences,
        closed_time,
    ).unwrap();

    // Swap inference record ordering post-sealing
    package.inference_records.swap(0, 1);

    // Chain verification must detect permutation and fail closed
    let verify_result = package.verify_chain();
    assert!(matches!(verify_result, Err(EvidenceClosureError::IntegrityViolation(_))));
}

#[test]
fn test_f011_007_authority_invariance() {
    let (sanitized, prop, risk, receipt, exec, inferences) = setup_end_to_end_fixtures();
    let closed_time = Utc::now();

    let package = ServiceEvidencePackage::build(
        sanitized,
        prop,
        risk,
        receipt,
        exec,
        inferences,
        closed_time,
    ).unwrap();

    assert_eq!(package.proposal_response.authority_delta.value(), 0);
    assert_eq!(package.execution_response.authority_delta.value(), 0);
}