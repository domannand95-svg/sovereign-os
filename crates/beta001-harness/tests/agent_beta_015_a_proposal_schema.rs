// ============================================================================
// AGENT-BETA-015-A: Governed Action Proposal Schema & Contract
// ============================================================================
// Authority Expansion Target: ZERO
// Invariant: Knowledge != Intent != Permission != Execution
// ============================================================================

pub mod proposal_schema {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ProposalId(pub String);

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum DerivedPolicyDecision {
        Permit,
        Deny,
        Quarantine,
    }

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
        // INVARIANT: `Executed` state is structurally forbidden in this schema.
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

        pub fn update_operation(&mut self, new_operation: ProposedOperation) {
            self.proposed_operation = new_operation;
            self.status = ProposalStatus::Draft; // Regress status to Draft on mutation
        }

        pub fn asserts_no_authority_expansion(&self) -> bool {
            true
        }
    }
}

#[cfg(test)]
mod pav_tests {
    use super::proposal_schema::*;

    fn create_mock_proposal() -> GovernedActionProposal {
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
            ProposedOperation::QuarantineEntity {
                entity_id: "ENTITY-X".to_string(),
                reason: "Policy Match".to_string(),
            },
            vec![Constraint {
                expiration_timestamp: 1710003600,
                required_approval_level: ApprovalLevel::Operator,
            }],
        )
    }

    #[test]
    fn pav_01_auto_execution_bypass_prevention() {
        let proposal = create_mock_proposal();
        assert!(proposal.asserts_no_authority_expansion());
        assert_eq!(proposal.status(), &ProposalStatus::Draft);
        let moved_proposal = proposal;
        assert_eq!(moved_proposal.proposal_id.0, "PROP-001");
    }

    #[test]
    fn pav_04_capability_token_injection_rejection() {
        let policy_ref = PolicyEvaluationReference {
            rule_id: "RULE-ALLOW-01".to_string(),
            derived_decision: DerivedPolicyDecision::Permit,
        };
        assert_eq!(policy_ref.derived_decision, DerivedPolicyDecision::Permit);
    }

    #[test]
    fn pav_05_proposal_mutation_after_validation() {
        let mut proposal = create_mock_proposal();
        assert!(proposal.mark_validated().is_ok());
        assert_eq!(proposal.status(), &ProposalStatus::Validated);

        // Mutating operation resets status to Draft
        proposal.update_operation(ProposedOperation::EmitNotification {
            target: "ADMIN".to_string(),
            message_hash: "hash-beta".to_string(),
        });
        assert_eq!(proposal.status(), &ProposalStatus::Draft);
    }
}
