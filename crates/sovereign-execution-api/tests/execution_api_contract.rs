use sovereign_execution_api::{
    ExecutionApiFacade,
    ExecutionApiError,
    ExecutionStatus,
    GovernedExecutionRequest,
    KernelInvoker,
};

#[derive(Default)]
struct MockKernelInvoker {
    should_fail: bool,
}

impl KernelInvoker for MockKernelInvoker {
    fn invoke_kernel(
        &self,
        _receipt_id: &str,
        _payload: &[u8],
    ) -> Result<String, String> {
        if self.should_fail {
            Err("mock kernel failure".to_string())
        } else {
            Ok("report-ref-001".to_string())
        }
    }
}

#[test]
fn valid_request_executes_through_kernel_boundary() {
    let facade = ExecutionApiFacade::new(MockKernelInvoker::default());

    let request = GovernedExecutionRequest::new(
        "exec-001".to_string(),
        "receipt-001".to_string(),
        vec![1, 2, 3],
    );

    let result = facade.execute(request);

    match result {
        Ok(response) => {
            assert_eq!(response.execution_id, "exec-001");
            assert_eq!(
                response.status,
                ExecutionStatus::AuthorizedAndExecuted
            );
            assert_eq!(
                response.report_reference,
                Some("report-ref-001".to_string())
            );
        }
        Err(error) => panic!("unexpected API error: {error}"),
    }
}

#[test]
fn empty_receipt_is_rejected_before_kernel_execution() {
    let facade = ExecutionApiFacade::new(MockKernelInvoker::default());

    let request = GovernedExecutionRequest::new(
        "exec-002".to_string(),
        "".to_string(),
        vec![1],
    );

    let result = facade.execute(request);

    match result {
        Err(ExecutionApiError::InvalidReceipt(_)) => {}
        _ => panic!("expected invalid receipt rejection"),
    }
}

#[test]
fn empty_payload_is_rejected_before_kernel_execution() {
    let facade = ExecutionApiFacade::new(MockKernelInvoker::default());

    let request = GovernedExecutionRequest::new(
        "exec-003".to_string(),
        "receipt-003".to_string(),
        Vec::new(),
    );

    let result = facade.execute(request);

    match result {
        Err(ExecutionApiError::SerializationFault(_)) => {}
        _ => panic!("expected payload rejection"),
    }
}

#[test]
fn kernel_failure_is_translated_into_execution_failure() {
    let facade = ExecutionApiFacade::new(MockKernelInvoker {
        should_fail: true,
    });

    let request = GovernedExecutionRequest::new(
        "exec-004".to_string(),
        "receipt-004".to_string(),
        vec![9],
    );

    let result = facade.execute(request);

    match result {
        Ok(response) => match response.status {
            ExecutionStatus::ExecutionFailed(_) => {}
            _ => panic!("expected execution failure status"),
        },
        Err(error) => panic!("unexpected API error: {error}"),
    }
}