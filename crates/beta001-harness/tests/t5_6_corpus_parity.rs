use beta001_harness::corpus::CorpusCase;
use beta001_harness::evaluator::{evaluate_candidate, CandidateTrace, EvaluationProfile};
use beta001_harness::schema::TraceSchemaValidator;
use serde_json::Value;
use std::path::PathBuf;

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(relative)
}

fn validator() -> TraceSchemaValidator {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specifications/BETA-001-TRACE-v1.schema.json");

    TraceSchemaValidator::new(path).expect("schema must load")
}

fn load_candidate(relative: &str) -> CandidateTrace {
    let raw =
        std::fs::read_to_string(fixture_path(relative)).expect("candidate fixture must exist");

    let value: Value = serde_json::from_str(&raw).expect("candidate fixture must parse");

    CandidateTrace {
        run_id: value["run_id"].as_str().unwrap().to_string(),

        task_id: value["task_id"].as_str().unwrap().to_string(),

        events: value["events"].as_array().unwrap().clone(),
    }
}

#[test]
fn corpus_path_matches_legacy_candidate_evaluation() {
    let legacy_candidate = load_candidate("golden/golden_001_minimal_pass.json");

    let legacy_result = evaluate_candidate(
        &validator(),
        legacy_candidate,
        &EvaluationProfile::default(),
    )
    .expect("legacy path must evaluate");

    let corpus = CorpusCase::load_fixture(&fixture_path("corpus/scenario_001_minimal_pass.json"))
        .expect("corpus fixture must load");

    let candidate_fixture = corpus.parameters["candidate_fixture"]
        .as_str()
        .expect("candidate_fixture required");

    let corpus_candidate = load_candidate(candidate_fixture);

    let corpus_result = evaluate_candidate(
        &validator(),
        corpus_candidate,
        &EvaluationProfile::default(),
    )
    .expect("corpus path must evaluate");

    assert_eq!(legacy_result.report, corpus_result.report);
}
