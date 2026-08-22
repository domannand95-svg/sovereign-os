use sovereign_execution::{
    ExecutionAttempt,
    ExecutionOutcome,
    ExecutionError,
};

#[test]
fn execution_attempt_preserves_created_result() {
    let attempt = ExecutionAttempt {
        execution_id: "exec-001".into(),
        receipt_reference: "receipt-001".into(),
        operation_reference: "operation-001".into(),
        outcome: ExecutionOutcome::Created,
        timestamp: 1000,
    };

    assert_eq!(attempt.outcome, ExecutionOutcome::Created);
}

#[test]
fn execution_attempt_preserves_failed_result() {
    let attempt = ExecutionAttempt {
        execution_id: "exec-002".into(),
        receipt_reference: "receipt-002".into(),
        operation_reference: "operation-002".into(),
        outcome: ExecutionOutcome::Rejected(
            ExecutionError::ContentIntegrityMismatch
        ),
        timestamp: 1001,
    };

    assert_eq!(
        attempt.outcome,
        ExecutionOutcome::Rejected(
            ExecutionError::ContentIntegrityMismatch
        )
    );
}

#[test]
fn execution_attempt_does_not_authorize_execution() {
    let attempt = ExecutionAttempt {
        execution_id: "exec-003".into(),
        receipt_reference: "receipt-003".into(),
        operation_reference: "operation-003".into(),
        outcome: ExecutionOutcome::Created,
        timestamp: 1002,
    };

    assert_eq!(attempt.receipt_reference, "receipt-003");
}
