use sovereign_execution_api::{
    ExecutionApiFacade, ExecutionStatus, GovernedExecutionRequest, KernelExecutionError,
    KernelExecutionRequest, KernelExecutionResponse, KernelInvoker,
};

#[derive(Default)]
struct MockKernelInvoker {
    called: std::cell::Cell<bool>,
    should_fail: bool,
}

impl KernelInvoker for MockKernelInvoker {
    fn invoke_kernel(
        &self,
        _request: KernelExecutionRequest,
    ) -> Result<KernelExecutionResponse, KernelExecutionError> {
        self.called.set(true);

        if self.should_fail {
            Err(KernelExecutionError::ExecutionFailure(
                "kernel failure".into(),
            ))
        } else {
            Ok(KernelExecutionResponse {
                report_reference: "report-001".into(),
            })
        }
    }
}

#[test]
fn valid_request_reaches_kernel_boundary() {
    let kernel = MockKernelInvoker::default();
    let facade = ExecutionApiFacade::new(kernel);

    let request =
        GovernedExecutionRequest::new("exec-001".into(), "receipt-001".into(), vec![1, 2, 3]);

    let result = facade.execute(request);

    assert!(result.is_ok());

    let response = match result {
        Ok(value) => value,
        Err(_) => panic!("expected successful execution"),
    };

    assert_eq!(response.execution_id, "exec-001");
    assert_eq!(response.status, ExecutionStatus::AuthorizedAndExecuted);
}

#[test]
fn empty_receipt_is_rejected_before_kernel_execution() {
    let kernel = MockKernelInvoker::default();
    let facade = ExecutionApiFacade::new(kernel);

    let request = GovernedExecutionRequest::new("exec-001".into(), "".into(), vec![1, 2, 3]);

    let result = facade.execute(request);

    assert!(result.is_err());
}

#[test]
fn empty_payload_is_rejected_before_kernel_execution() {
    let kernel = MockKernelInvoker::default();
    let facade = ExecutionApiFacade::new(kernel);

    let request =
        GovernedExecutionRequest::new("exec-001".into(), "receipt-001".into(), Vec::new());

    let result = facade.execute(request);

    assert!(result.is_err());
}

#[test]
fn kernel_failure_is_translated_into_execution_failure() {
    let kernel = MockKernelInvoker {
        called: std::cell::Cell::new(false),
        should_fail: true,
    };

    let facade = ExecutionApiFacade::new(kernel);

    let request = GovernedExecutionRequest::new("exec-001".into(), "receipt-001".into(), vec![1]);

    let result = facade.execute(request);

    assert!(result.is_ok());

    let response = match result {
        Ok(value) => value,
        Err(_) => panic!("expected response"),
    };

    match response.status {
        ExecutionStatus::ExecutionFailed => {}
        _ => panic!("expected execution failure"),
    }
}
