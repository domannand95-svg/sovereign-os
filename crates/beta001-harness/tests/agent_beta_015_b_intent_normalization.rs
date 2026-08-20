// ============================================================================
// AGENT-BETA-015-B: Intent Normalization Boundary
// ============================================================================
// Invariant: Ambiguity != Flexibility. Normalization != Execution.
// ============================================================================

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
}

#[derive(Debug, PartialEq, Eq)]
pub enum NormalizationError {
    AmbiguousTargetNamespace,
    InvalidCryptographicHashLength,
    UnboundedParameterSize,
    SemanticContradiction,
    InvalidLifecycleState,
}

pub struct IntentNormalizer;

impl IntentNormalizer {
    pub fn normalize_and_validate(
        proposal: &mut GovernedActionProposal,
    ) -> Result<(), NormalizationError> {
        if proposal.status() != &ProposalStatus::Draft {
            return Err(NormalizationError::InvalidLifecycleState);
        }

        // Semantic contradiction: Denied policy cannot propose state mutation
        if proposal.policy_context.derived_decision == DerivedPolicyDecision::Deny {
            if let ProposedOperation::RequestStateMutation { .. } = proposal.proposed_operation {
                return Err(NormalizationError::SemanticContradiction);
            }
        }

        // Parameter bounding & namespace checks
        match &proposal.proposed_operation {
            ProposedOperation::EmitNotification {
                target,
                message_hash,
            } => {
                if !target.starts_with("urn:internal:") {
                    return Err(NormalizationError::AmbiguousTargetNamespace);
                }
                if message_hash.len() != 64 {
                    return Err(NormalizationError::InvalidCryptographicHashLength);
                }
            }
            ProposedOperation::QuarantineEntity { entity_id, reason } => {
                if entity_id.is_empty() || !entity_id.starts_with("urn:entity:") {
                    return Err(NormalizationError::AmbiguousTargetNamespace);
                }
                if reason.len() > 256 {
                    return Err(NormalizationError::UnboundedParameterSize);
                }
            }
            ProposedOperation::RequestStateMutation {
                key,
                new_value_hash,
            } => {
                if key.is_empty() || !key.starts_with("urn:internal:") {
                    return Err(NormalizationError::AmbiguousTargetNamespace);
                }
                if new_value_hash.len() != 64 {
                    return Err(NormalizationError::InvalidCryptographicHashLength);
                }
            }
        }

        proposal
            .mark_validated()
            .map_err(|_| NormalizationError::InvalidLifecycleState)?;
        Ok(())
    }
}

// ============================================================================
// PAV SUITE EXTENSION (015-B)
// ============================================================================

#[cfg(test)]
mod pav_normalization_tests {
    use super::*;

    fn generate_draft_proposal(
        op: ProposedOperation,
        decision: DerivedPolicyDecision,
    ) -> GovernedActionProposal {
        GovernedActionProposal::new(
            ProposalId("PROP-NORM-01".to_string()),
            EpistemicObjectReference {
                object_digest: "digest".to_string(),
                verification_epoch: 1000,
            },
            PolicyEvaluationReference {
                rule_id: "RULE-01".to_string(),
                derived_decision: decision,
            },
            op,
            vec![],
        )
    }

    #[test]
    fn pav_02_intent_confusion_ambiguous_namespace() {
        let mut proposal = generate_draft_proposal(
            ProposedOperation::EmitNotification {
                target: "external-smtp-server".to_string(),
                message_hash: "a".repeat(64),
            },
            DerivedPolicyDecision::Permit,
        );

        let result = IntentNormalizer::normalize_and_validate(&mut proposal);
        assert_eq!(result, Err(NormalizationError::AmbiguousTargetNamespace));
        assert_eq!(proposal.status(), &ProposalStatus::Draft);
    }

    #[test]
    fn pav_02_intent_confusion_invalid_hash() {
        let mut proposal = generate_draft_proposal(
            ProposedOperation::RequestStateMutation {
                key: "urn:internal:state_key".to_string(),
                new_value_hash: "too_short".to_string(),
            },
            DerivedPolicyDecision::Permit,
        );

        let result = IntentNormalizer::normalize_and_validate(&mut proposal);
        assert_eq!(
            result,
            Err(NormalizationError::InvalidCryptographicHashLength)
        );
    }

    #[test]
    fn pav_06_semantic_contradiction_detection() {
        let mut proposal = generate_draft_proposal(
            ProposedOperation::RequestStateMutation {
                key: "urn:internal:critical_flag".to_string(),
                new_value_hash: "a".repeat(64),
            },
            DerivedPolicyDecision::Deny,
        );

        let result = IntentNormalizer::normalize_and_validate(&mut proposal);
        assert_eq!(result, Err(NormalizationError::SemanticContradiction));
    }

    #[test]
    fn pav_07_unbounded_parameter_rejection() {
        let mut proposal = generate_draft_proposal(
            ProposedOperation::QuarantineEntity {
                entity_id: "urn:entity:123".to_string(),
                reason: "a".repeat(300),
            },
            DerivedPolicyDecision::Quarantine,
        );

        let result = IntentNormalizer::normalize_and_validate(&mut proposal);
        assert_eq!(result, Err(NormalizationError::UnboundedParameterSize));
    }
}
