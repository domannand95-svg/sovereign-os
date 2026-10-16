// Adapter tests trust caller-supplied authentication results. They do not prove
// cryptographic authentication, durability, or lost-ack safety.
// Fixed times 1000..2000 satisfy the receipt lifetime contract; this API has no clock check.
use sovereign_execution::{
    ExecutionError, ExecutionOutcome, FileCreationAdapter, FileCreationOperation, GovernedExecutor,
};

use sovereign_audit::authorization_receipt::{
    AuthorizationReceipt, IssuerContext, ReceiptAuthenticationResult,
};
use sovereign_audit::governance_admission::{AdmissionDecision, AdmissionOutcome};

use std::fs;
use tempfile::TempDir;

// Helper: build a minimal AdmissionDecision::Permit and an IssuerContext suitable for
// AuthorizationReceipt::generate() in tests. The decision id is reused for intent and
// decision_reference to ensure fixture consistency between admission and receipt.
fn make_permit_decision(base_ref: &str, scope: &str) -> AdmissionDecision {
    AdmissionDecision {
        decision_reference: base_ref.into(),
        intent_reference: base_ref.into(),
        evaluation_reference: format!("{}-eval", base_ref),
        governance_context_reference: format!("{}-ctx", base_ref),
        outcome: AdmissionOutcome::Permit,
        authorized_scope: scope.into(),
        reason_references: vec!["test-policy-allow".into()],
    }
}

fn mock_issuer() -> IssuerContext {
    IssuerContext {
        issuer_reference: "gov_node_alpha".to_string(),
        signing_key_reference: "key_01".to_string(),
    }
}

#[test]
fn test_unauthenticated_rejected_and_sentinel_unchanged() {
    let tmp = TempDir::new().expect("tempdir");
    let target_path = tmp.path().join("unauth.txt");
    let target_str = target_path.to_string_lossy().to_string();

    // Pre-existing sentinel file at the requested destination to ensure unauthenticated
    // rejection does not alter existing bytes.
    let sentinel_bytes = b"sentinel-before-unauth";
    fs::write(&target_path, sentinel_bytes).expect("write sentinel");

    // Build a valid admission + receipt structure (structural only). Reuse the same
    // base id for decision and intent to keep fixtures consistent.
    let base = "ad_unauth";
    let admission = make_permit_decision(base, "test-scope");
    let issuer = mock_issuer();

    let receipt = AuthorizationReceipt::generate(
        &admission,
        "subject-agent-unauth",
        &admission.intent_reference,
        "policy-test",
        &admission.governance_context_reference,
        "file_create",
        &target_str,
        1000,
        2000,
        &issuer,
        "nonce-unauth-001",
    )
    .expect("generate receipt");

    let content = b"payload";
    let operation = FileCreationOperation {
        path: target_str.clone(),
        content_hash: *blake3::hash(content).as_bytes(),
    };

    let adapter = FileCreationAdapter;

    // Supply ReceiptAuthenticationResult::Invalid to simulate unauthenticated receipt.
    // This test treats ReceiptAuthenticationResult::Invalid as a caller-supplied input;
    // it does NOT attempt cryptographic verification here.
    let result = adapter.execute(
        &receipt,
        ReceiptAuthenticationResult::Invalid,
        &operation,
        &operation,
        content,
    );

    assert_eq!(result.unwrap_err(), ExecutionError::Unauthenticated);

    // Ensure sentinel file remains unchanged.
    let read_back = fs::read(&target_path).expect("read sentinel");
    assert_eq!(read_back.as_slice(), sentinel_bytes);
}

#[test]
fn test_operation_mismatch_rejected_absent_requested_and_sentinel_unchanged() {
    // This test covers the rejection when the requested_operation path does not exist.
    let tmp = TempDir::new().expect("tempdir");
    let sentinel_path = tmp.path().join("governed_sentinel.bin");
    let requested_path = tmp.path().join("requested_absent.bin");

    // Create a sentinel file that must remain unchanged on rejection.
    let sentinel_bytes = b"original-bytes";
    fs::write(&sentinel_path, sentinel_bytes).expect("write sentinel");

    let sentinel_str = sentinel_path.to_string_lossy().to_string();
    let requested_str = requested_path.to_string_lossy().to_string();

    let base = "ad_op_mismatch_absent";
    let admission = make_permit_decision(base, "test-scope");
    let issuer = mock_issuer();

    let receipt = AuthorizationReceipt::generate(
        &admission,
        "subject-agent-op-absent",
        &admission.intent_reference,
        "policy-test",
        &admission.governance_context_reference,
        "file_create",
        &sentinel_str,
        1000,
        2000,
        &issuer,
        "nonce-op-absent-001",
    )
    .expect("generate receipt");

    // governed_operation is the sentinel, requested_operation points at a different path (absent).
    let content = b"irrelevant";
    let governed_operation = FileCreationOperation {
        path: sentinel_str.clone(),
        content_hash: *blake3::hash(content).as_bytes(),
    };
    let requested_operation = FileCreationOperation {
        path: requested_str.clone(),
        content_hash: *blake3::hash(content).as_bytes(),
    };

    let adapter = FileCreationAdapter;

    // ReceiptAuthenticationResult::Valid is a trusted test input (test-only).
    let result = adapter.execute(
        &receipt,
        ReceiptAuthenticationResult::Valid,
        &governed_operation,
        &requested_operation,
        content,
    );

    assert_eq!(result.unwrap_err(), ExecutionError::OperationMismatch);

    // Sentinel file must be unchanged.
    let read_back = fs::read(&sentinel_path).expect("read sentinel");
    assert_eq!(read_back.as_slice(), sentinel_bytes);
    // Requested path must not have been created.
    assert!(
        !requested_path.exists(),
        "requested path must not be created on operation mismatch"
    );
}

