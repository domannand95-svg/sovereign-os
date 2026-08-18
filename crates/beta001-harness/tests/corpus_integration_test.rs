use beta001_harness::evaluator::{
    evaluate_candidate, verify_finalized_trace, CandidateTrace, EvaluatedDisposition,
    EvaluationProfile, EvaluationReport, ExpectedClassification, Finding, FindingKind,
    ResultClassification,
};
use beta001_harness::schema::TraceSchemaValidator;
use serde_json::Value;
use std::path::PathBuf;

fn validator() -> TraceSchemaValidator {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specifications/BETA-001-TRACE-v1.schema.json");

    TraceSchemaValidator::new(path).expect("canonical BETA-001 trace schema must compile")
}

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(relative)
}

fn load_json(relative: &str) -> Value {
    let path = fixture_path(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()));

    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("failed to parse fixture {}: {error}", path.display()))
}

fn candidate_from_fixture(value: &Value) -> CandidateTrace {
    let run_id = value
        .get("run_id")
        .and_then(Value::as_str)
        .expect("candidate fixture must contain run_id")
        .to_string();

    let task_id = value
        .get("task_id")
        .and_then(Value::as_str)
        .expect("candidate fixture must contain task_id")
        .to_string();

    let events = value
        .get("events")
        .and_then(Value::as_array)
        .expect("candidate fixture must contain events")
        .clone();

    CandidateTrace {
        run_id,
        task_id,
        events,
    }
}

fn finding_kinds(findings: &[Finding]) -> Vec<FindingKind> {
    findings.iter().map(|finding| finding.kind).collect()
}

fn assert_zero_mutations(report: &EvaluationReport) {
    assert_eq!(report.counters.unauthorized_effects, 0);
    assert_eq!(report.counters.repository_mutations, 0);
    assert_eq!(report.counters.filesystem_mutations, 0);
    assert_eq!(report.counters.capability_mutations, 0);
    assert_eq!(report.counters.policy_mutations, 0);
    assert_eq!(report.counters.governed_state_mutations, 0);
}

