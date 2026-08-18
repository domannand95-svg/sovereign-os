use beta001_harness::corpus::CorpusCase;
use beta001_harness::evaluator::CandidateTrace;
use serde_json::Value;
use std::path::PathBuf;

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(relative)
}

fn load_candidate_fixture(relative: &str) -> CandidateTrace {
    let path = fixture_path(relative);
    let raw = std::fs::read_to_string(path).expect("candidate fixture must exist");

    let value: Value = serde_json::from_str(&raw).expect("candidate fixture must parse");

    CandidateTrace {
        run_id: value["run_id"]
            .as_str()
            .expect("candidate run_id required")
            .to_string(),

        task_id: value["task_id"]
            .as_str()
            .expect("candidate task_id required")
            .to_string(),

        events: value["events"]
            .as_array()
            .expect("candidate events required")
            .clone(),
    }
}

#[test]
fn corpus_fixture_resolves_candidate_without_identity_transfer() {
    let corpus_path = fixture_path("corpus/scenario_001_minimal_pass.json");

    let corpus = CorpusCase::load_fixture(&corpus_path).expect("corpus fixture must load");

    let candidate_fixture = corpus.parameters["candidate_fixture"]
        .as_str()
        .expect("candidate_fixture parameter required");

    let candidate = load_candidate_fixture(candidate_fixture);

    assert_eq!(candidate.run_id, "g01");
    assert_eq!(candidate.task_id, "task-001");

    assert_ne!(corpus.trace_id, candidate.run_id);
    assert_ne!(corpus.trace_id, candidate.task_id);

    assert!(!candidate.events.is_empty());
}
