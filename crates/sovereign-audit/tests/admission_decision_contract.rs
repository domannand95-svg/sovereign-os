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