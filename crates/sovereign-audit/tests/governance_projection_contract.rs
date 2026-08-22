use sovereign_audit::governance_admission::AdmissionOutcome;
use sovereign_audit::governance_projection::{AuthorityContext, GovernanceProjectionAdapter};
use sovereign_policy::{
    DerivedPolicyDecision, EpistemicObjectReference, GovernedActionProposal,
    PolicyEvaluationReference, PolicyEvaluationResult, ProposalId, ProposedOperation,
};

fn proposal() -> GovernedActionProposal {
    GovernedActionProposal::new(
        ProposalId("PROP-BETA016-001".to_string()),
        EpistemicObjectReference {
            object_digest: "digest".to_string(),
            verification_epoch: 1,
        },
        PolicyEvaluationReference {
            rule_id: "RULE-001".to_string(),
            derived_decision: DerivedPolicyDecision::Permit,
        },
        ProposedOperation::EmitNotification {
            target: "urn:internal:test".to_string(),
            message_hash: "a".repeat(64),
        },
        vec![],
    )
}

#[test]
fn projection_preserves_policy_decision_without_authority_expansion() {
    let evaluation = PolicyEvaluationResult {
        evaluated_rule_id: "RULE-001".to_string(),
        decision: DerivedPolicyDecision::Permit,
        authority_expansion: 0,
    };

    let authority = AuthorityContext {
        context_reference: "CTX-001".to_string(),
    };

    let result = GovernanceProjectionAdapter::project(&proposal(), &evaluation, &authority)
        .expect("projection should succeed");

    assert_eq!(result.outcome, AdmissionOutcome::Permit);
    assert_eq!(result.authorized_scope, "none");
}

#[test]
fn projection_does_not_execute_or_create_capability() {
    let evaluation = PolicyEvaluationResult {
        evaluated_rule_id: "RULE-001".to_string(),
        decision: DerivedPolicyDecision::Deny,
        authority_expansion: 0,
    };

    let authority = AuthorityContext {
        context_reference: "CTX-001".to_string(),
    };

    let result = GovernanceProjectionAdapter::project(&proposal(), &evaluation, &authority)
        .expect("projection should preserve denial");

    assert_eq!(result.outcome, AdmissionOutcome::Deny);
    assert_eq!(result.authorized_scope, "none");
}
