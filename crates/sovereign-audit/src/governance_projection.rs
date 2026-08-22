//! BETA-016 Governance Projection Adapter
//!
//! Converts epistemic policy outputs into audit-layer governance artifacts.
//!
//! Invariants:
//! Proposal != Permission
//! Admission != Execution
//! Projection != Authority Expansion

use sovereign_policy::{DerivedPolicyDecision, GovernedActionProposal, PolicyEvaluationResult};

use crate::governance_admission::{AdmissionDecision, AdmissionOutcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityContext {
    pub context_reference: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectionError {
    MissingReference,
}

pub struct GovernanceProjectionAdapter;

impl GovernanceProjectionAdapter {
    pub fn project(
        proposal: &GovernedActionProposal,
        evaluation: &PolicyEvaluationResult,
        authority: &AuthorityContext,
    ) -> Result<AdmissionDecision, ProjectionError> {
        if proposal.proposal_id.0.is_empty()
            || authority.context_reference.is_empty()
            || evaluation.evaluated_rule_id.is_empty()
        {
            return Err(ProjectionError::MissingReference);
        }

        let outcome = match evaluation.decision {
            DerivedPolicyDecision::Permit => AdmissionOutcome::Permit,
            DerivedPolicyDecision::Deny => AdmissionOutcome::Deny,
            DerivedPolicyDecision::Quarantine => AdmissionOutcome::Quarantine,
        };

        Ok(AdmissionDecision {
            decision_reference: proposal.proposal_id.0.clone(),
            intent_reference: proposal.proposal_id.0.clone(),
            evaluation_reference: evaluation.evaluated_rule_id.clone(),
            governance_context_reference: authority.context_reference.clone(),
            outcome,
            authorized_scope: "none".to_string(),
            reason_references: vec![],
        })
    }
}
