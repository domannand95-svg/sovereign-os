use jsonschema::Validator;
use serde_json::Value;
use std::fs;
use std::path::Path;

fn load_schema() -> Validator {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let schema_path = Path::new(manifest_dir)
        .join(r#"../../docs/specifications/schemas/REQUEST_EVALUATION_RESULT-v1.schema.json"#);

    let schema_path = if schema_path.exists() {
        schema_path.canonicalize().unwrap()
    } else {
        Path::new("docs/specifications/schemas/REQUEST_EVALUATION_RESULT-v1.schema.json")
            .canonicalize()
            .expect("Failed to locate REQUEST_EVALUATION_RESULT-v1 schema")
    };

    let schema_str = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read schema at {:?}", schema_path));
    let schema_json: Value =
        serde_json::from_str(&schema_str).expect("Failed to parse schema JSON");
    jsonschema::validator_for(&schema_json)
        .expect("Failed to compile REQUEST_EVALUATION_RESULT-v1 schema")
}

#[test]
fn test_exp_011_valid_fixtures() {
    let validator = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let valid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_011/valid");

    if valid_dir.exists() {
        for entry in fs::read_dir(&valid_dir).expect("Failed to read valid evaluation fixtures") {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path).unwrap();
                let instance: Value = serde_json::from_str(&content).unwrap();
                assert!(
                    validator.is_valid(&instance),
                    "Valid evaluation fixture rejected: {:?}",
                    path
                );
            }
        }
    }
}

#[test]
fn test_exp_011_invalid_fixtures() {
    let validator = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let invalid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_011/invalid");

    if invalid_dir.exists() {
        for entry in fs::read_dir(&invalid_dir).expect("Failed to read invalid evaluation fixtures")
        {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path).unwrap();
                let instance: Value = serde_json::from_str(&content).unwrap();
                assert!(
                    !validator.is_valid(&instance),
                    "Invalid/Adversarial evaluation fixture incorrectly accepted: {:?}",
                    path
                );
            }
        }
    }
}