#[test]
fn test_operation_mismatch_rejected_with_preexisting_requested_sentinel_unchanged() {
    // This test covers the rejection when the requested_operation path already exists.
    let tmp = TempDir::new().expect("tempdir");
    let governed_path = tmp.path().join("governed.bin");
    let requested_path = tmp.path().join("requested_preexists.bin");

    // Create the governed sentinel and the requested pre-existing sentinel.
    let governed_bytes = b"governed-original";
    let requested_bytes = b"requested-original";
    fs::write(&governed_path, governed_bytes).expect("write governed sentinel");
    fs::write(&requested_path, requested_bytes).expect("write requested sentinel");

    let governed_str = governed_path.to_string_lossy().to_string();
    let requested_str = requested_path.to_string_lossy().to_string();

    let base = "ad_op_mismatch_pre";
    let admission = make_permit_decision(base, "test-scope");
    let issuer = mock_issuer();

    let receipt = AuthorizationReceipt::generate(
        &admission,
        "subject-agent-op-pre",
        &admission.intent_reference,
        "policy-test",
        &admission.governance_context_reference,
        "file_create",
        &governed_str,
        1000,
        2000,
        &issuer,
        "nonce-op-pre-001",
    )
    .expect("generate receipt");

    let content = b"irrelevant";
    let governed_operation = FileCreationOperation {
        path: governed_str.clone(),
        content_hash: *blake3::hash(content).as_bytes(),
    };
    let requested_operation = FileCreationOperation {
        path: requested_str.clone(),
        content_hash: *blake3::hash(content).as_bytes(),
    };

    let adapter = FileCreationAdapter;

    let result = adapter.execute(
        &receipt,
        ReceiptAuthenticationResult::Valid,
        &governed_operation,
        &requested_operation,
        content,
    );

    assert_eq!(result.unwrap_err(), ExecutionError::OperationMismatch);

    // Both sentinel files must be unchanged.
    let governed_read = fs::read(&governed_path).expect("read governed sentinel");
    assert_eq!(governed_read.as_slice(), governed_bytes);
    let requested_read = fs::read(&requested_path).expect("read requested sentinel");
    assert_eq!(requested_read.as_slice(), requested_bytes);
}

#[test]
fn test_content_integrity_rejected_and_preexisting_unmodified() {
    let tmp = TempDir::new().expect("tempdir");
    let target_path = tmp.path().join("content_mismatch.txt");
    let target_str = target_path.to_string_lossy().to_string();

    // Create a pre-existing file to ensure content integrity failure does not modify it.
    let pre_bytes = b"pre-existing-bytes";
    fs::write(&target_path, pre_bytes).expect("write preexisting");

    let base = "ad_content_mismatch";
    let admission = make_permit_decision(base, "test-scope");
    let issuer = mock_issuer();

    let receipt = AuthorizationReceipt::generate(
        &admission,
        "subject-agent-content",
        &admission.intent_reference,
        "policy-test",
        &admission.governance_context_reference,
        "file_create",
        &target_str,
        1000,
        2000,
        &issuer,
        "nonce-content-001",
    )
    .expect("generate receipt");

    // expected content hash is for b"expected", but we will pass b"different"
    let expected = b"expected";
    let actual = b"different";

    let operation = FileCreationOperation {
        path: target_str.clone(),
        content_hash: *blake3::hash(expected).as_bytes(),
    };

    let adapter = FileCreationAdapter;

    let result = adapter.execute(
        &receipt,
        ReceiptAuthenticationResult::Valid,
        &operation,
        &operation,
        actual,
    );

    assert_eq!(
        result.unwrap_err(),
        ExecutionError::ContentIntegrityMismatch
    );

    // Ensure pre-existing file was not modified.
    let read_back = fs::read(&target_path).expect("read preexisting");
    assert_eq!(read_back.as_slice(), pre_bytes);
}

