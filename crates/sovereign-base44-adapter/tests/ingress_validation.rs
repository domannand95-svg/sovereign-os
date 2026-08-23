//! Ingress validation integration tests.

extern crate sovereign_base44_adapter;

use sha2::Digest;
use sovereign_base44_adapter::{error::Base44AdapterError, validation::IngressValidator};

#[test]
fn test_valid_ingress_request() {
    let request_id = "req-001";
    let receipt_reference = "a".repeat(64);
    let operation = "file.create";
    let target = "/data/test.txt";
    let content = b"test content";
    let content_digest = sha2::Sha256::digest(content);
    let content_digest_hex = hex::encode(content_digest);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let result = IngressValidator::validate_ingress(
        request_id,
        &receipt_reference,
        operation,
        target,
        &content_digest_hex,
        content,
        timestamp,
    );

    assert!(result.is_ok());
}

#[test]
fn test_invalid_receipt_reference_length() {
    let result = IngressValidator::validate_ingress(
        "req-001",
        "invalid",
        "file.create",
        "/data/test.txt",
        &"b".repeat(64),
        b"test",
        1234567890,
    );

    assert!(matches!(
        result,
        Err(Base44AdapterError::IngressValidation(_))
    ));
}

#[test]
fn test_invalid_receipt_reference_not_hex() {
    let result = IngressValidator::validate_ingress(
        "req-001",
        &"z".repeat(64),
        "file.create",
        "/data/test.txt",
        &"a".repeat(64),
        b"test",
        1234567890,
    );

    assert!(matches!(
        result,
        Err(Base44AdapterError::IngressValidation(_))
    ));
}

#[test]
fn test_payload_too_large() {
    let large_payload = vec![0u8; 65 * 1024];

    let result = IngressValidator::validate_ingress(
        "req-001",
        &"a".repeat(64),
        "file.create",
        "/data/test.txt",
        &"b".repeat(64),
        &large_payload,
        1234567890,
    );

    assert!(matches!(
        result,
        Err(Base44AdapterError::PayloadTooLarge { .. })
    ));
}

#[test]
fn test_timestamp_outside_window() {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let old_timestamp = current_time - 600;

    let result = IngressValidator::validate_ingress(
        "req-001",
        &"a".repeat(64),
        "file.create",
        "/data/test.txt",
        &"b".repeat(64),
        b"test",
        old_timestamp,
    );

    assert!(matches!(
        result,
        Err(Base44AdapterError::InvalidTimestamp(_))
    ));
}

#[test]
fn test_digest_mismatch() {
    let content = b"test content";
    let wrong_digest = "a".repeat(64);

    let result = IngressValidator::verify_content_digest(content, &wrong_digest);

    assert!(matches!(
        result,
        Err(Base44AdapterError::DigestMismatch { .. })
    ));
}

#[test]
fn test_digest_match() {
    let content = b"test content";
    let correct_digest = sha2::Sha256::digest(content);
    let correct_digest_hex = hex::encode(correct_digest);

    let result = IngressValidator::verify_content_digest(content, &correct_digest_hex);

    assert!(result.is_ok());
}
