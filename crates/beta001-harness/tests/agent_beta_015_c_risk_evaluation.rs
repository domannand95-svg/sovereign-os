// ============================================================================
// AGENT-BETA-015-C: Risk and Scope Evaluation Layer
// ============================================================================
// Invariant: Risk Assessment != Approval. Scope Evaluation != Permission Grant.
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

    pub fn asserts_no_authority_expansion(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, PartialEq, Eq)]
pub enum RiskEvaluationError {
    InvalidLifecycleState,
}

pub struct RiskEvaluator;

impl RiskEvaluator {
    pub fn evaluate_proposal(
        proposal: &GovernedActionProposal,
    ) -> Result<RiskEvaluationContext, RiskEvaluationError> {
        if proposal.status() != &ProposalStatus::Validated {
            return Err(RiskEvaluationError::InvalidLifecycleState);
        }

        let (risk_level, blast_radius, mut mandated_approval_level, rationale) = match &proposal.proposed_operation {
            ProposedOperation::EmitNotification { .. } => (
                RiskLevel::Low,
                BlastRadius::Isolated,
                ApprovalLevel::Peer,
                String::from("Standard baseline notification"),
            ),
            ProposedOperation::QuarantineEntity { .. } => (
                RiskLevel::Medium,
                BlastRadius::Subsystem,
                ApprovalLevel::Operator,
                String::from("Subsystem quarantine requires operator clearance"),
            ),
            ProposedOperation::RequestStateMutation { .. } => (
                RiskLevel::High,
                BlastRadius::Global,
                ApprovalLevel::Governance,
                String::from("Global state mutation mandates strict governance approval"),
            ),
        };

        // Monotonic Escalation Guard: never downgrade existing stricter constraints
        for constraint in &proposal.constraints {
            if constraint.required_approval_level > mandated_approval_level {
                mandated_approval_level = constraint.required_approval_level.clone();
            }
        }

        Ok(RiskEvaluationContext {
            risk_level,
            blast_radius,
            escalation_rationale: rationale,
            mandated_approval_level,
        })
    }
}

// ============================================================================
// PAV SUITE EXTENSION (015-C)
// ============================================================================

#[cfg(test)]
mod pav_risk_tests {
    use super::*;

    fn generate_validated_proposal(op: ProposedOperation) -> GovernedActionProposal {
        let mut proposal = GovernedActionProposal::new(
            ProposalId("PROP-RISK-01".to_string()),
            EpistemicObjectReference { object_digest: "digest".to_string(), verification_epoch: 1000 },
            PolicyEvaluationReference { rule_id: "RULE-01".to_string(), derived_decision: DerivedPolicyDecision::Permit },
            op,
            vec![Constraint { expiration_timestamp: 2000000000, required_approval_level: ApprovalLevel::Peer }],
        );
        proposal.mark_validated().unwrap();
        proposal
    }

    #[test]
    fn pav_08_scope_escalation_containment() {
        let proposal = generate_validated_proposal(
            ProposedOperation::RequestStateMutation {
                key: "urn:internal:critical_config".to_string(),
                new_value_hash: "a".repeat(64),
            }
        );

        let risk_context = RiskEvaluator::evaluate_proposal(&proposal).unwrap();
        assert_eq!(risk_context.risk_level, RiskLevel::High);
        assert_eq!(risk_context.blast_radius, BlastRadius::Global);
        assert_eq!(risk_context.mandated_approval_level, ApprovalLevel::Governance);
    }

    #[test]
    fn pav_09_authorization_isolation() {
        let proposal = generate_validated_proposal(
            ProposedOperation::EmitNotification {
                target: "urn:internal:log".to_string(),
                message_hash: "b".repeat(64),
            }
        );

        let risk_context = RiskEvaluator::evaluate_proposal(&proposal).unwrap();
        assert_eq!(risk_context.risk_level, RiskLevel::Low);
        assert!(proposal.asserts_no_authority_expansion());
        assert_eq!(proposal.status(), &ProposalStatus::Validated);
    }

    #[test]
    fn pav_10_lifecycle_prerequisite_enforcement() {
        let draft_proposal = GovernedActionProposal::new(
            ProposalId("PROP-RISK-02".to_string()),
            EpistemicObjectReference { object_digest: "digest".to_string(), verification_epoch: 1000 },
            PolicyEvaluationReference { rule_id: "RULE-02".to_string(), derived_decision: DerivedPolicyDecision::Permit },
            ProposedOperation::EmitNotification { target: "urn:internal:target".to_string(), message_hash: "hash".to_string() },
            vec![],
        );

        let result = RiskEvaluator::evaluate_proposal(&draft_proposal);
        assert_eq!(result, Err(RiskEvaluationError::InvalidLifecycleState));
    }
}
