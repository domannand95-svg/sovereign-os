use crate::{DerivedPolicyDecision, GovernedActionProposal, ProposalStatus, ProposedOperation};

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

        if proposal.policy_context.derived_decision == DerivedPolicyDecision::Deny {
            if let ProposedOperation::RequestStateMutation { .. } = proposal.proposed_operation {
                return Err(NormalizationError::SemanticContradiction);
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EpistemicObjectReference, PolicyEvaluationReference, ProposalId};

    fn draft_proposal(
        operation: ProposedOperation,
        decision: DerivedPolicyDecision,
    ) -> GovernedActionProposal {
        GovernedActionProposal::new(
            ProposalId("PROP-NORM-001".to_string()),
            EpistemicObjectReference {
                object_digest: "digest".to_string(),
                verification_epoch: 1,
            },
            PolicyEvaluationReference {
                rule_id: "RULE-001".to_string(),
                derived_decision: decision,
            },
            operation,
            vec![],
        )
    }

    #[test]
    fn ambiguous_namespace_is_rejected() {
        let mut proposal = draft_proposal(
            ProposedOperation::EmitNotification {
                target: "external-system".to_string(),
                message_hash: "a".repeat(64),
            },
            DerivedPolicyDecision::Permit,
        );

        assert_eq!(
            IntentNormalizer::normalize_and_validate(&mut proposal),
            Err(NormalizationError::AmbiguousTargetNamespace)
        );

        assert_eq!(proposal.status(), &ProposalStatus::Draft);
    }

    #[test]
    fn invalid_hash_is_rejected() {
        let mut proposal = draft_proposal(
            ProposedOperation::RequestStateMutation {
                key: "urn:internal:key".to_string(),
                new_value_hash: "short".to_string(),
            },
            DerivedPolicyDecision::Permit,
        );

        assert_eq!(
            IntentNormalizer::normalize_and_validate(&mut proposal),
            Err(NormalizationError::InvalidCryptographicHashLength)
        );
    }

    #[test]
    fn denied_mutation_is_semantically_invalid() {
        let mut proposal = draft_proposal(
            ProposedOperation::RequestStateMutation {
                key: "urn:internal:key".to_string(),
                new_value_hash: "a".repeat(64),
            },
            DerivedPolicyDecision::Deny,
        );

        assert_eq!(
            IntentNormalizer::normalize_and_validate(&mut proposal),
            Err(NormalizationError::SemanticContradiction)
        );
    }

    #[test]
    fn valid_intent_becomes_validated_proposal_only() {
        let mut proposal = draft_proposal(
            ProposedOperation::EmitNotification {
                target: "urn:internal:log".to_string(),
                message_hash: "a".repeat(64),
            },
            DerivedPolicyDecision::Permit,
        );

        assert_eq!(
            IntentNormalizer::normalize_and_validate(&mut proposal),
            Ok(())
        );

        assert_eq!(proposal.status(), &ProposalStatus::Validated);
    }
}
