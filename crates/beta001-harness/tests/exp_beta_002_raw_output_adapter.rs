use beta001_harness::raw_output_adapter::{
    NormalizationKind, RawOutputAdapter, RawOutputAdmission, RawOutputRejection,
};
use serde_json::Value;

const FIXTURE_001: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/001_clean_exact_match.txt",
);
const FIXTURE_002: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/002_markdown_fenced_json.txt",
);
const FIXTURE_003: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/003_missing_required_fields.txt",
);
const FIXTURE_004: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/004_hallucinated_schema_properties.txt",
);
const FIXTURE_005: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/005_trailing_garbage_text.txt",
);
const FIXTURE_006: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/006_adversarial_context_request.txt",
);
const FIXTURE_007: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/007_valid_context_request.txt",
);
const EXPECTED_OUTCOMES_V2: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/EXPECTED_OUTCOMES-v2.json",
);

const ALL_FIXTURES: [(&str, &[u8]); 7] = [
    ("001_clean_exact_match.txt", FIXTURE_001),
    ("002_markdown_fenced_json.txt", FIXTURE_002),
    ("003_missing_required_fields.txt", FIXTURE_003),
    ("004_hallucinated_schema_properties.txt", FIXTURE_004),
    ("005_trailing_garbage_text.txt", FIXTURE_005),
    ("006_adversarial_context_request.txt", FIXTURE_006),
    ("007_valid_context_request.txt", FIXTURE_007),
];

fn parse_json(raw: &[u8]) -> Value {
    serde_json::from_slice(raw).expect("frozen admitted fixture must parse as JSON")
}

fn assert_rejection(raw: &[u8], expected: RawOutputRejection) {
    let result = RawOutputAdapter::admit(raw);

    assert_eq!(
        result.raw_output.as_slice(),
        raw,
        "rejected raw bytes must be preserved byte-for-byte"
    );
    assert_eq!(
        result.admission,
        RawOutputAdmission::Reject { reason: expected }
    );
}

#[test]
fn frozen_fixture_oracles_match_adapter_results() {
    let result_001 = RawOutputAdapter::admit(FIXTURE_001);
    assert_eq!(result_001.raw_output.as_slice(), FIXTURE_001);
    match result_001.admission {
        RawOutputAdmission::Admit { candidate } => {
            assert_eq!(candidate, parse_json(FIXTURE_001));
        }
        other => panic!("fixture 001 expected Admit, observed {other:?}"),
    }

    let result_002 = RawOutputAdapter::admit(FIXTURE_002);
    assert_eq!(result_002.raw_output.as_slice(), FIXTURE_002);

    let prefix = b"```json\n";
    let suffix = b"\n```\n";
    assert!(FIXTURE_002.starts_with(prefix));
    assert!(FIXTURE_002.ends_with(suffix));

    let normalized_body = &FIXTURE_002[prefix.len()..FIXTURE_002.len() - suffix.len()];

    match result_002.admission {
        RawOutputAdmission::AdmitNormalized {
            candidate,
            normalization,
        } => {
            assert_eq!(
                normalization,
                NormalizationKind::ExactOuterJsonMarkdownFenceRemoval
            );
            assert_eq!(candidate, parse_json(normalized_body));
        }
        other => panic!("fixture 002 expected AdmitNormalized, observed {other:?}"),
    }

    assert_rejection(FIXTURE_003, RawOutputRejection::SchemaViolation);
    assert_rejection(FIXTURE_004, RawOutputRejection::SchemaViolation);
    assert_rejection(FIXTURE_005, RawOutputRejection::TrailingContent);
    assert_rejection(FIXTURE_006, RawOutputRejection::SchemaViolation);

    let result_007 = RawOutputAdapter::admit(FIXTURE_007);
    assert_eq!(result_007.raw_output.as_slice(), FIXTURE_007);
    match result_007.admission {
        RawOutputAdmission::Admit { candidate } => {
            assert_eq!(candidate, parse_json(FIXTURE_007));
        }
        other => panic!("fixture 007 expected Admit, observed {other:?}"),
    }
}

#[test]
fn every_fixture_preserves_original_raw_bytes() {
    for (name, raw) in ALL_FIXTURES {
        let result = RawOutputAdapter::admit(raw);
        assert_eq!(
            result.raw_output.as_slice(),
            raw,
            "raw evidence changed for {name}"
        );
    }
}

#[test]
fn identical_raw_bytes_replay_deterministically() {
    for (name, raw) in ALL_FIXTURES {
        let first = RawOutputAdapter::admit(raw);

        for replay in 0..100 {
            let observed = RawOutputAdapter::admit(raw);
            assert_eq!(
                observed, first,
                "determinism failure for {name} at replay {replay}"
            );
        }
    }
}

#[test]
fn corrected_expected_outcome_contract_matches_test_oracles() {
    let contract: Value = serde_json::from_slice(EXPECTED_OUTCOMES_V2)
        .expect("corrected frozen expected-outcome contract must parse");

    assert_eq!(contract["schema_version"], 2);
    assert_eq!(contract["experiment"], "EXP-BETA-002");

    let fixtures = contract["fixtures"]
        .as_array()
        .expect("contract fixtures must be an array");

    let expected = [
        ("001_clean_exact_match.txt", "ADMIT", "NONE"),
        (
            "002_markdown_fenced_json.txt",
            "ADMIT_NORMALIZED",
            "EXACT_OUTER_JSON_MARKDOWN_FENCE_REMOVAL",
        ),
        ("003_missing_required_fields.txt", "REJECT", "NONE"),
        ("004_hallucinated_schema_properties.txt", "REJECT", "NONE"),
        ("005_trailing_garbage_text.txt", "REJECT", "NONE"),
        ("006_adversarial_context_request.txt", "REJECT", "NONE"),
        ("007_valid_context_request.txt", "ADMIT", "NONE"),
    ];

    assert_eq!(fixtures.len(), expected.len());

    for (entry, (file, outcome, normalization)) in fixtures.iter().zip(expected) {
        assert_eq!(entry["file"], file);
        assert_eq!(entry["expected_outcome"], outcome);
        assert_eq!(entry["normalization"], normalization);
    }
}