#[test]
fn test_successful_creation_and_report_fields() {
    let tmp = TempDir::new().expect("tempdir");
    let target_path = tmp.path().join("success.txt");
    let target_str = target_path.to_string_lossy().to_string();

    let base = "ad_success";
    let admission = make_permit_decision(base, "workspace");
    let issuer = mock_issuer();

    let receipt = AuthorizationReceipt::generate(
        &admission,
        "subject-agent-success",
        &admission.intent_reference,
        "policy-capability-promotion",
        &admission.governance_context_reference,
        "file_create",
        &target_str,
        1000,
        2000,
        &issuer,
        "nonce-success-001",
    )
    .expect("generate receipt");

    let content = b"TRIAL_SUCCESS_BYTES\n";
    let operation = FileCreationOperation {
        path: target_str.clone(),
        content_hash: *blake3::hash(content).as_bytes(),
    };

    let adapter = FileCreationAdapter;

    // Supply ReceiptAuthenticationResult::Valid as a trusted test input (test-only).
    let exec_report = adapter
        .execute(
            &receipt,
            ReceiptAuthenticationResult::Valid,
            &operation,
            &operation,
            content,
        )
        .expect("execution should succeed");

    // Verify report fields supported by the actual API.
    assert_eq!(exec_report.outcome, ExecutionOutcome::Created);
    assert_eq!(exec_report.attempt.outcome, ExecutionOutcome::Created);
    assert_eq!(exec_report.attempt.timestamp, 0);
    assert_eq!(exec_report.attempt.execution_id, "exec-local-001");
    assert_eq!(
        exec_report.attempt.receipt_reference,
        receipt.receipt_reference
    );
    assert_eq!(exec_report.attempt.operation_reference, target_str);

    // Ensure bytes were written exactly.
    let written = fs::read(&target_path).expect("read written file");
    assert_eq!(written.as_slice(), content);
}

#[test]
fn test_unauthenticated_rejected_and_absent_target_not_created() {
    let tmp = TempDir::new().expect("tempdir");
    let target_path = tmp.path().join("unauth.txt");
    let target_str = target_path.to_string_lossy().to_string();

    assert!(!target_path.exists());

    // Build a valid admission + receipt structure (structural only). Reuse the same
    // base id for decision and intent to keep fixtures consistent.
    let base = "ad_unauth";
    let admission = make_permit_decision(base, "test-scope");
    let issuer = mock_issuer();

    let receipt = AuthorizationReceipt::generate(
        &admission,
        "subject-agent-unauth",
        &admission.intent_reference,
        "policy-test",
        &admission.governance_context_reference,
        "file_create",
        &target_str,
        1000,
        2000,
        &issuer,
        "nonce-unauth-001",
    )
    .expect("generate receipt");

    let content = b"payload";
    let operation = FileCreationOperation {
        path: target_str.clone(),
        content_hash: *blake3::hash(content).as_bytes(),
    };

    let adapter = FileCreationAdapter;

    // Supply ReceiptAuthenticationResult::Invalid to simulate unauthenticated receipt.
    // This test treats ReceiptAuthenticationResult::Invalid as a caller-supplied input;
    // it does NOT attempt cryptographic verification here.
    let result = adapter.execute(
        &receipt,
        ReceiptAuthenticationResult::Invalid,
        &operation,
        &operation,
        content,
    );

    assert_eq!(result.unwrap_err(), ExecutionError::Unauthenticated);

    assert!(
        !target_path.exists(),
        "unauthenticated target must remain absent"
    );
}

#[test]
fn test_content_integrity_rejected_and_absent_target_not_created() {
    let tmp = TempDir::new().expect("tempdir");
    let target_path = tmp.path().join("content_mismatch.txt");
    let target_str = target_path.to_string_lossy().to_string();

    assert!(!target_path.exists());

    let base = "ad_content_mismatch";
    let admission = make_permit_decision(base, "test-scope");
    let issuer = mock_issuer();

    let receipt = AuthorizationReceipt::generate(
        &admission,
        "subject-agent-content",
        &admission.intent_reference,
        "policy-test",
        &admission.governance_context_reference,
        "file_create",
        &target_str,
        1000,
        2000,
        &issuer,
        "nonce-content-001",
    )
    .expect("generate receipt");

    // expected content hash is for b"expected", but we will pass b"different"
    let expected = b"expected";
    let actual = b"different";

    let operation = FileCreationOperation {
        path: target_str.clone(),
        content_hash: *blake3::hash(expected).as_bytes(),
    };

    let adapter = FileCreationAdapter;

    let result = adapter.execute(
        &receipt,
        ReceiptAuthenticationResult::Valid,
        &operation,
        &operation,
        actual,
    );

    assert_eq!(
        result.unwrap_err(),
        ExecutionError::ContentIntegrityMismatch
    );

    assert!(
        !target_path.exists(),
        "content mismatch target must remain absent"
    );
}
