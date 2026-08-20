use jsonschema::Validator;
use serde_json::Value;
use std::fs;
use std::path::Path;

fn load_schema(schema_filename: &str) -> Validator {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let schema_path = Path::new(manifest_dir).join(format!(
        r#"../../docs/specifications/schemas/{}"#,
        schema_filename
    ));

    let schema_path = if schema_path.exists() {
        schema_path.canonicalize().unwrap()
    } else {
        Path::new(&format!("docs/specifications/schemas/{}", schema_filename))
            .canonicalize()
            .unwrap_or_else(|_| panic!("Failed to locate schema: {}", schema_filename))
    };

    let schema_str = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read schema at {:?}", schema_path));
    let schema_json: Value =
        serde_json::from_str(&schema_str).expect("Failed to parse schema JSON");
    jsonschema::validator_for(&schema_json).expect("Failed to compile schema")
}

#[test]
fn test_exp_014_valid_execution_request() {
    let validator = load_schema("CAPABILITY_EXECUTION_REQUEST-v1.schema.json");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let valid_path =
        Path::new(manifest_dir).join("tests/fixtures/exp_beta_014/valid/valid_execution_001.json");

    let content = fs::read_to_string(&valid_path).expect("Failed to read valid execution fixture");
    let instance: Value = serde_json::from_str(&content).expect("Failed to parse JSON");
    assert!(
        validator.is_valid(&instance),
        "Valid execution request rejected"
    );
}

#[test]
fn test_exp_014_invalid_execution_request() {
    let validator = load_schema("CAPABILITY_EXECUTION_REQUEST-v1.schema.json");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let invalid_path = Path::new(manifest_dir)
        .join("tests/fixtures/exp_beta_014/invalid/invalid_scope_and_credential.json");

    let content =
        fs::read_to_string(&invalid_path).expect("Failed to read invalid execution fixture");
    let instance: Value = serde_json::from_str(&content).expect("Failed to parse JSON");
    assert!(
        !validator.is_valid(&instance),
        "Adversarial execution request incorrectly accepted"
    );
}
