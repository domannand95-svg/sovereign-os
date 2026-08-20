// ============================================================================
// AGENT-BETA-016-C: Execution Receipt & State Attestation Schema
// ============================================================================
// Authority Expansion Target: ZERO
// Invariant: Actuator Execution -> Verified Post-State Attestation -> Zero Residual Authority
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionDisposition {
    Success,
    Failed(String),
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReceipt {
    pub receipt_id: String,
    pub token_id: String,
    pub target_resource: String,
    pub operation_executed: String,
    pub disposition: ExecutionDisposition,
    pub pre_state_digest: String,
    pub post_state_digest: String,
    pub executed_at: u64,
    pub residual_authority_retained: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttestationError {
    ResidualAuthorityViolation,
    InvalidStateTransitionDigest,
    MissingTargetResource,
    IncompleteExecutionRecord,
}

pub struct ExecutionReceiptValidator;

impl ExecutionReceiptValidator {
    pub fn validate_and_attest(receipt: &ExecutionReceipt) -> Result<String, AttestationError> {
        if receipt.residual_authority_retained != 0 {
            return Err(AttestationError::ResidualAuthorityViolation);
        }

        if receipt.target_resource.is_empty() || receipt.operation_executed.is_empty() {
            return Err(AttestationError::MissingTargetResource);
        }

        if !receipt.pre_state_digest.starts_with("blake3:")
            || !receipt.post_state_digest.starts_with("blake3:")
        {
            return Err(AttestationError::InvalidStateTransitionDigest);
        }

        if receipt.receipt_id.is_empty() || receipt.token_id.is_empty() {
            return Err(AttestationError::IncompleteExecutionRecord);
        }

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"SOVEREIGN_EXECUTION_ATTESTATION_V1");
        hasher.update(receipt.receipt_id.as_bytes());
        hasher.update(receipt.token_id.as_bytes());
        hasher.update(receipt.target_resource.as_bytes());
        hasher.update(receipt.operation_executed.as_bytes());
        hasher.update(receipt.pre_state_digest.as_bytes());
        hasher.update(receipt.post_state_digest.as_bytes());
        hasher.update(&receipt.executed_at.to_be_bytes());

        let attestation_hash = format!("blake3:{}", hasher.finalize().to_hex());
        Ok(attestation_hash)
    }
}

// ============================================================================
// EXECUTION ATTESTATION VALIDATION SUITE (EXE-01..05)
// ============================================================================

#[cfg(test)]
mod execution_receipt_tests {
    use super::*;

    fn get_valid_receipt() -> ExecutionReceipt {
        ExecutionReceipt {
            receipt_id: "rcpt_exec_001".into(),
            token_id: "cap_tok_0123456789abcdef".into(),
            target_resource: "urn:internal:entity:x".into(),
            operation_executed: "QUARANTINE".into(),
            disposition: ExecutionDisposition::Success,
            pre_state_digest:
                "blake3:1111111111111111111111111111111111111111111111111111111111111111".into(),
            post_state_digest:
                "blake3:2222222222222222222222222222222222222222222222222222222222222222".into(),
            executed_at: 1710000030,
            residual_authority_retained: 0,
        }
    }

    #[test]
    fn exe_01_valid_execution_attestation() {
        let receipt = get_valid_receipt();
        let attestation = ExecutionReceiptValidator::validate_and_attest(&receipt);
        assert!(attestation.is_ok());
        let hash = attestation.unwrap();
        assert!(hash.starts_with("blake3:"));
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn exe_02_residual_authority_violation_rejection() {
        let mut receipt = get_valid_receipt();
        receipt.residual_authority_retained = 1;

        let res = ExecutionReceiptValidator::validate_and_attest(&receipt);
        assert_eq!(res, Err(AttestationError::ResidualAuthorityViolation));
    }

    #[test]
    fn exe_03_invalid_state_transition_digest_rejection() {
        let mut receipt = get_valid_receipt();
        receipt.post_state_digest = "invalid_hash_format".into();

        let res = ExecutionReceiptValidator::validate_and_attest(&receipt);
        assert_eq!(res, Err(AttestationError::InvalidStateTransitionDigest));
    }

    #[test]
    fn exe_04_missing_target_resource_rejection() {
        let mut receipt = get_valid_receipt();
        receipt.target_resource = "".into();

        let res = ExecutionReceiptValidator::validate_and_attest(&receipt);
        assert_eq!(res, Err(AttestationError::MissingTargetResource));
    }

    #[test]
    fn exe_05_deterministic_attestation_replay() {
        let receipt = get_valid_receipt();
        let hash_a = ExecutionReceiptValidator::validate_and_attest(&receipt).unwrap();
        let hash_b = ExecutionReceiptValidator::validate_and_attest(&receipt).unwrap();
        assert_eq!(hash_a, hash_b);
    }
}
