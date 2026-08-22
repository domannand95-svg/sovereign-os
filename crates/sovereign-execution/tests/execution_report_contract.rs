use sovereign_execution::{ExecutionAttempt, ExecutionOutcome, ExecutionReport};

#[test]
fn execution_report_contains_attempt_and_outcome() {
    let attempt = ExecutionAttempt {
        execution_id: "exec-report-001".into(),
        receipt_reference: "receipt-001".into(),
        operation_reference: "operation-001".into(),
        outcome: ExecutionOutcome::Created,
        timestamp: 2000,
    };

    let report = ExecutionReport {
        attempt,
        outcome: ExecutionOutcome::Created,
    };

    assert_eq!(report.attempt.execution_id, "exec-report-001");

    assert_eq!(report.outcome, ExecutionOutcome::Created);
}

#[test]
fn execution_report_preserves_rejection_without_authority() {
    use sovereign_execution::ExecutionError;

    let attempt = ExecutionAttempt {
        execution_id: "exec-report-002".into(),
        receipt_reference: "receipt-002".into(),
        operation_reference: "operation-002".into(),
        outcome: ExecutionOutcome::Rejected(ExecutionError::OperationMismatch),
        timestamp: 2001,
    };

    let report = ExecutionReport {
        attempt,
        outcome: ExecutionOutcome::Rejected(ExecutionError::OperationMismatch),
    };

    assert_eq!(report.attempt.receipt_reference, "receipt-002");
}
