use sovereign_base44_adapter::{
    Base44Dispatcher,
    Base44IngressRequest,
};

use sovereign_execution_api::{
    ExecutionApiFacade,
    KernelExecutionRequest,
    KernelExecutionResponse,
    KernelExecutionError,
    KernelInvoker,
};

struct MockKernel;

impl KernelInvoker for MockKernel {
    fn invoke_kernel(
        &self,
        _request: KernelExecutionRequest,
    ) -> Result<KernelExecutionResponse, KernelExecutionError> {
        Ok(KernelExecutionResponse {
            report_reference: "mock-report".to_string(),
        })
    }
}

#[test]
fn base44_request_crosses_api_boundary() {
    let request = Base44IngressRequest {
        request_id: "req-001".to_string(),
        receipt_reference: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        operation: "create".to_string(),
        target: "test.txt".to_string(),
        content_digest: "239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5".to_string(),
        content: b"payload".to_vec(),
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("system time must be valid").as_secs(),
    };

    let kernel = MockKernel;
    let api = ExecutionApiFacade::new(kernel);

    let dispatcher = Base44Dispatcher::new();

    let result = dispatcher.dispatch(request, |kernel_request| {
        api.execute(kernel_request).map_err(|_| sovereign_execution_api::ExecutionApiError::KernelFault("mock api failure".to_string()))
    });

    match result {
        Ok(response) => {
            assert_eq!(response.execution_id, "req-001");
        }
        Err(error) => {
            panic!("integration failure: {:?}", error);
        }
    }
}







