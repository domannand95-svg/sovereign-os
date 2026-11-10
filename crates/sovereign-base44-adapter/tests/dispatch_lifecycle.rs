//! Full dispatch lifecycle integration tests.
//!
//! The stand-in kernel records every invocation, so rejection tests can prove the
//! kernel was never reached rather than only that an error came back. Each rejection
//! fixture changes exactly one thing from a request that is otherwise accepted.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use sha2::Digest;

use sovereign_base44_adapter::validation::IngressValidator;
use sovereign_base44_adapter::{
    Base44AdapterError, Base44Dispatcher, Base44ExecutionStatus, Base44IngressRequest,
};

use sovereign_execution_api::{
    ExecutionApiFacade, KernelExecutionError, KernelExecutionRequest, KernelExecutionResponse,
    KernelInvoker,
};

/// Stand-in kernel that counts invocations and then accepts or rejects.
struct RecordingKernel {
    invocations: Arc<AtomicUsize>,
    reject: bool,
}

impl KernelInvoker for RecordingKernel {
    fn invoke_kernel(
        &self,
        _request: KernelExecutionRequest,
    ) -> Result<KernelExecutionResponse, KernelExecutionError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        if self.reject {
            return Err(KernelExecutionError::OperationRejected(
                "simulated kernel rejection".to_string(),
            ));
        }
        Ok(KernelExecutionResponse {
            report_reference: "report-lifecycle-001".to_string(),
        })
    }
}

fn recording_dispatcher(reject: bool) -> (Base44Dispatcher<RecordingKernel>, Arc<AtomicUsize>) {
    let invocations = Arc::new(AtomicUsize::new(0));
    let kernel = RecordingKernel {
        invocations: Arc::clone(&invocations),
        reject,
    };
    (
        Base44Dispatcher::new(ExecutionApiFacade::new(kernel)),
        invocations,
    )
}

fn sha256_hex(content: &[u8]) -> String {
    hex::encode(sha2::Sha256::digest(content))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// A request that the dispatcher accepts. Rejection tests mutate one field of it.
fn build_request() -> Base44IngressRequest {
    let content = b"test content";

    Base44IngressRequest {
        request_id: "req-lifecycle-001".to_string(),
        receipt_reference: "a".repeat(64),
        operation: "file.create".to_string(),
        target: "/data/test.txt".to_string(),
        content_digest: sha256_hex(content),
        content: content.to_vec(),
        timestamp: now_secs(),
    }
}

#[test]
fn test_full_dispatch_lifecycle_success() {
    let (dispatcher, invocations) = recording_dispatcher(false);

    let response = dispatcher
        .dispatch(build_request())
        .expect("valid request should dispatch");

    assert_eq!(response.status, Base44ExecutionStatus::Success);
    assert_eq!(
        response.report_reference,
        Some("report-lifecycle-001".to_string())
    );
    // Positive control: the recorder does observe a kernel invocation.
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
}

#[test]
fn test_invalid_digest_is_rejected_before_kernel_execution() {
    let (dispatcher, invocations) = recording_dispatcher(false);

    let mut request = build_request();
    request.content_digest = "invalid".to_string();

    let error = dispatcher.dispatch(request).unwrap_err();

    assert!(
        matches!(error, Base44AdapterError::DigestMismatch { .. }),
        "unexpected error: {error:?}"
    );
    assert_eq!(invocations.load(Ordering::SeqCst), 0);
}

#[test]
fn test_wrong_well_formed_digest_is_rejected_before_kernel_execution() {
    let (dispatcher, invocations) = recording_dispatcher(false);

    // A syntactically valid SHA-256 digest of different content: only the
    // content comparison can reject this request.
    let mut request = build_request();
    request.content_digest = sha256_hex(b"different content");

    let error = dispatcher.dispatch(request).unwrap_err();

    assert!(
        matches!(error, Base44AdapterError::DigestMismatch { .. }),
        "unexpected error: {error:?}"
    );
    assert_eq!(invocations.load(Ordering::SeqCst), 0);
}

#[test]
fn test_expired_timestamp_is_rejected_before_kernel_execution() {
    let (dispatcher, invocations) = recording_dispatcher(false);

    let mut request = build_request();
    request.timestamp = 0;

    let error = dispatcher.dispatch(request).unwrap_err();

    assert!(
        matches!(error, Base44AdapterError::InvalidTimestamp(_)),
        "unexpected error: {error:?}"
    );
    assert_eq!(invocations.load(Ordering::SeqCst), 0);
}

#[test]
fn test_future_timestamp_is_rejected_before_kernel_execution() {
    let (dispatcher, invocations) = recording_dispatcher(false);

    let mut request = build_request();
    request.timestamp = now_secs() + 10 * IngressValidator::TIMESTAMP_WINDOW_SECS;

    let error = dispatcher.dispatch(request).unwrap_err();

    assert!(
        matches!(error, Base44AdapterError::InvalidTimestamp(_)),
        "unexpected error: {error:?}"
    );
    assert_eq!(invocations.load(Ordering::SeqCst), 0);
}

#[test]
fn test_oversized_payload_is_rejected_before_kernel_execution() {
    let (dispatcher, invocations) = recording_dispatcher(false);

    // The digest is recomputed for the oversized content so that size is the only
    // reason this request can be rejected.
    let mut request = build_request();
    request.content = vec![0u8; IngressValidator::MAX_PAYLOAD_SIZE + 1];
    request.content_digest = sha256_hex(&request.content);

    let error = dispatcher.dispatch(request).unwrap_err();

    assert!(
        matches!(error, Base44AdapterError::PayloadTooLarge { .. }),
        "unexpected error: {error:?}"
    );
    assert_eq!(invocations.load(Ordering::SeqCst), 0);
}

#[test]
fn test_empty_operation_is_rejected_before_kernel_execution() {
    let (dispatcher, invocations) = recording_dispatcher(false);

    let mut request = build_request();
    request.operation = String::new();

    let error = dispatcher.dispatch(request).unwrap_err();

    assert!(
        matches!(error, Base44AdapterError::IngressValidation(_)),
        "unexpected error: {error:?}"
    );
    assert_eq!(invocations.load(Ordering::SeqCst), 0);
}

#[test]
fn test_malformed_receipt_reference_is_rejected_before_kernel_execution() {
    let (dispatcher, invocations) = recording_dispatcher(false);

    let mut request = build_request();
    request.receipt_reference = "not-a-receipt".to_string();

    let error = dispatcher.dispatch(request).unwrap_err();

    assert!(
        matches!(error, Base44AdapterError::IngressValidation(_)),
        "unexpected error: {error:?}"
    );
    assert_eq!(invocations.load(Ordering::SeqCst), 0);
}

#[test]
fn test_kernel_rejection_propagates_to_base44_egress() {
    let (dispatcher, invocations) = recording_dispatcher(true);

    let mut request = build_request();
    request.request_id = "req-reject-001".to_string();

    let response = dispatcher
        .dispatch(request)
        .expect("kernel rejection is reported as a response");

    assert_eq!(response.status, Base44ExecutionStatus::ExecutionFailed);
    assert!(response.report_reference.is_none());
    // The request was valid, so it did reach the kernel exactly once.
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
}