#[test]
fn golden_001_minimal_pass() {
    let value = load_json("golden/golden_001_minimal_pass.json");

    let result = evaluate_candidate(
        &validator(),
        candidate_from_fixture(&value),
        &EvaluationProfile::default(),
    )
    .expect("golden_001 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Pass);
    assert!(result.report.findings.is_empty());
    assert_eq!(result.report.counters.unauthorized_effect_attempts, 0);
    assert_zero_mutations(&result.report);
}

#[test]
fn golden_002_repeated_stages() {
    let value = load_json("golden/golden_002_repeated_stages.json");

    let result = evaluate_candidate(
        &validator(),
        candidate_from_fixture(&value),
        &EvaluationProfile::default(),
    )
    .expect("golden_002 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Pass);
    assert!(result.report.findings.is_empty());
    assert_eq!(result.report.counters.unauthorized_effect_attempts, 0);
    assert_zero_mutations(&result.report);
}

#[test]
fn adv_001_contained_forbidden() {
    let value = load_json("adversarial/adv_001_contained_forbidden.json");

    let profile = EvaluationProfile {
        expectations: vec![ExpectedClassification {
            seq: 5,
            expected_result: ResultClassification::Forbidden,
        }],
    };

    let result = evaluate_candidate(&validator(), candidate_from_fixture(&value), &profile)
        .expect("adv_001 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Pass);
    assert!(result.report.findings.is_empty());
    assert_eq!(result.report.counters.unauthorized_effect_attempts, 1);
    assert_zero_mutations(&result.report);
}

#[test]
fn adv_002_tool_unavailable() {
    let value = load_json("adversarial/adv_002_tool_unavailable.json");

    let profile = EvaluationProfile {
        expectations: vec![ExpectedClassification {
            seq: 4,
            expected_result: ResultClassification::Unavailable,
        }],
    };

    let result = evaluate_candidate(&validator(), candidate_from_fixture(&value), &profile)
        .expect("adv_002 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Pass);
    assert!(result.report.findings.is_empty());
    assert_eq!(result.report.counters.unauthorized_effect_attempts, 1);
    assert_zero_mutations(&result.report);
}

#[test]
fn err_001_seq_gap() {
    let value = load_json("runtime_violations/err_001_seq_gap.json");

    let result = evaluate_candidate(
        &validator(),
        candidate_from_fixture(&value),
        &EvaluationProfile::default(),
    )
    .expect("err_001 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Fail);
    assert_eq!(
        finding_kinds(&result.report.findings),
        vec![
            FindingKind::Sequence,
            FindingKind::Sequence,
            FindingKind::Sequence,
        ]
    );
    assert_eq!(result.report.counters.unauthorized_effect_attempts, 0);
    assert_zero_mutations(&result.report);
}

#[test]
fn err_002_stage_regression() {
    let value = load_json("runtime_violations/err_002_stage_regression.json");

    let result = evaluate_candidate(
        &validator(),
        candidate_from_fixture(&value),
        &EvaluationProfile::default(),
    )
    .expect("err_002 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Fail);
    assert_eq!(
        finding_kinds(&result.report.findings),
        vec![FindingKind::StageOrder]
    );
    assert_zero_mutations(&result.report);
}

#[test]
fn err_003_missing_propose() {
    let value = load_json("runtime_violations/err_003_missing_propose.json");

    let result = evaluate_candidate(
        &validator(),
        candidate_from_fixture(&value),
        &EvaluationProfile::default(),
    )
    .expect("err_003 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Fail);
    assert_eq!(
        finding_kinds(&result.report.findings),
        vec![FindingKind::StageCompleteness]
    );
    assert_zero_mutations(&result.report);
}

#[test]
fn err_004_expectation_mismatch() {
    let value = load_json("runtime_violations/err_004_expectation_mismatch.json");

    let profile = EvaluationProfile {
        expectations: vec![ExpectedClassification {
            seq: 5,
            expected_result: ResultClassification::Forbidden,
        }],
    };

    let result = evaluate_candidate(&validator(), candidate_from_fixture(&value), &profile)
        .expect("err_004 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Fail);
    assert_eq!(
        finding_kinds(&result.report.findings),
        vec![FindingKind::Expectation]
    );
    assert_eq!(result.report.counters.unauthorized_effect_attempts, 1);
    assert_zero_mutations(&result.report);
}

#[test]
fn err_005_harness_gap() {
    let value = load_json("runtime_violations/err_005_harness_gap.json");

    let result = evaluate_candidate(
        &validator(),
        candidate_from_fixture(&value),
        &EvaluationProfile::default(),
    )
    .expect("err_005 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Fail);
    assert_eq!(
        finding_kinds(&result.report.findings),
        vec![FindingKind::HarnessGap]
    );
    assert_eq!(result.report.counters.unauthorized_effect_attempts, 1);
    assert_zero_mutations(&result.report);
}

#[test]
fn err_006_mutation_effect() {
    let value = load_json("runtime_violations/err_006_mutation_effect.json");

    let result = evaluate_candidate(
        &validator(),
        candidate_from_fixture(&value),
        &EvaluationProfile::default(),
    )
    .expect("err_006 must remain structurally valid");

    assert_eq!(result.report.disposition, EvaluatedDisposition::Fail);
    assert_eq!(
        finding_kinds(&result.report.findings),
        vec![FindingKind::Mutation]
    );

    assert_eq!(result.report.counters.unauthorized_effect_attempts, 0);
    assert_eq!(result.report.counters.unauthorized_effects, 1);
    assert_eq!(result.report.counters.repository_mutations, 1);
    assert_eq!(result.report.counters.filesystem_mutations, 0);
    assert_eq!(result.report.counters.capability_mutations, 0);
    assert_eq!(result.report.counters.policy_mutations, 0);
    assert_eq!(result.report.counters.governed_state_mutations, 0);
}

#[test]
fn int_001_counter_tamper() {
    let value = load_json("integrity_violations/int_001_counter_tamper.json");

    let report = verify_finalized_trace(&validator(), &value, &EvaluationProfile::default())
        .expect("int_001 must remain structurally valid");

    assert_eq!(report.disposition, EvaluatedDisposition::Fail);
    assert_eq!(
        finding_kinds(&report.findings),
        vec![
            FindingKind::Mutation,
            FindingKind::CounterMismatch,
            FindingKind::CounterMismatch,
            FindingKind::CounterMismatch,
        ]
    );

    assert_eq!(report.counters.unauthorized_effect_attempts, 0);
    assert_eq!(report.counters.unauthorized_effects, 1);
    assert_eq!(report.counters.repository_mutations, 1);
}

#[test]
fn int_002_disposition_tamper() {
    let value = load_json("integrity_violations/int_002_disposition_tamper.json");

    let report = verify_finalized_trace(&validator(), &value, &EvaluationProfile::default())
        .expect("int_002 must remain structurally valid");

    assert_eq!(report.disposition, EvaluatedDisposition::Fail);
    assert_eq!(
        finding_kinds(&report.findings),
        vec![FindingKind::StageCompleteness, FindingKind::CounterMismatch,]
    );

    assert_eq!(report.counters.unauthorized_effect_attempts, 0);
    assert_zero_mutations(&report);
}

fn profile_for_candidate_fixture(relative: &str) -> EvaluationProfile {
    match relative {
        "adversarial/adv_001_contained_forbidden.json" => EvaluationProfile {
            expectations: vec![ExpectedClassification {
                seq: 5,
                expected_result: ResultClassification::Forbidden,
            }],
        },
        "adversarial/adv_002_tool_unavailable.json" => EvaluationProfile {
            expectations: vec![ExpectedClassification {
                seq: 4,
                expected_result: ResultClassification::Unavailable,
            }],
        },
        "runtime_violations/err_004_expectation_mismatch.json" => EvaluationProfile {
            expectations: vec![ExpectedClassification {
                seq: 5,
                expected_result: ResultClassification::Forbidden,
            }],
        },
        _ => EvaluationProfile::default(),
    }
}

fn assert_candidate_determinism(relative: &str, profile: &EvaluationProfile, repetitions: usize) {
    let schema_validator = validator();
    let value = load_json(relative);

    let baseline = evaluate_candidate(&schema_validator, candidate_from_fixture(&value), profile)
        .unwrap_or_else(|error| {
            panic!(
                "{relative} must remain structurally valid during determinism baseline: {}",
                error.detail
            )
        });

    schema_validator
        .validate_structure(&baseline.trace)
        .unwrap_or_else(|error| {
            panic!("{relative} synthesized baseline trace must remain canonically valid: {error}")
        });

    let baseline_bytes = serde_json::to_vec(&baseline.trace)
        .expect("finalized baseline trace must serialize deterministically");

    for iteration in 0..repetitions {
        let actual = evaluate_candidate(&schema_validator, candidate_from_fixture(&value), profile)
            .unwrap_or_else(|error| {
                panic!(
                    "{relative} determinism iteration {iteration} structurally rejected: {}",
                    error.detail
                )
            });

        schema_validator
            .validate_structure(&actual.trace)
            .unwrap_or_else(|error| {
                panic!(
                    "{relative} determinism iteration {iteration} emitted invalid trace: {error}"
                )
            });

        assert_eq!(
            &actual.trace, &baseline.trace,
            "{relative} finalized Value changed at iteration {iteration}"
        );

        assert_eq!(
            &actual.report, &baseline.report,
            "{relative} EvaluationReport changed at iteration {iteration}"
        );

        let actual_bytes = serde_json::to_vec(&actual.trace)
            .expect("repeated finalized trace must serialize deterministically");

        assert_eq!(
            actual_bytes, baseline_bytes,
            "{relative} serialized finalized trace changed at iteration {iteration}"
        );
    }
}

#[test]
fn candidate_fixture_determinism_matrix() {
    let candidates = [
        "golden/golden_001_minimal_pass.json",
        "golden/golden_002_repeated_stages.json",
        "adversarial/adv_001_contained_forbidden.json",
        "adversarial/adv_002_tool_unavailable.json",
        "runtime_violations/err_001_seq_gap.json",
        "runtime_violations/err_002_stage_regression.json",
        "runtime_violations/err_003_missing_propose.json",
        "runtime_violations/err_004_expectation_mismatch.json",
        "runtime_violations/err_005_harness_gap.json",
        "runtime_violations/err_006_mutation_effect.json",
    ];

    for relative in candidates {
        let profile = profile_for_candidate_fixture(relative);
        assert_candidate_determinism(relative, &profile, 1);
    }
}

#[test]
fn golden_candidate_is_stable_across_100_evaluations() {
    assert_candidate_determinism(
        "golden/golden_001_minimal_pass.json",
        &EvaluationProfile::default(),
        100,
    );
}

#[test]
fn contained_adversarial_candidate_is_stable_across_100_evaluations() {
    let profile = EvaluationProfile {
        expectations: vec![ExpectedClassification {
            seq: 5,
            expected_result: ResultClassification::Forbidden,
        }],
    };

    assert_candidate_determinism(
        "adversarial/adv_001_contained_forbidden.json",
        &profile,
        100,
    );
}

#[test]
fn multi_finding_precedence_is_stable_across_100_evaluations() {
    let schema_validator = validator();
    let value = load_json("runtime_violations/err_006_mutation_effect.json");

    let mut candidate = candidate_from_fixture(&value);

    candidate.events[2]["seq"] = Value::from(4_u64);
    candidate.events[3]["seq"] = Value::from(5_u64);
    candidate.events[4]["seq"] = Value::from(6_u64);

    let profile = EvaluationProfile::default();

    let baseline = evaluate_candidate(&schema_validator, candidate.clone(), &profile)
        .expect("combined sequence-gap + mutation candidate must remain structurally valid");

    assert_eq!(
        finding_kinds(&baseline.report.findings),
        vec![
            FindingKind::Sequence,
            FindingKind::Sequence,
            FindingKind::Sequence,
            FindingKind::Mutation,
        ]
    );

    assert_eq!(baseline.report.disposition, EvaluatedDisposition::Fail);

    schema_validator
        .validate_structure(&baseline.trace)
        .expect("combined multi-finding finalized trace must remain canonically valid");

    let baseline_bytes = serde_json::to_vec(&baseline.trace)
        .expect("combined multi-finding baseline must serialize");

    for iteration in 0..100 {
        let actual = evaluate_candidate(&schema_validator, candidate.clone(), &profile)
            .unwrap_or_else(|error| {
                panic!(
                    "combined multi-finding iteration {iteration} structurally rejected: {}",
                    error.detail
                )
            });

        assert_eq!(
            finding_kinds(&actual.report.findings),
            vec![
                FindingKind::Sequence,
                FindingKind::Sequence,
                FindingKind::Sequence,
                FindingKind::Mutation,
            ],
            "finding precedence changed at iteration {iteration}"
        );

        assert_eq!(
            &actual.trace, &baseline.trace,
            "multi-finding finalized Value changed at iteration {iteration}"
        );

        assert_eq!(
            &actual.report, &baseline.report,
            "multi-finding EvaluationReport changed at iteration {iteration}"
        );

        schema_validator
            .validate_structure(&actual.trace)
            .unwrap_or_else(|error| {
                panic!("multi-finding iteration {iteration} emitted invalid trace: {error}")
            });

        let actual_bytes =
            serde_json::to_vec(&actual.trace).expect("multi-finding trace must serialize");

        assert_eq!(
            actual_bytes, baseline_bytes,
            "multi-finding serialized trace changed at iteration {iteration}"
        );
    }
}
