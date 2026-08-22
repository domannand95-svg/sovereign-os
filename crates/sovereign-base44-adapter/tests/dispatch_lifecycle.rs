//! Full dispatch lifecycle integration tests.

extern crate sovereign_base44_adapter;

use sha2::Digest;
use sovereign_base44_adapter::{Base44Dispatcher, Base44ExecutionStatus, Base44IngressRequest};
use sovereign_execution_api::{
    ExecutionApiError, ExecutionStatus, GovernedExecutionRequest, GovernedExecutionResponse,
};

#[test]
fn test_full_dispatch_lifecycle_success() {
    let dispatcher = Base44Dispatcher::new();
    let content = b"test content";
    let content_digest = sha2::Sha256::digest(content);
    let content_digest_hex = hex::encode(content_digest);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let request = Base44IngressRequest {
        request_id: "req-lifecycle-001".to_string(),
        receipt_reference: "a".repeat(64),
        operation: "file.create".to_string(),
        target: "/data/test.txt".to_string(),
        content_digest: content_digest_hex,
        content: content.to_vec(),
        timestamp,
    };

    let executor = |_governed_request: GovernedExecutionRequest| {
        Ok(GovernedExecutionResponse {
            execution_id: "exec-lifecycle-001".to_string(),
            status: ExecutionStatus::AuthorizedAndExecuted,
            report_reference: Some("report-lifecycle-001".to_string()),
        })
    };

    let result = dispatcher.dispatch(request, executor);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.request_id, "req-lifecycle-001");
    assert_eq!(response.execution_id, "exec-lifecycle-001");
    assert_eq!(response.status, Base44ExecutionStatus::Success);
}

#[test]
fn test_dispatch_with_execution_failure() {
    let dispatcher = Base44Dispatcher::new();
    let content = b"test content";
    let content_digest = sha2::Sha256::digest(content);
    let content_digest_hex = hex::encode(content_digest);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let request = Base44IngressRequest {
        request_id: "req-lifecycle-002".to_string(),
        receipt_reference: "a".repeat(64),
        operation: "file.create".to_string(),
        target: "/data/test.txt".to_string(),
        content_digest: content_digest_hex,
        content: content.to_vec(),
        timestamp,
    };

    let executor = |_governed_request: GovernedExecutionRequest| {
        Ok(GovernedExecutionResponse {
            execution_id: "exec-lifecycle-002".to_string(),
            status: ExecutionStatus::ExecutionFailed("simulated failure".to_string()),
            report_reference: None,
        })
    };

    let result = dispatcher.dispatch(request, executor);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, Base44ExecutionStatus::ExecutionFailed);
}

#[test]
fn test_dispatch_with_api_error() {
    let dispatcher = Base44Dispatcher::new();
    let content = b"test content";
    let content_digest = sha2::Sha256::digest(content);
    let content_digest_hex = hex::encode(content_digest);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let request = Base44IngressRequest {
        request_id: "req-lifecycle-003".to_string(),
        receipt_reference: "a".repeat(64),
        operation: "file.create".to_string(),
        target: "/data/test.txt".to_string(),
        content_digest: content_digest_hex,
        content: content.to_vec(),
        timestamp,
    };

    let executor = |_governed_request: GovernedExecutionRequest| {
        Err(ExecutionApiError::KernelFault(
            "simulated kernel fault".to_string(),
        ))
    };

    let result = dispatcher.dispatch(request, executor);

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(sovereign_base44_adapter::Base44AdapterError::ExecutionApi(
            _
        ))
    ));
}

#[test]
fn test_dispatch_with_invalid_ingress() {
    let dispatcher = Base44Dispatcher::new();

    let request = Base44IngressRequest {
        request_id: "req-lifecycle-004".to_string(),
        receipt_reference: "invalid".to_string(),
        operation: "file.create".to_string(),
        target: "/data/test.txt".to_string(),
        content_digest: "a".repeat(64),
        content: b"test".to_vec(),
        timestamp: 1234567890,
    };

    let executor = |_governed_request: GovernedExecutionRequest| {
        Ok(GovernedExecutionResponse {
            execution_id: "exec-004".to_string(),
            status: ExecutionStatus::AuthorizedAndExecuted,
            report_reference: None,
        })
    };

    let result = dispatcher.dispatch(request, executor);

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(
            sovereign_base44_adapter::Base44AdapterError::ExecutionApi(_)
                | sovereign_base44_adapter::Base44AdapterError::IngressValidation(_)
        )
    ));
}
