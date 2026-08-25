use sovereign_execution_api::{
    AuthorizationReceiptRef, CanonicalAction, DigestRef, ExecutionApiFacade, ExecutionId,
    ExecutionStatus, GovernedExecutionRequest, KernelExecutionError, KernelExecutionRequest,
    KernelExecutionResponse, KernelInvoker,
};

fn valid_request() -> GovernedExecutionRequest {
    use sha2::Digest;
    let payload = [1, 2, 3];
    let digest = DigestRef::from_bytes(sha2::Sha256::digest(payload).into());
    GovernedExecutionRequest::new(
        ExecutionId::parse("exec-001").unwrap(),
        AuthorizationReceiptRef::parse_hex(&"a".repeat(64)).unwrap(),
        CanonicalAction::encode("write", "/bounded/target", digest, &payload).unwrap(),
    )
}

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

    let request = valid_request();

    let result = facade.execute(request);

    assert!(result.is_ok());

    let response = match result {
        Ok(value) => value,
        Err(_) => panic!("expected successful execution"),
    };

    assert_eq!(response.execution_id.as_str(), "exec-001");
    assert_eq!(response.status, ExecutionStatus::AuthorizedAndExecuted);
}

#[test]
fn empty_receipt_cannot_construct_governed_request() {
    let action = valid_request().action().clone();
    assert!(GovernedExecutionRequest::parse("exec-001", "", action.as_bytes()).is_err());
}

#[test]
fn empty_payload_cannot_become_canonical_action() {
    assert!(CanonicalAction::parse(&[]).is_err());
}

#[test]
fn kernel_failure_is_translated_into_execution_failure() {
    let kernel = MockKernelInvoker {
        called: std::cell::Cell::new(false),
        should_fail: true,
    };

    let facade = ExecutionApiFacade::new(kernel);

    let request = valid_request();

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
