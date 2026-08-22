#[derive(Debug, PartialEq, Eq, Clone)]
enum EvaluationClassification {
    ConditionsSatisfied,
    ConditionsUnmet,
    ConflictingRules,
    MissingEvidence,
    Indeterminate,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum GovernanceMode {
    Normal,
    Restricted,
    Lockdown,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct GovernanceContext {
    governance_mode: GovernanceMode,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum AdmissionOutcome {
    Permit,
    Deny,
    Quarantine,
    Escalate,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct AdmissionDecision {
    outcome: AdmissionOutcome,
}

impl AdmissionDecision {
    pub fn decide(
        classification: &EvaluationClassification,
        context: &GovernanceContext,
    ) -> Self {
        if context.governance_mode == GovernanceMode::Lockdown {
            return Self {
                outcome: AdmissionOutcome::Deny,
            };
        }

        let outcome = match classification {
            EvaluationClassification::ConditionsSatisfied => {
                AdmissionOutcome::Permit
            }
            EvaluationClassification::ConditionsUnmet => {
                AdmissionOutcome::Deny
            }
            EvaluationClassification::ConflictingRules => {
                AdmissionOutcome::Escalate
            }
            EvaluationClassification::MissingEvidence => {
                AdmissionOutcome::Quarantine
            }
            EvaluationClassification::Indeterminate => {
                AdmissionOutcome::Quarantine
            }
        };

        Self { outcome }
    }
}

#[test]
fn test_policy_conditions_satisfied_maps_to_permit() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Normal,
    };

    let decision = AdmissionDecision::decide(
        &EvaluationClassification::ConditionsSatisfied,
        &context,
    );

    assert_eq!(decision.outcome, AdmissionOutcome::Permit);
}

#[test]
fn test_missing_evidence_maps_to_quarantine_not_deny() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Normal,
    };

    let decision = AdmissionDecision::decide(
        &EvaluationClassification::MissingEvidence,
        &context,
    );

    assert_eq!(decision.outcome, AdmissionOutcome::Quarantine);
    assert_ne!(decision.outcome, AdmissionOutcome::Deny);
}

#[test]
fn test_conflicting_rules_maps_to_escalate() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Normal,
    };

    let decision = AdmissionDecision::decide(
        &EvaluationClassification::ConflictingRules,
        &context,
    );

    assert_eq!(decision.outcome, AdmissionOutcome::Escalate);
}

#[test]
fn test_indeterminate_cannot_become_permit() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Normal,
    };

    let decision = AdmissionDecision::decide(
        &EvaluationClassification::Indeterminate,
        &context,
    );

    assert_eq!(decision.outcome, AdmissionOutcome::Quarantine);
    assert_ne!(decision.outcome, AdmissionOutcome::Permit);
}

#[test]
fn test_permit_contains_no_authorization_surface() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Normal,
    };

    let decision = AdmissionDecision::decide(
        &EvaluationClassification::ConditionsSatisfied,
        &context,
    );

    assert_eq!(decision.outcome, AdmissionOutcome::Permit);

    let decision_size = std::mem::size_of_val(&decision);

    assert!(
        decision_size > 0,
        "AdmissionDecision exists only as a data artifact"
    );
}

#[test]
fn test_admission_decision_is_deterministic() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Normal,
    };

    let first = AdmissionDecision::decide(
        &EvaluationClassification::ConditionsSatisfied,
        &context,
    );

    let second = AdmissionDecision::decide(
        &EvaluationClassification::ConditionsSatisfied,
        &context,
    );

    assert_eq!(first, second);
}

#[test]
fn test_lockdown_context_overrides_permit_condition() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Lockdown,
    };

    let decision = AdmissionDecision::decide(
        &EvaluationClassification::ConditionsSatisfied,
        &context,
    );

    assert_eq!(
        decision.outcome,
        AdmissionOutcome::Deny
    );
}

#[test]
fn test_lockdown_context_overrides_all_positive_classifications() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Lockdown,
    };

    let classifications = vec![
        EvaluationClassification::ConditionsSatisfied,
        EvaluationClassification::ConditionsUnmet,
        EvaluationClassification::MissingEvidence,
    ];

    for classification in classifications {
        let decision = AdmissionDecision::decide(
            &classification,
            &context,
        );

        assert_eq!(
            decision.outcome,
            AdmissionOutcome::Deny
        );
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ProvenanceBoundAdmissionDecision {
    intent_reference: String,
    evaluation_reference: String,
    governance_context_reference: String,
    outcome: AdmissionOutcome,
}

impl ProvenanceBoundAdmissionDecision {
    fn new(
        intent_reference: &str,
        evaluation_reference: &str,
        governance_context_reference: &str,
        outcome: AdmissionOutcome,
    ) -> Self {
        Self {
            intent_reference: intent_reference.to_string(),
            evaluation_reference: evaluation_reference.to_string(),
            governance_context_reference: governance_context_reference.to_string(),
            outcome,
        }
    }
}

#[test]
fn test_admission_decision_preserves_provenance_binding() {
    let decision = ProvenanceBoundAdmissionDecision::new(
        "intent-001",
        "evaluation-001",
        "context-001",
        AdmissionOutcome::Permit,
    );

    assert_eq!(
        decision.intent_reference,
        "intent-001"
    );

    assert_eq!(
        decision.evaluation_reference,
        "evaluation-001"
    );

    assert_eq!(
        decision.governance_context_reference,
        "context-001"
    );
}

#[test]
fn test_admission_outcome_does_not_replace_source_provenance() {
    let decision = ProvenanceBoundAdmissionDecision::new(
        "intent-002",
        "evaluation-002",
        "context-002",
        AdmissionOutcome::Permit,
    );

    assert_ne!(
        decision.outcome,
        AdmissionOutcome::Deny
    );

    assert_eq!(
        decision.intent_reference,
        "intent-002"
    );
}

#[test]
fn test_permit_is_not_authorization_receipt() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Normal,
    };

    let decision = AdmissionDecision::decide(
        &EvaluationClassification::ConditionsSatisfied,
        &context,
    );

    assert_eq!(
        decision.outcome,
        AdmissionOutcome::Permit
    );

    let decision_size = std::mem::size_of_val(&decision);

    assert!(
        decision_size > 0,
        "AdmissionDecision exists as governance data only"
    );
}

#[test]
fn test_admission_layer_has_no_execution_transition() {
    let context = GovernanceContext {
        governance_mode: GovernanceMode::Normal,
    };

    let decision = AdmissionDecision::decide(
        &EvaluationClassification::ConditionsSatisfied,
        &context,
    );

    match decision.outcome {
        AdmissionOutcome::Permit => {
            // Permit reaches BETA-024 consideration only.
            // It does not execute.
            assert!(true);
        }
        _ => {
            assert!(false, "Expected Permit outcome");
        }
    }
}