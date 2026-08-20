use jsonschema::Validator;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationFailureClass {
    SchemaVersionMismatch,
    MissingRequiredField,
    InvalidStateTransition,
    AdditionalProperty,
    LineageViolation,
    InvalidValue,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub fixture_id: String,
    pub schema_version: String,
    pub accepted: bool,
    pub failure_class: Option<ValidationFailureClass>,
}

fn load_schema() -> Validator {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let schema_path = Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs")
        .join("experiments")
        .join("local-agent-beta")
        .join("CAPABILITY_LIFECYCLE_EVENT-v1.schema.json");

    let schema_str = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read lifecycle schema at {:?}", schema_path));
    let schema_json: Value =
        serde_json::from_str(&schema_str).expect("Failed to parse lifecycle schema JSON");
    jsonschema::validator_for(&schema_json)
        .expect("Failed to compile CAPABILITY_LIFECYCLE_EVENT-v1 schema")
}

fn classify_errors(instance: &Value, errors: Vec<String>) -> ValidationFailureClass {
    for msg in &errors {
        if msg.contains("schema_version")
            && (msg.contains("const") || msg.contains("enum") || msg.contains("value"))
        {
            return ValidationFailureClass::SchemaVersionMismatch;
        }
    }

    if let Some(sv) = instance.get("schema_version").and_then(|v| v.as_str()) {
        if sv != "CAPABILITY_LIFECYCLE_EVENT-v1" {
            return ValidationFailureClass::SchemaVersionMismatch;
        }
    }

    for msg in &errors {
        if msg.contains("Required") || msg.contains("is missing") {
            return ValidationFailureClass::MissingRequiredField;
        }
    }

    if instance.get("proposal_reference").is_none()
        || instance.get("evaluation_reference").is_none()
    {
        return ValidationFailureClass::LineageViolation;
    }

    if let (Some(ev), Some(prev), Some(new)) = (
        instance.get("event_type").and_then(|v| v.as_str()),
        instance.get("previous_state").and_then(|v| v.as_str()),
        instance.get("new_state").and_then(|v| v.as_str()),
    ) {
        if ev == "ACTIVATED" && prev == "ACTIVE" && new == "ACTIVE" {
            return ValidationFailureClass::InvalidStateTransition;
        }
    }

    for msg in &errors {
        if msg.contains("Additional properties") || msg.contains("additionalProperties") {
            return ValidationFailureClass::AdditionalProperty;
        }
    }

    ValidationFailureClass::InvalidValue
}

pub fn evaluate_fixture(fixture_path: &Path, compiled_schema: &Validator) -> ValidationResult {
    let fixture_id = fixture_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let content = fs::read_to_string(fixture_path)
        .unwrap_or_else(|_| panic!("Failed to read lifecycle fixture: {:?}", fixture_path));
    let instance: Value = serde_json::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse lifecycle fixture JSON: {:?}", fixture_path));

    let schema_version = instance
        .get("schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let structural_valid = compiled_schema.is_valid(&instance);

    let mut semantic_valid = true;
    if instance.get("proposal_reference").is_none()
        || instance.get("evaluation_reference").is_none()
    {
        semantic_valid = false;
    }

    if let (Some(ev), Some(prev), Some(new)) = (
        instance.get("event_type").and_then(|v| v.as_str()),
        instance.get("previous_state").and_then(|v| v.as_str()),
        instance.get("new_state").and_then(|v| v.as_str()),
    ) {
        if ev == "ACTIVATED" && prev == "ACTIVE" && new == "ACTIVE" {
            semantic_valid = false;
        }
    }

    if structural_valid && semantic_valid {
        ValidationResult {
            fixture_id,
            schema_version,
            accepted: true,
            failure_class: None,
        }
    } else {
        let err_vec: Vec<String> = compiled_schema
            .iter_errors(&instance)
            .map(|e| e.to_string())
            .collect();
        let failure_class = classify_errors(&instance, err_vec);
        ValidationResult {
            fixture_id,
            schema_version,
            accepted: false,
            failure_class: Some(failure_class),
        }
    }
}

#[test]
fn test_capability_lifecycle_valid_fixtures() {
    let schema = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let valid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_008/valid");

    for entry in fs::read_dir(&valid_dir)
        .unwrap_or_else(|_| panic!("Failed to read valid lifecycle fixtures at {:?}", valid_dir))
    {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let result = evaluate_fixture(&path, &schema);
            assert!(
                result.accepted,
                "Valid lifecycle fixture {:?} was rejected with {:?}",
                path, result.failure_class
            );
        }
    }
}

#[test]
fn test_capability_lifecycle_invalid_fixtures() {
    let schema = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let invalid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_008/invalid");

    for entry in fs::read_dir(&invalid_dir).unwrap_or_else(|_| {
        panic!(
            "Failed to read invalid lifecycle fixtures at {:?}",
            invalid_dir
        )
    }) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let result = evaluate_fixture(&path, &schema);
            assert!(
                !result.accepted,
                "Invalid authority-bearing or corrupted lifecycle fixture {:?} was unexpectedly ACCEPTED",
                path
            );
            assert!(
                result.failure_class.is_some(),
                "Rejected lifecycle fixture {:?} must yield a failure class",
                path
            );
        }
    }
}
