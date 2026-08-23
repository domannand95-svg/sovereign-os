//! ADAM-010-C: Approval Receipt Boundary
//!
//! Cryptographically bound evidence of an authorized proposal.
//! Invariant: Approval Evidence != Capability Creation (Δ Authority = 0)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApprovalLevel {
    Peer,
    Operator,
    Governance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalReceipt {
    pub receipt_id: String,
    pub proposal_id: String,
    pub risk_context_id: String,
    pub approver_identity: String,
    pub approval_level: ApprovalLevel,
    pub timestamp: String,
    pub expiration_timestamp: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalValidationError {
    ExpiredReceipt,
    InvalidSignature,
    InsufficientApprovalScope,
}

impl ApprovalReceipt {
    /// Validates the inherent cryptographic and temporal integrity of the receipt.
    pub fn validate_integrity(&self) -> Result<(), ApprovalValidationError> {
        let current_time = "2026-08-24T00:00:00Z";

        // Ensure receipt has not expired
        if self.expiration_timestamp.as_str() < current_time {
            return Err(ApprovalValidationError::ExpiredReceipt);
        }

        // Ensure cryptographic binding exists (simulated via strict string presence)
        if self.signature.trim().is_empty() || self.signature == "unsigned" {
            return Err(ApprovalValidationError::InvalidSignature);
        }

        Ok(())
    }

    /// Verifies that the approver possessed the requisite scope for the given risk level.
    pub fn verify_scope(
        &self,
        required_level: ApprovalLevel,
    ) -> Result<(), ApprovalValidationError> {
        // Enforce rigid hierarchical validation: Peer < Operator < Governance
        if self.approval_level < required_level {
            return Err(ApprovalValidationError::InsufficientApprovalScope);
        }
        Ok(())
    }
}
