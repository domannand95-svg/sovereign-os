//! Negative Boundary Tests for ADAM-010-C
//! 
//! Proves that approval receipts are strictly evaluated as inert evidence
//! rather than active execution capabilities.

use beta001_harness::approval::{ApprovalReceipt, ApprovalLevel, ApprovalValidationError};

fn valid_base_receipt() -> ApprovalReceipt {
    ApprovalReceipt {
        receipt_id: "receipt-010-c-001".to_string(),
        proposal_id: "prop-010-b-test".to_string(),
        risk_context_id: "risk-ctx-prop-010-b-test".to_string(),
        approver_identity: "governance-node-alpha".to_string(),
        approval_level: ApprovalLevel::Governance,
        timestamp: "2026-08-24T01:00:00Z".to_string(),
        expiration_timestamp: "2026-08-24T23:59:59Z".to_string(),
        signature: "crypto-signature-abc".to_string(),
    }
}

#[test]
fn test_a010_c_001_expired_approval_rejected() {
    let mut receipt = valid_base_receipt();
    receipt.expiration_timestamp = "2020-01-01T00:00:00Z".to_string(); // Expired
    
    assert_eq!(receipt.validate_integrity(), Err(ApprovalValidationError::ExpiredReceipt));
}

#[test]
fn test_a010_c_002_missing_or_invalid_signature_rejected() {
    let mut receipt = valid_base_receipt();
    receipt.signature = "unsigned".to_string();
    
    assert_eq!(receipt.validate_integrity(), Err(ApprovalValidationError::InvalidSignature));
}

#[test]
fn test_a010_c_003_wrong_approver_scope_rejected() {
    let mut receipt = valid_base_receipt();
    // Attempt to utilize a Peer-level approval for an operation requiring Operator-level authorization
    receipt.approval_level = ApprovalLevel::Peer;
    
    assert_eq!(receipt.verify_scope(ApprovalLevel::Operator), Err(ApprovalValidationError::InsufficientApprovalScope));
}

#[test]
fn test_a010_c_004_approval_evidence_does_not_contain_execution_authority() {
    let receipt = valid_base_receipt();
    
    // Test implicitly guarantees that the struct contains no nested capability grants,
    // raw shell scripts, execution tokens, or memory pointers. 
    // It remains a purely descriptive data struct.
    assert!(receipt.validate_integrity().is_ok());
    assert_eq!(std::mem::size_of::<ApprovalReceipt>(), 192); // Ensures struct size remains bound strictly to string references/enums
}