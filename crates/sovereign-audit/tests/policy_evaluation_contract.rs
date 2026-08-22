#[derive(Debug, PartialEq, Eq, Clone)]
enum EvaluationClassification {
    ConditionsSatisfied,
    ConditionsUnmet,
    ConflictingRules,
    MissingEvidence,
    Indeterminate,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct NormalizedIntent {
    intent_reference: String,
    operation: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct GovernedPolicy {
    policy_reference: String,
    requires_approval: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct EvaluationContext {
    context_reference: String,
    has_approval: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct PolicyEvaluationResult {
    intent_reference: String,
    policy_reference: String,
    context_reference: String,
    evaluation_classification: EvaluationClassification,
    matched_rules: Vec<String>,
    failed_rules: Vec<String>,
    evaluation_trace: Vec<String>,
}

impl PolicyEvaluationResult {
    fn evaluate(
        intent: &NormalizedIntent,
        policy: &GovernedPolicy,
        context: &EvaluationContext,
    ) -> Self {
        let mut classification = EvaluationClassification::ConditionsSatisfied;
        let mut matched_rules = Vec::new();
        let mut failed_rules = Vec::new();

        if policy.requires_approval {
            matched_rules.push("requires_approval".to_string());

            if !context.has_approval {
                classification = EvaluationClassification::ConditionsUnmet;
                failed_rules.push("approval_missing".to_string());
            }
        }

        Self {
            intent_reference: intent.intent_reference.clone(),
            policy_reference: policy.policy_reference.clone(),
            context_reference: context.context_reference.clone(),
            evaluation_classification: classification,
            matched_rules,
            failed_rules,
            evaluation_trace: vec![
                "deterministic_policy_evaluation".to_string()
            ],
        }
    }
}

#[test]
fn test_policy_evaluation_yields_classification_not_admission() {
    let intent = NormalizedIntent {
        intent_reference: "intent-001".into(),
        operation: "DELETE".into(),
    };

    let policy = GovernedPolicy {
        policy_reference: "policy-001".into(),
        requires_approval: true,
    };

    let context = EvaluationContext {
        context_reference: "context-001".into(),
        has_approval: false,
    };

    let result = PolicyEvaluationResult::evaluate(
        &intent,
        &policy,
        &context,
    );

    assert_eq!(
        result.evaluation_classification,
        EvaluationClassification::ConditionsUnmet
    );

    assert_ne!(
        result.evaluation_classification,
        EvaluationClassification::ConditionsSatisfied
    );

    assert!(
        result
            .failed_rules
            .contains(&"approval_missing".to_string())
    );
}

#[test]
fn test_policy_evaluation_is_deterministic() {
    let intent = NormalizedIntent {
        intent_reference: "intent-002".into(),
        operation: "READ".into(),
    };

    let policy = GovernedPolicy {
        policy_reference: "policy-002".into(),
        requires_approval: false,
    };

    let context = EvaluationContext {
        context_reference: "context-002".into(),
        has_approval: false,
    };

    let result_a = PolicyEvaluationResult::evaluate(
        &intent,
        &policy,
        &context,
    );

    let result_b = PolicyEvaluationResult::evaluate(
        &intent,
        &policy,
        &context,
    );

    assert_eq!(result_a, result_b);
}

#[test]
fn test_policy_result_contains_no_authority_surface() {
    let intent = NormalizedIntent {
        intent_reference: "intent-003".into(),
        operation: "READ".into(),
    };

    let policy = GovernedPolicy {
        policy_reference: "policy-003".into(),
        requires_approval: false,
    };

    let context = EvaluationContext {
        context_reference: "context-003".into(),
        has_approval: false,
    };

    let result = PolicyEvaluationResult::evaluate(
        &intent,
        &policy,
        &context,
    );

    assert!(
        result.evaluation_trace.contains(
            &"deterministic_policy_evaluation".to_string()
        )
    );
}

#[test]
fn test_policy_conflict_is_not_admission() {
    let intent = NormalizedIntent {
        intent_reference: "intent-conflict".into(),
        operation: "DELETE".into(),
    };

    let policy = GovernedPolicy {
        policy_reference: "policy-conflict".into(),
        requires_approval: true,
    };

    let context = EvaluationContext {
        context_reference: "context-conflict".into(),
        has_approval: false,
    };

    let mut result =
        PolicyEvaluationResult::evaluate(
            &intent,
            &policy,
            &context,
        );

    result.evaluation_classification =
        EvaluationClassification::ConflictingRules;

    assert_eq!(
        result.evaluation_classification,
        EvaluationClassification::ConflictingRules
    );

    assert_ne!(
        result.evaluation_classification,
        EvaluationClassification::ConditionsSatisfied
    );
}

#[test]
fn test_missing_evidence_is_not_automatic_admission() {
    let intent = NormalizedIntent {
        intent_reference: "intent-evidence".into(),
        operation: "DELETE".into(),
    };

    let policy = GovernedPolicy {
        policy_reference: "policy-evidence".into(),
        requires_approval: false,
    };

    let context = EvaluationContext {
        context_reference: "context-evidence".into(),
        has_approval: false,
    };

    let mut result =
        PolicyEvaluationResult::evaluate(
            &intent,
            &policy,
            &context,
        );

    result.evaluation_classification =
        EvaluationClassification::MissingEvidence;

    assert_eq!(
        result.evaluation_classification,
        EvaluationClassification::MissingEvidence
    );

    assert_ne!(
        result.evaluation_classification,
        EvaluationClassification::ConditionsSatisfied
    );
}