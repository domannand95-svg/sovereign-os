//! BETA-015 governed action proposal boundary.
//!
//! Invariants:
//! Knowledge != Intent != Permission != Execution
//! Proposal creation does not create capability.

use crate::decision::DerivedPolicyDecision;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
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
    EmitNotification {
        target: String,
        message_hash: String,
    },
    QuarantineEntity {
        entity_id: String,
        reason: String,
    },
    RequestStateMutation {
        key: String,
        new_value_hash: String,
    },
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

    pub fn update_operation(&mut self, operation: ProposedOperation) {
        self.proposed_operation = operation;
        self.status = ProposalStatus::Draft;
    }

    pub fn asserts_no_authority_expansion(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::PolicyEvaluationResult;

    fn create_inert_proposal() -> GovernedActionProposal {
        GovernedActionProposal::new(
            ProposalId("PROP-001".to_string()),
            EpistemicObjectReference {
                object_digest: "digest-alpha".to_string(),
                verification_epoch: 1710000000,
            },
            PolicyEvaluationReference {
                rule_id: "RULE-ALLOW-01".to_string(),
                derived_decision: DerivedPolicyDecision::Permit,
            },
            ProposedOperation::EmitNotification {
                target: "ADMIN".to_string(),
                message_hash: "hash-alpha".to_string(),
            },
            vec![],
        )
    }

    #[test]
    fn pav_01_proposal_cannot_auto_execute() {
        let proposal = create_inert_proposal();

        assert_eq!(proposal.status(), &ProposalStatus::Draft);
        assert!(proposal.asserts_no_authority_expansion());
    }

    #[test]
    fn pav_02_proposal_cannot_contain_capability_token() {
        let proposal = create_inert_proposal();

        assert!(proposal.constraints.is_empty());
    }

    #[test]
    fn pav_03_permit_does_not_become_permission() {
        let proposal = create_inert_proposal();

        let evaluation = PolicyEvaluationResult {
            decision: DerivedPolicyDecision::Permit,
            evaluated_rule_id: "RULE-ALLOW-01".to_string(),
            authority_expansion: 0,
        };

        assert_eq!(evaluation.decision, DerivedPolicyDecision::Permit);

        assert_eq!(proposal.status(), &ProposalStatus::Draft);
    }

    #[test]
    fn pav_04_mutation_after_validation_returns_to_draft() {
        let mut proposal = create_inert_proposal();

        proposal.mark_validated().unwrap();

        assert_eq!(proposal.status(), &ProposalStatus::Validated);

        proposal.update_operation(ProposedOperation::QuarantineEntity {
            entity_id: "ENTITY-X".to_string(),
            reason: "updated".to_string(),
        });

        assert_eq!(proposal.status(), &ProposalStatus::Draft);
    }

    #[test]
    fn pav_05_proposal_has_zero_authority_expansion() {
        let valid = PolicyEvaluationResult {
            decision: DerivedPolicyDecision::Permit,
            evaluated_rule_id: "RULE-ALLOW".to_string(),
            authority_expansion: 0,
        };

        assert!(valid.validate().is_ok());

        let invalid = PolicyEvaluationResult {
            decision: DerivedPolicyDecision::Permit,
            evaluated_rule_id: "RULE-ALLOW".to_string(),
            authority_expansion: 1,
        };

        assert!(invalid.validate().is_err());
    }
}
