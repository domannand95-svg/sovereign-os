use beta001_harness::corpus::CorpusCase;
use beta001_harness::evaluator::{evaluate_candidate, CandidateTrace, EvaluationProfile};
use beta001_harness::schema::TraceSchemaValidator;
use std::fs;
use std::path::PathBuf;

fn get_manifest_path(relative: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative);
    path
}

fn validator() -> TraceSchemaValidator {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specifications/BETA-001-TRACE-v1.schema.json");
    TraceSchemaValidator::new(path).expect("schema must load")
}

fn load_candidate(relative: &str) -> CandidateTrace {
    let raw =
        fs::read_to_string(get_manifest_path(relative)).expect("candidate fixture must exist");
    let value: serde_json::Value =
        serde_json::from_str(&raw).expect("candidate fixture must parse");

    CandidateTrace {
        run_id: value["run_id"].as_str().unwrap().to_string(),
        task_id: value["task_id"].as_str().unwrap().to_string(),
        events: value["events"].as_array().unwrap().clone(),
    }
}

// Reusable parity assertion helper
fn assert_migration_parity(candidate_relative: &str, corpus_relative: &str) {
    let val = validator();
    let profile = EvaluationProfile::default();

    // PATH A: Legacy Direct Loading
    let legacy_trace = load_candidate(candidate_relative);
    let legacy_result = evaluate_candidate(&val, legacy_trace, &profile);

    // PATH B: Corpus-Mediated Orchestration
    let corpus_path = get_manifest_path(corpus_relative);
    let corpus_case = CorpusCase::load_fixture(&corpus_path).expect("Corpus scenario must load");

    let candidate_ref = corpus_case.parameters["candidate_fixture"]
        .as_str()
        .unwrap();
    let corpus_trace = load_candidate(&format!("fixtures/{}", candidate_ref));

    let corpus_result = evaluate_candidate(&val, corpus_trace, &profile);

    assert_eq!(
        format!("{:#?}", legacy_result),
        format!("{:#?}", corpus_result),
        "Evaluation drifted for fixture: {}",
        candidate_relative
    );
}

#[test]
fn migration_parity_err_001_seq_gap() {
    assert_migration_parity(
        "fixtures/runtime_violations/err_001_seq_gap.json",
        "fixtures/corpus/scenario_err_001_seq_gap.json",
    );
}

#[test]
fn migration_parity_err_002_stage_regression() {
    assert_migration_parity(
        "fixtures/runtime_violations/err_002_stage_regression.json",
        "fixtures/corpus/scenario_err_002_stage_regression.json",
    );
}

#[test]
fn migration_parity_err_003_missing_propose() {
    assert_migration_parity(
        "fixtures/runtime_violations/err_003_missing_propose.json",
        "fixtures/corpus/scenario_err_003_missing_propose.json",
    );
}

#[test]
fn migration_parity_err_004_expectation_mismatch() {
    assert_migration_parity(
        "fixtures/runtime_violations/err_004_expectation_mismatch.json",
        "fixtures/corpus/scenario_err_004_expectation_mismatch.json",
    );
}

#[test]
fn migration_parity_err_005_harness_gap() {
    assert_migration_parity(
        "fixtures/runtime_violations/err_005_harness_gap.json",
        "fixtures/corpus/scenario_err_005_harness_gap.json",
    );
}

#[test]
fn migration_parity_err_006_mutation_effect() {
    assert_migration_parity(
        "fixtures/runtime_violations/err_006_mutation_effect.json",
        "fixtures/corpus/scenario_err_006_mutation_effect.json",
    );
}
