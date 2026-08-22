use tempfile::tempdir;

use sovereign_audit::authorization_receipt::{
    AuthorizationReceipt,
    ReceiptAuthenticationResult,
};

use sovereign_execution::{
    ExecutionError,
    ExecutionOutcome,
    FileCreationAdapter,
    FileCreationOperation,
    GovernedExecutor,
};

fn test_content() -> &'static [u8] {
    b"approved content"
}

fn test_operation() -> FileCreationOperation {
    FileCreationOperation {
        path: "archive/test.md".to_string(),
        content_hash: *blake3::hash(test_content()).as_bytes(),
    }
}

fn test_receipt() -> AuthorizationReceipt {
    AuthorizationReceipt {
        receipt_reference: "receipt-001".into(),
        subject_reference: "subject-001".into(),
        intent_reference: "intent-001".into(),
        admission_reference: "admission-001".into(),
        policy_reference: "policy-001".into(),
        governance_context_reference: "context-001".into(),
        authorized_operation: "file_create".into(),
        authorized_target: "archive/test.md".into(),
        authorized_scope: "archive".into(),
        constraints: vec![],
        issued_at: 1000,
        expires_at: 2000,
        revocation_reference: "none".into(),
        issuer_reference: "issuer-001".into(),
        nonce: "nonce-001".into(),
        signature: "signature".into(),
    }
}

#[test]
fn valid_authentication_is_accepted() {
    let adapter = FileCreationAdapter;

    let dir = tempdir().unwrap();
    let path = dir.path().join("test.md");

    let operation = FileCreationOperation {
        path: path.to_string_lossy().to_string(),
        content_hash: *blake3::hash(test_content()).as_bytes(),
    };

    let result = adapter.execute(
        &test_receipt(),
        ReceiptAuthenticationResult::Valid,
        &operation,
        &operation,
        test_content(),
    );

    assert_eq!(result.unwrap().outcome, ExecutionOutcome::Created);
    assert!(path.exists());
}

#[test]
fn invalid_authentication_is_rejected() {
    let adapter = FileCreationAdapter;

    let operation = test_operation();

    let result = adapter.execute(
        &test_receipt(),
        ReceiptAuthenticationResult::Invalid,
        &operation,
        &operation,
        test_content(),
    );

    assert_eq!(result, Err(ExecutionError::Unauthenticated));
}

#[test]
fn matching_operation_is_accepted() {
    let adapter = FileCreationAdapter;

    let dir = tempdir().unwrap();
    let path = dir.path().join("test.md");

    let operation = FileCreationOperation {
        path: path.to_string_lossy().to_string(),
        content_hash: *blake3::hash(test_content()).as_bytes(),
    };

    let result = adapter.execute(
        &test_receipt(),
        ReceiptAuthenticationResult::Valid,
        &operation,
        &operation,
        test_content(),
    );

    assert_eq!(result.unwrap().outcome, ExecutionOutcome::Created);
    assert!(path.exists());
}

#[test]
fn mutated_operation_is_rejected() {
    let adapter = FileCreationAdapter;

    let governed = test_operation();

    let requested = FileCreationOperation {
        path: "archive/other.md".to_string(),
        content_hash: governed.content_hash,
    };

    let result = adapter.execute(
        &test_receipt(),
        ReceiptAuthenticationResult::Valid,
        &governed,
        &requested,
        test_content(),
    );

    assert_eq!(result, Err(ExecutionError::OperationMismatch));
}

#[test]
fn content_hash_match_is_valid() {
    let operation = test_operation();

    assert!(operation.verify_content(test_content()));
}

#[test]
fn content_hash_mismatch_is_rejected() {
    let operation = test_operation();

    assert!(!operation.verify_content(b"modified content"));
}



