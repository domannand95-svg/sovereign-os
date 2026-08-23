//! ADAM-010-D: Execution Adapter Isolation
//!
//! The final boundary before external capability invocation.
//! Invariant: Adapter Capability != Authority (Δ Authority = 0)

use crate::approval::{ApprovalLevel, ApprovalReceipt, ApprovalValidationError};
use crate::proposal::GovernedActionProposal;
use crate::risk::RiskEvaluationContext;
use std::collections::HashSet;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterIsolationError {
    MissingOrInvalidApproval(ApprovalValidationError),
    ProposalMismatch,
    RiskContextMismatch,
    ReplayDetected,
}

/// The absolute gatekeeper enforcing the integration of all prior governance boundaries.
pub struct ExecutionAdapterGate {
    /// Tracks executed proposal nonces to enforce anti-replay invariants.
    executed_proposals: Mutex<HashSet<String>>,
}

impl Default for ExecutionAdapterGate {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionAdapterGate {
    pub fn new() -> Self {
        Self {
            executed_proposals: Mutex::new(HashSet::new()),
        }
    }

    /// Evaluates the complete authorization package before yielding to an external capability.
    pub fn verify_execution_readiness(
        &self,
        proposal: &GovernedActionProposal,
        risk_context: &RiskEvaluationContext,
        receipt: &ApprovalReceipt,
    ) -> Result<(), AdapterIsolationError> {
        // 1. Enforce strict cryptographic binding between Receipt and Proposal
        if receipt.proposal_id != proposal.proposal_id {
            return Err(AdapterIsolationError::ProposalMismatch);
        }

        // 2. Enforce strict cryptographic binding between Receipt and Risk Context
        if receipt.risk_context_id != risk_context.context_id {
            return Err(AdapterIsolationError::RiskContextMismatch);
        }

        // 3. Perform inherent cryptographic and temporal validation of the Receipt
        receipt
            .validate_integrity()
            .map_err(AdapterIsolationError::MissingOrInvalidApproval)?;

        // 4. Enforce Scope Boundaries strictly dictated by computed Risk Level
        let required_level = match risk_context.risk_level {
            crate::risk::RiskLevel::Low => ApprovalLevel::Peer,
            crate::risk::RiskLevel::Medium => ApprovalLevel::Operator,
            crate::risk::RiskLevel::High | crate::risk::RiskLevel::Critical => {
                ApprovalLevel::Governance
            }
        };

        receipt
            .verify_scope(required_level)
            .map_err(AdapterIsolationError::MissingOrInvalidApproval)?;

        // 5. Enforce Anti-Replay Nonce Constraint
        let mut cache = self.executed_proposals.lock().unwrap();
        if cache.contains(&proposal.proposal_id) {
            return Err(AdapterIsolationError::ReplayDetected);
        }

        // Lock the execution nonce to immediately burn the approval token
        cache.insert(proposal.proposal_id.clone());

        Ok(())
    }
}
