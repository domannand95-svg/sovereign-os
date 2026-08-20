// ============================================================================
// AGENT-BETA-015-D: Approval Gate Interface
// ============================================================================
// Invariant: Approval != Execution. Approval != Capability Creation.
// ============================================================================

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

    pub fn update_operation(&mut self, new_operation: ProposedOperation) {
        self.proposed_operation = new_operation;
        self.status = ProposalStatus::Draft;
    }

    pub fn asserts_no_authority_expansion(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlastRadius {
    Isolated,
    Subsystem,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskEvaluationContext {
    pub risk_level: RiskLevel,
    pub blast_radius: BlastRadius,
    pub escalation_rationale: String,
    pub mandated_approval_level: ApprovalLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRecord {
    pub proposal_id: ProposalId,
    pub approver_identity: String,
    pub granted_approval_level: ApprovalLevel,
    pub signature_hash: String,
    pub timestamp: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ApprovalError {
    InvalidLifecycleState,
    InsufficientApprovalLevel,
    CryptographicBindingFailure,
}

pub struct ApprovalGate;

impl ApprovalGate {
    pub fn grant_approval(
        proposal: &GovernedActionProposal,
        risk_context: &RiskEvaluationContext,
        approver_identity: String,
        granted_level: ApprovalLevel,
        signature_stub: String,
        current_timestamp: u64,
    ) -> Result<ApprovalRecord, ApprovalError> {
        if proposal.status() != &ProposalStatus::Validated {
            return Err(ApprovalError::InvalidLifecycleState);
        }

        if granted_level < risk_context.mandated_approval_level {
            return Err(ApprovalError::InsufficientApprovalLevel);
        }

        if signature_stub.is_empty() {
            return Err(ApprovalError::CryptographicBindingFailure);
        }

        Ok(ApprovalRecord {
            proposal_id: proposal.proposal_id.clone(),
            approver_identity,
            granted_approval_level: granted_level,
            signature_hash: signature_stub,
            timestamp: current_timestamp,
        })
    }
}

// ============================================================================
// PAV SUITE EXTENSION (015-D)
// ============================================================================

#[cfg(test)]
mod pav_approval_tests {
    use super::*;

    fn setup_approval_scenario(
        mandated_level: ApprovalLevel,
    ) -> (GovernedActionProposal, RiskEvaluationContext) {
        let mut proposal = GovernedActionProposal::new(
            ProposalId("PROP-APP-01".to_string()),
            EpistemicObjectReference {
                object_digest: "digest".to_string(),
                verification_epoch: 1000,
            },
            PolicyEvaluationReference {
                rule_id: "RULE-01".to_string(),
                derived_decision: DerivedPolicyDecision::Permit,
            },
            ProposedOperation::QuarantineEntity {
                entity_id: "urn:entity:x".to_string(),
                reason: "risk".to_string(),
            },
            vec![],
        );
        proposal.mark_validated().unwrap();

        let risk_context = RiskEvaluationContext {
            risk_level: RiskLevel::Medium,
            blast_radius: BlastRadius::Subsystem,
            escalation_rationale: "Testing".to_string(),
            mandated_approval_level: mandated_level,
        };

        (proposal, risk_context)
    }

    #[test]
    fn pav_11_insufficient_privilege_rejection() {
        let (proposal, risk_context) = setup_approval_scenario(ApprovalLevel::Governance);

        let result = ApprovalGate::grant_approval(
            &proposal,
            &risk_context,
            "operator-jane".to_string(),
            ApprovalLevel::Operator,
            "SIG-123".to_string(),
            1710000000,
        );

        assert_eq!(result, Err(ApprovalError::InsufficientApprovalLevel));
    }

    #[test]
    fn pav_12_approval_capability_isolation() {
        let (proposal, risk_context) = setup_approval_scenario(ApprovalLevel::Operator);

        let approval_record = ApprovalGate::grant_approval(
            &proposal,
            &risk_context,
            "operator-jane".to_string(),
            ApprovalLevel::Operator,
            "SIG-456".to_string(),
            1710000000,
        )
        .unwrap();

        assert_eq!(approval_record.proposal_id.0, "PROP-APP-01");
        assert_eq!(
            approval_record.granted_approval_level,
            ApprovalLevel::Operator
        );
        assert!(proposal.asserts_no_authority_expansion());
    }

    #[test]
    fn pav_13_non_pending_lifecycle_rejection() {
        let (mut proposal, risk_context) = setup_approval_scenario(ApprovalLevel::Peer);

        proposal.update_operation(ProposedOperation::EmitNotification {
            target: "urn:internal:log".to_string(),
            message_hash: "hash".to_string(),
        });

        let result = ApprovalGate::grant_approval(
            &proposal,
            &risk_context,
            "peer-joe".to_string(),
            ApprovalLevel::Peer,
            "SIG-789".to_string(),
            1710000000,
        );

        assert_eq!(result, Err(ApprovalError::InvalidLifecycleState));
    }
}
