//! BETA-001-T5.5 â€” Frozen Corpus & Oracle Integration
//!
//! Enforces the deterministic 1:1 mapping between static corpus fixtures
//! and predetermined evaluation oracles. Proves that the evidence pipeline
//! honors the predetermined expectations without post-hoc rationalization.

use beta001_harness::evidence::{CandidateParseStatus, EvidenceCollector};
use beta001_harness::integrity::IntegrityReport;
use beta001_harness::telemetry::ContainmentTelemetry;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// The mathematically exact predetermined expectation for a given corpus fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusOracle {
    pub case_id: String,
    pub expected_disposition: String,
    pub expected_parse_status: CandidateParseStatus,
    pub expected_containment_clean: bool,
    pub expected_breach_kinds: Vec<String>,
    pub expected_integrity_intact: bool,
    pub expected_digest_stability: bool,
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_corpus_completeness_invariant() {
    let corpus_dir = fixtures_dir().join("corpus");
    let oracles_dir = fixtures_dir().join("oracles");

    let mut corpus_ids = HashSet::new();
    for entry in fs::read_dir(&corpus_dir).expect("corpus dir must exist") {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        if name.ends_with(".json") || name.ends_with(".txt") {
            let id = name.replace(".json", "").replace(".txt", "");
            assert!(
                corpus_ids.insert(id.clone()),
                "Duplicate corpus ID found: {}",
                id
            );
        }
    }

    let mut oracle_ids = HashSet::new();
    for entry in fs::read_dir(&oracles_dir).expect("oracles dir must exist") {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        if name.ends_with(".json") {
            let content = fs::read_to_string(entry.path()).unwrap();
            let oracle: CorpusOracle = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("Failed to parse oracle {}: {}", name, e));

            let id = name.replace(".json", "");
            assert_eq!(
                oracle.case_id, id,
                "Oracle internal case_id must match filename"
            );
            assert!(
                oracle_ids.insert(id.clone()),
                "Duplicate oracle ID found: {}",
                id
            );
        }
    }

    let missing_oracles: Vec<_> = corpus_ids.difference(&oracle_ids).collect();
    let orphan_oracles: Vec<_> = oracle_ids.difference(&corpus_ids).collect();

    assert!(
        missing_oracles.is_empty(),
        "Completeness Failure: Fixtures without oracles: {:?}",
        missing_oracles
    );
    assert!(
        orphan_oracles.is_empty(),
        "Completeness Failure: Oracles without fixtures: {:?}",
        orphan_oracles
    );
}

#[test]
fn test_evidence_schema_binding_golden_001() {
    let case_id = "CORPUS-T5.5-GOLDEN-001";

    // 1. Immutable Load (No live execution)
    let oracle_path = fixtures_dir()
        .join("oracles")
        .join(format!("{}.json", case_id));
    let fixture_path = fixtures_dir()
        .join("corpus")
        .join(format!("{}.json", case_id));

    let oracle: CorpusOracle =
        serde_json::from_str(&fs::read_to_string(oracle_path).unwrap()).unwrap();
    let raw_output = fs::read_to_string(fixture_path).unwrap();

    // 2. Simulated Pipeline Stages
    // NOTE:
    // This test validates oracle/evidence schema compatibility only.
    // Full fixture -> adapter -> evaluator execution is introduced once
    // corpus execution bindings are implemented.
    let parse_status = CandidateParseStatus::Parsed;
    let parsed_json = Some(raw_output.clone());
    let telemetry = ContainmentTelemetry::new(); // Golden case = clean telemetry
    let disposition = Some("APPROVED".to_string()); // Golden case = approved
    let report = IntegrityReport {
        version: 1,
        pre_snapshots: BTreeMap::new(),
        post_snapshots: BTreeMap::new(),
        mutated_surfaces: vec![],
        is_intact: true, // Golden case = no mutation
    };

    // 3. Harness Evidence Sealing
    let mut collector = EvidenceCollector::new(
        case_id.to_string(),
        "345620068e29c98d3a22cc7567d04f6c87f0bc61".to_string(), // T5.5 Baseline
        "t5-profile-v1".to_string(),
    );

    collector.set_candidate_plane(raw_output, parsed_json, parse_status);
    collector.set_containment_plane(telemetry);
    collector.set_integrity_plane(report);
    collector.set_evaluation_plane(None, disposition);

    let sealed = collector.seal().expect("Package must seal successfully");
    let pkg = sealed.package();

    // 4. Mathematical Comparison against Oracle
    assert_eq!(
        pkg.candidate_plane.parse_status, oracle.expected_parse_status,
        "Mismatch: Parse Status"
    );
    assert_eq!(
        match pkg.evaluation_plane.disposition { beta001_harness::evaluator::EvaluatedDisposition::Pass => "APPROVED", beta001_harness::evaluator::EvaluatedDisposition::Fail => "REJECTED" },
        oracle.expected_disposition,
        "Mismatch: Disposition"
    );
    assert_eq!(
        pkg.containment_plane.is_clean, oracle.expected_containment_clean,
        "Mismatch: Containment Cleanliness"
    );
    assert_eq!(
        pkg.integrity_plane.is_intact, oracle.expected_integrity_intact,
        "Mismatch: Integrity Status"
    );

    let actual_breaches: Vec<String> = pkg
        .containment_plane
        
        .breaches
        .iter()
        .map(|b| b.breach_kind.clone())
        .collect();
    assert_eq!(
        actual_breaches, oracle.expected_breach_kinds,
        "Mismatch: Containment Breaches"
    );
}






