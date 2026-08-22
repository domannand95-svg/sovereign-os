//! Egress translation integration tests.

extern crate sovereign_base44_adapter;

use sovereign_base44_adapter::{
    Base44EgressResponse, Base44EgressTranslator, Base44ExecutionStatus,
};
use sovereign_execution_api::{ExecutionStatus, GovernedExecutionResponse};

#[test]
fn test_translate_success_response() {
    let translator = Base44EgressTranslator::new();
    let governed_response = GovernedExecutionResponse {
        execution_id: "exec-001".to_string(),
        status: ExecutionStatus::AuthorizedAndExecuted,
        report_reference: Some("report-001".to_string()),
    };

    let result = translator.translate(governed_response, "req-001".to_string());

    assert!(result.is_ok());
    let egress = result.unwrap();
    assert_eq!(egress.request_id, "req-001");
    assert_eq!(egress.execution_id, "exec-001");
    assert_eq!(egress.status, Base44ExecutionStatus::Success);
    assert_eq!(egress.report_reference, Some("report-001".to_string()));
}

#[test]
fn test_translate_authentication_failed() {
    let translator = Base44EgressTranslator::new();
    let governed_response = GovernedExecutionResponse {
        execution_id: "exec-002".to_string(),
        status: ExecutionStatus::AuthenticationFailed,
        report_reference: None,
    };

    let result = translator.translate(governed_response, "req-002".to_string());

    assert!(result.is_ok());
    let egress = result.unwrap();
    assert_eq!(egress.status, Base44ExecutionStatus::AuthenticationFailed);
}

#[test]
fn test_translate_execution_failed_sanitizes_error() {
    let translator = Base44EgressTranslator::new();
    let governed_response = GovernedExecutionResponse {
        execution_id: "exec-003".to_string(),
        status: ExecutionStatus::ExecutionFailed,
        report_reference: None,
    };

    let result = translator.translate(governed_response, "req-003".to_string());

    assert!(result.is_ok());
    let egress = result.unwrap();
    assert_eq!(egress.status, Base44ExecutionStatus::ExecutionFailed);
}

#[test]
fn test_to_json_serialization() {
    let translator = Base44EgressTranslator::new();
    let response = Base44EgressResponse {
        request_id: "req-004".to_string(),
        execution_id: "exec-004".to_string(),
        status: Base44ExecutionStatus::Success,
        report_reference: Some("report-004".to_string()),
    };

    let json_result = translator.to_json(&response);

    assert!(json_result.is_ok());
    let json = json_result.unwrap();
    assert!(json.contains("req-004"));
    assert!(json.contains("exec-004"));
    assert!(json.contains("Success"));
}

