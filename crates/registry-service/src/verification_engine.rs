use crate::VerificationProof;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationError {
    EmptyExecutionTrace,
    StepTraceCorrupted,
    CryptographicHashMismatch,
    StabilityBoundaryViolation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationMetrics {
    pub total_assertions_evaluated: u32,
    pub evaluation_depth: u32,
    pub performance_score_delta: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationEngine {
    pub max_permitted_depth: u32,
    pub strict_hash_verification: bool,
}

impl VerificationEngine {
    pub fn new(max_permitted_depth: u32, strict_hash_verification: bool) -> Self {
        Self {
            max_permitted_depth,
            strict_hash_verification,
        }
    }

    pub fn verify_proof_trace(
        &self,
        proof: &VerificationProof,
    ) -> Result<VerificationMetrics, ValidationError> {
        if proof.step_trace.is_empty() {
            return Err(ValidationError::EmptyExecutionTrace);
        }

        let depth = proof.step_trace.len() as u32;

        if depth > self.max_permitted_depth {
            return Err(ValidationError::StepTraceCorrupted);
        }

        if !self.audit_stability_margins(&proof.step_trace) {
            return Err(ValidationError::StabilityBoundaryViolation);
        }

        Ok(VerificationMetrics {
            total_assertions_evaluated: depth,
            evaluation_depth: depth,
            performance_score_delta: depth * 10,
        })
    }

    pub fn validate_state_delta(
        &self,
        computed_hash: &[u8; 32],
        declared_hash: &[u8; 32],
    ) -> Result<(), ValidationError> {
        if self.strict_hash_verification && computed_hash != declared_hash {
            return Err(ValidationError::CryptographicHashMismatch);
        }

        Ok(())
    }

    pub fn audit_stability_margins(&self, trace: &[String]) -> bool {
        trace
            .iter()
            .all(|line| !line.contains("VIOLATION") && !line.contains("CRITICAL_DROP"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proof(trace: Vec<&str>, hash: [u8; 32]) -> VerificationProof {
        VerificationProof {
            step_trace: trace.into_iter().map(String::from).collect(),
            state_delta_hash: hash,
        }
    }

    #[test]
    fn rejects_empty_trace() {
        let engine = VerificationEngine::new(10, true);
        let proof = proof(vec![], [1; 32]);

        assert_eq!(
            engine.verify_proof_trace(&proof),
            Err(ValidationError::EmptyExecutionTrace)
        );
    }

    #[test]
    fn rejects_trace_exceeding_depth() {
        let engine = VerificationEngine::new(2, true);
        let proof = proof(vec!["step 1", "step 2", "step 3"], [1; 32]);

        assert_eq!(
            engine.verify_proof_trace(&proof),
            Err(ValidationError::StepTraceCorrupted)
        );
    }

    #[test]
    fn rejects_stability_violation() {
        let engine = VerificationEngine::new(10, true);
        let proof = proof(vec!["step 1", "CRITICAL_DROP detected"], [1; 32]);

        assert_eq!(
            engine.verify_proof_trace(&proof),
            Err(ValidationError::StabilityBoundaryViolation)
        );
    }

    #[test]
    fn accepts_valid_trace() {
        let engine = VerificationEngine::new(10, true);
        let proof = proof(vec!["loaded input", "computed output"], [1; 32]);

        let metrics = engine.verify_proof_trace(&proof).unwrap();

        assert_eq!(metrics.evaluation_depth, 2);
        assert_eq!(metrics.total_assertions_evaluated, 2);
        assert_eq!(metrics.performance_score_delta, 20);
    }

    #[test]
    fn strict_hash_verification_rejects_mismatch() {
        let engine = VerificationEngine::new(10, true);

        assert_eq!(
            engine.validate_state_delta(&[1; 32], &[2; 32]),
            Err(ValidationError::CryptographicHashMismatch)
        );
    }

    #[test]
    fn non_strict_hash_verification_allows_mismatch() {
        let engine = VerificationEngine::new(10, false);

        assert_eq!(engine.validate_state_delta(&[1; 32], &[2; 32]), Ok(()));
    }
}
