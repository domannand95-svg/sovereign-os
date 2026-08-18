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

// Helper to load a candidate trace via the corpus envelope path
fn load_corpus_candidate(corpus_relative: &str) -> CandidateTrace {
    let corpus_path = get_manifest_path(corpus_relative);
    let corpus_case = CorpusCase::load_fixture(&corpus_path).expect("Corpus scenario must load");

    let candidate_ref = corpus_case.parameters["candidate_fixture"]
        .as_str()
        .unwrap();
    let raw = fs::read_to_string(get_manifest_path(&format!("fixtures/{}", candidate_ref)))
        .expect("candidate fixture must exist");
    let value: serde_json::Value =
        serde_json::from_str(&raw).expect("candidate fixture must parse");

    CandidateTrace {
        run_id: value["run_id"].as_str().unwrap().to_string(),
        task_id: value["task_id"].as_str().unwrap().to_string(),
        events: value["events"].as_array().unwrap().clone(),
    }
}

#[test]
fn golden_candidate_is_stable_across_100_evaluations() {
    let val = validator();
    let profile = EvaluationProfile::default();
    let trace = load_corpus_candidate("fixtures/corpus/scenario_001_minimal_pass.json");

    let baseline = evaluate_candidate(&val, trace.clone(), &profile)
        .expect("candidate evaluation must remain structurally valid");

    for i in 0..100 {
        let current = evaluate_candidate(&val, trace.clone(), &profile)
            .expect("candidate evaluation must remain structurally valid");
        assert_eq!(
            baseline.report.disposition, current.report.disposition,
            "Iteration {} drifted on disposition",
            i
        );
        assert_eq!(
            baseline.report.findings.len(),
            current.report.findings.len(),
            "Iteration {} drifted on finding count",
            i
        );
    }
}

#[test]
fn contained_adversarial_candidate_is_stable_across_100_evaluations() {
    let val = validator();
    let profile = EvaluationProfile::default();
    let trace = load_corpus_candidate("fixtures/corpus/scenario_adv_001_contained_forbidden.json");

    let baseline = evaluate_candidate(&val, trace.clone(), &profile)
        .expect("candidate evaluation must remain structurally valid");

    for i in 0..100 {
        let current = evaluate_candidate(&val, trace.clone(), &profile)
            .expect("candidate evaluation must remain structurally valid");
        assert_eq!(
            baseline.report.disposition, current.report.disposition,
            "Iteration {} drifted on adversarial disposition",
            i
        );
    }
}

#[test]
fn multi_finding_precedence_is_stable_across_100_evaluations() {
    let val = validator();
    let profile = EvaluationProfile::default();
    // Load err_006 base trace via corpus envelope
    let mut trace = load_corpus_candidate("fixtures/corpus/scenario_err_006_mutation_effect.json");

    // Apply the synthetic multi-finding ordering mutations
    if trace.events.len() >= 5 {
        trace.events[2]["seq"] = serde_json::Value::from(4_u64);
        trace.events[3]["seq"] = serde_json::Value::from(5_u64);
        trace.events[4]["seq"] = serde_json::Value::from(6_u64);
    }

    let baseline = evaluate_candidate(&val, trace.clone(), &profile)
        .expect("candidate evaluation must remain structurally valid");

    for i in 0..100 {
        let current = evaluate_candidate(&val, trace.clone(), &profile)
            .expect("candidate evaluation must remain structurally valid");
        assert_eq!(
            baseline.report.disposition, current.report.disposition,
            "Iteration {} drifted on mutated precedence disposition",
            i
        );
        assert_eq!(
            baseline.report.findings.len(),
            current.report.findings.len(),
            "Iteration {} drifted on mutated finding count",
            i
        );
    }
}

#[test]
fn candidate_fixture_determinism_matrix() {
    let val = validator();
    let profile = EvaluationProfile::default();

    // Verify matrix stability across multiple corpus-resolved traces
    let corpus_fixtures = vec![
        "fixtures/corpus/scenario_001_minimal_pass.json",
        "fixtures/corpus/scenario_002_repeated_stages.json",
        "fixtures/corpus/scenario_err_001_seq_gap.json",
    ];

    for fixture in corpus_fixtures {
        let trace = load_corpus_candidate(fixture);
        let first = evaluate_candidate(&val, trace.clone(), &profile);
        let second = evaluate_candidate(&val, trace, &profile);

        assert_eq!(
            format!("{:#?}", first),
            format!("{:#?}", second),
            "Determinism matrix failed for {}",
            fixture
        );
    }
}
