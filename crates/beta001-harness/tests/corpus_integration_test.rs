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
