// ============================================================================
// AGENT-BETA-015-E: Proposal Lifecycle Replay & Integrity
// ============================================================================
// Invariant: Historical Record != Active Authority. Replay != Capability.
// ============================================================================

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedPolicyDecision {
    Permit,
    Deny,
    Quarantine,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApprovalLevel {
    Peer,
    Operator,
    Governance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpistemicObjectReference {
    pub object_digest: String,
    pub verification_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEvaluationReference {
    pub rule_id: String,
    pub derived_decision: DerivedPolicyDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalStatus {
    Draft,
    Validated,
    PendingApproval,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposedOperation {
    EmitNotification { target: String, message_hash: String },
    QuarantineEntity { entity_id: String, reason: String },
    RequestStateMutation { key: String, new_value_hash: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    pub expiration_timestamp: u64,
    pub required_approval_level: ApprovalLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernedActionProposal {
    pub proposal_id: ProposalId,
    pub source_object: EpistemicObjectReference,
    pub policy_context: PolicyEvaluationReference,
    pub proposed_operation: ProposedOperation,
    pub constraints: Vec<Constraint>,
    status: ProposalStatus,
}

impl GovernedActionProposal {
    pub fn new(
        proposal_id: ProposalId,
        source_object: EpistemicObjectReference,
        policy_context: PolicyEvaluationReference,
        proposed_operation: ProposedOperation,
        constraints: Vec<Constraint>,
    ) -> Self {
        Self {
            proposal_id,
            source_object,
            policy_context,
            proposed_operation,
            constraints,
            status: ProposalStatus::Draft,
        }
    }

    pub fn status(&self) -> &ProposalStatus {
        &self.status
    }

    pub fn mark_validated(&mut self) -> Result<(), &'static str> {
        if self.status != ProposalStatus::Draft {
            return Err("Only Draft proposals can be validated.");
        }
        self.status = ProposalStatus::Validated;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRecord {
    pub proposal_id: ProposalId,
    pub approver_identity: String,
    pub granted_approval_level: ApprovalLevel,
    pub signature_hash: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedGovernanceDossier {
    pub proposal_id: ProposalId,
    pub proposal_hash: String,
    pub approval_record: ApprovalRecord,
    pub sealed_timestamp: u64,
    pub authority_expansion: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IntegrityError {
    TemporalDriftDetected,
    ApprovalReplayAttack,
    IdMismatch,
    InvalidLifecycleState,
}

pub struct LifecycleIntegrityLedger {
    consumed_approvals: HashSet<String>,
}

impl LifecycleIntegrityLedger {
    pub fn new() -> Self {
        Self {
            consumed_approvals: HashSet::new(),
        }
    }

    pub fn seal_and_verify(
        &mut self,
        proposal: &GovernedActionProposal,
        approval_record: &ApprovalRecord,
        proposal_hash: String,
        current_timestamp: u64,
    ) -> Result<SealedGovernanceDossier, IntegrityError> {
        if proposal.proposal_id != approval_record.proposal_id {
            return Err(IntegrityError::IdMismatch);
        }
        if proposal.status() != &ProposalStatus::Validated {
            return Err(IntegrityError::InvalidLifecycleState);
        }

        for constraint in &proposal.constraints {
            if current_timestamp > constraint.expiration_timestamp {
                return Err(IntegrityError::TemporalDriftDetected);
            }
        }

        if self.consumed_approvals.contains(&approval_record.signature_hash) {
            return Err(IntegrityError::ApprovalReplayAttack);
        }

        self.consumed_approvals.insert(approval_record.signature_hash.clone());

        Ok(SealedGovernanceDossier {
            proposal_id: proposal.proposal_id.clone(),
            proposal_hash,
            approval_record: approval_record.clone(),
            sealed_timestamp: current_timestamp,
            authority_expansion: 0,
        })
    }
}

// ============================================================================
// PAV SUITE EXTENSION (015-E)
// ============================================================================

#[cfg(test)]
mod pav_integrity_tests {
    use super::*;

    fn setup_integrity_scenario() -> (GovernedActionProposal, ApprovalRecord) {
        let mut proposal = GovernedActionProposal::new(
            ProposalId("PROP-INT-01".to_string()),
            EpistemicObjectReference { object_digest: "digest".to_string(), verification_epoch: 1000 },
            PolicyEvaluationReference { rule_id: "RULE-01".to_string(), derived_decision: DerivedPolicyDecision::Permit },
            ProposedOperation::EmitNotification { target: "urn:internal:log".to_string(), message_hash: "hash".to_string() },
            vec![Constraint { expiration_timestamp: 1710005000, required_approval_level: ApprovalLevel::Peer }],
        );
        proposal.mark_validated().unwrap();

        let approval = ApprovalRecord {
            proposal_id: ProposalId("PROP-INT-01".to_string()),
            approver_identity: "peer-joe".to_string(),
            granted_approval_level: ApprovalLevel::Peer,
            signature_hash: "SIG-VALID-999".to_string(),
            timestamp: 1710001000,
        };

        (proposal, approval)
    }

    #[test]
    fn pav_14_temporal_drift_rejection() {
        let mut ledger = LifecycleIntegrityLedger::new();
        let (proposal, approval) = setup_integrity_scenario();
        let current_time_expired = 1710006000;

        let result = ledger.seal_and_verify(&proposal, &approval, "HASH-123".to_string(), current_time_expired);
        assert_eq!(result, Err(IntegrityError::TemporalDriftDetected));
    }

    #[test]
    fn pav_15_approval_replay_attack() {
        let mut ledger = LifecycleIntegrityLedger::new();
        let (proposal, approval) = setup_integrity_scenario();
        let current_time_valid = 1710002000;

        let first_result = ledger.seal_and_verify(&proposal, &approval, "HASH-123".to_string(), current_time_valid);
        assert!(first_result.is_ok());

        let second_result = ledger.seal_and_verify(&proposal, &approval, "HASH-123".to_string(), current_time_valid);
        assert_eq!(second_result, Err(IntegrityError::ApprovalReplayAttack));
    }

    #[test]
    fn pav_16_historical_state_containment() {
        let mut ledger = LifecycleIntegrityLedger::new();
        let (proposal, approval) = setup_integrity_scenario();

        let dossier = ledger.seal_and_verify(&proposal, &approval, "HASH-123".to_string(), 1710002000).unwrap();
        assert_eq!(dossier.authority_expansion, 0);
        assert_eq!(dossier.proposal_id.0, "PROP-INT-01");
    }
}
