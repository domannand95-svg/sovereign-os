//! ADAM-010-B: Risk / Scope Evaluation Boundary
//!
//! Determines the potential impact of a proposal.
//! Invariant: Risk Evaluation != Execution Authority (Δ Authority = 0)

use crate::proposal::{GovernedActionProposal, ProposedOperation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlastRadius {
    Isolated,
    Subsystem,
    System,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskEvaluationContext {
    pub context_id: String,
    pub proposal_id: String,
    pub risk_level: RiskLevel,
    pub blast_radius: BlastRadius,
    pub evaluator_identity: String,
}

/// A deterministic evaluator that binds risk levels and blast radiuses to proposed operations.
pub struct RiskEvaluator;

impl RiskEvaluator {
    /// Evaluates a governed action proposal and generates a rigid risk context.
    /// The proposal cannot self-select its risk.
    pub fn evaluate_proposal(proposal: &GovernedActionProposal) -> RiskEvaluationContext {
        let (risk_level, blast_radius) = match proposal.proposed_operation {
            ProposedOperation::EmitNotification => (RiskLevel::Low, BlastRadius::Isolated),
            ProposedOperation::RequestReview => (RiskLevel::Low, BlastRadius::Subsystem),
            ProposedOperation::RequestApproval => (RiskLevel::Medium, BlastRadius::Subsystem),
            ProposedOperation::RequestStateMutation => (RiskLevel::High, BlastRadius::System),
        };

        RiskEvaluationContext {
            context_id: format!("risk-ctx-{}", proposal.proposal_id),
            proposal_id: proposal.proposal_id.clone(),
            risk_level,
            blast_radius,
            evaluator_identity: "system-risk-evaluator-v1".to_string(),
        }
    }
}
