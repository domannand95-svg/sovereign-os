//! Full dispatch lifecycle integration tests.

use sha2::Digest;

use sovereign_base44_adapter::{
    Base44Dispatcher,
    Base44ExecutionStatus,
    Base44IngressRequest,
};

use sovereign_execution_api::{
    ExecutionApiFacade,
    KernelExecutionError,
    KernelExecutionRequest,
    KernelExecutionResponse,
    KernelInvoker,
};

struct MockKernel;

impl KernelInvoker for MockKernel {
    fn invoke_kernel(
        &self,
        _request: KernelExecutionRequest,
    ) -> Result<KernelExecutionResponse, KernelExecutionError> {
        Ok(KernelExecutionResponse {
            report_reference: "report-lifecycle-001".to_string(),
        })
    }
}

fn build_request() -> Base44IngressRequest {
    let content = b"test content";
    let content_digest = sha2::Sha256::digest(content);

    Base44IngressRequest {
        request_id: "req-lifecycle-001".to_string(),
        receipt_reference: "a".repeat(64),
        operation: "file.create".to_string(),
        target: "/data/test.txt".to_string(),
        content_digest: hex::encode(content_digest),
        content: content.to_vec(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}

#[test]
fn test_full_dispatch_lifecycle_success() {
    let kernel = MockKernel;
    let api = ExecutionApiFacade::new(kernel);
    let dispatcher = Base44Dispatcher::new(api);

    let result = dispatcher.dispatch(build_request());

    assert!(result.is_ok());

    let response = result.unwrap();

    assert_eq!(
        response.status,
        Base44ExecutionStatus::Success
    );

    assert_eq!(
        response.report_reference,
        Some("report-lifecycle-001".to_string())
    );
}

#[test]
fn test_invalid_digest_is_rejected_before_kernel_execution() {
    let kernel = MockKernel;
    let api = ExecutionApiFacade::new(kernel);
    let dispatcher = Base44Dispatcher::new(api);

    let mut request = build_request();
    request.content_digest = "invalid".to_string();

    let result = dispatcher.dispatch(request);

    assert!(result.is_err());
}

#[test]
fn test_expired_timestamp_is_rejected_before_kernel_execution() {
    let kernel = MockKernel;
    let api = ExecutionApiFacade::new(kernel);
    let dispatcher = Base44Dispatcher::new(api);

    let mut request = build_request();

    request.timestamp = 0;

    let result = dispatcher.dispatch(request);

    assert!(result.is_err());
}

#[test]
fn test_oversized_payload_is_rejected_before_kernel_execution() {
    let kernel = MockKernel;
    let api = ExecutionApiFacade::new(kernel);
    let dispatcher = Base44Dispatcher::new(api);

    let mut request = build_request();

    request.content = vec![0u8; 65 * 1024];

    let result = dispatcher.dispatch(request);

    assert!(result.is_err());
}