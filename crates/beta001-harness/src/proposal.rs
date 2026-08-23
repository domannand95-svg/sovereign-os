//! ADAM-010-A: Governed Action Proposal
//! 
//! Canonical data contract representing an evaluated intelligence proposal.
//! Invariant: Proposal != Execution (Δ Authority = 0)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposedOperation {
    EmitNotification,
    RequestReview,
    RequestApproval,
    RequestStateMutation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernedActionProposal {
    pub schema_version: String,
    pub proposal_id: String,
    pub proposer_identity: String,
    pub source_evidence_references: Vec<String>,
    pub intent: String,
    pub proposed_operation: ProposedOperation,
    pub risk_evaluation_reference: Option<String>,
    pub created_timestamp: String,
    pub expiration_timestamp: String,
    pub provenance_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalValidationError {
    UnsupportedSchemaVersion(String),
    MissingEvidence,
    InvalidIdentity,
    ExpiredProposal,
    ContainsExecutionAuthority,
}

impl GovernedActionProposal {
    /// Validates the structural and semantic boundaries of the proposal.
    /// This enforces that the proposal is correctly formed and does not contain
    /// hostile authorization injections.
    pub fn validate(&self) -> Result<(), ProposalValidationError> {
        if self.schema_version != "v1.0.0" {
            return Err(ProposalValidationError::UnsupportedSchemaVersion(self.schema_version.clone()));
        }
        
        if self.proposer_identity.trim().is_empty() {
            return Err(ProposalValidationError::InvalidIdentity);
        }
        
        if self.source_evidence_references.is_empty() {
            return Err(ProposalValidationError::MissingEvidence);
        }
        
        // Lexicographical string comparison for strict ISO8601 dates
        let current_time = "2026-08-24T00:00:00Z";
        if self.expiration_timestamp < current_time.to_string() {
            return Err(ProposalValidationError::ExpiredProposal);
        }
        
        // Semantic boundary: Reject implicit execution or authority escalation attempts
        let lower_intent = self.intent.to_lowercase();
        if lower_intent.contains("grant_authority") 
            || lower_intent.contains("execute root") 
            || lower_intent.contains("bypass policy") 
        {
            return Err(ProposalValidationError::ContainsExecutionAuthority);
        }
        
        Ok(())
    }
}