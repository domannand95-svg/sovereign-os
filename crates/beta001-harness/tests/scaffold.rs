use beta001_harness::schema::TraceSchemaValidator;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root must resolve")
}

fn validator() -> TraceSchemaValidator {
    TraceSchemaValidator::new(repo_root().join("docs/specifications/BETA-001-TRACE-v1.schema.json"))
        .expect("canonical BETA-001 schema must compile")
}

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/scaffold")
        .join(name);

    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));

    serde_json::from_str(&text).unwrap_or_else(|error| panic!("failed to parse {path:?}: {error}"))
}

#[test]
fn canonical_schema_loads() {
    let _ = validator();
}

#[test]
fn schema_valid_runtime_incomplete_trace_is_structurally_accepted() {
    let trace = fixture("schema_valid_runtime_incomplete.json");

    validator()
        .validate_structure(&trace)
        .expect("structural schema validation should succeed");
}

#[test]
fn missing_final_is_structurally_rejected() {
    let trace = fixture("missing_final.json");

    assert!(validator().validate_structure(&trace).is_err());
}

#[test]
fn malformed_timestamp_is_structurally_rejected() {
    let trace = fixture("invalid_timestamp.json");

    assert!(validator().validate_structure(&trace).is_err());
}
