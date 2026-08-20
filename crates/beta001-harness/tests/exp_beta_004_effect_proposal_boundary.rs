use jsonschema::Validator;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationFailureClass {
    MissingRequiredField,
    InvalidType,
    InvalidValue,
    AdditionalProperty,
    SchemaVersionMismatch,
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
        .join("../../docs/experiments/local-agent-beta/EFFECT_PROPOSAL-v1.schema.json");
    let schema_str = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read schema at {:?}", schema_path));
    let schema_json: Value =
        serde_json::from_str(&schema_str).expect("Failed to parse schema JSON");
    jsonschema::validator_for(&schema_json).expect("Failed to compile EFFECT_PROPOSAL-v1 schema")
}

fn classify_errors(errors: Vec<String>) -> ValidationFailureClass {
    for msg in &errors {
        if msg.contains("schema_version")
            && (msg.contains("const") || msg.contains("enum") || msg.contains("value"))
        {
            return ValidationFailureClass::SchemaVersionMismatch;
        }
    }

    let mut highest_class = ValidationFailureClass::AdditionalProperty;

    for err_str in errors {
        let class = match err_str {
            _ if err_str.contains("Required") || err_str.contains("is missing") => {
                ValidationFailureClass::MissingRequiredField
            }
            _ if err_str.contains("Type") => ValidationFailureClass::InvalidType,
            _ if err_str.contains("Enum") || err_str.contains("Const") => {
                ValidationFailureClass::InvalidValue
            }
            _ => ValidationFailureClass::AdditionalProperty,
        };

        match class {
            ValidationFailureClass::SchemaVersionMismatch => return class,
            ValidationFailureClass::MissingRequiredField => {
                highest_class = ValidationFailureClass::MissingRequiredField
            }
            ValidationFailureClass::InvalidType
                if highest_class != ValidationFailureClass::MissingRequiredField =>
            {
                highest_class = ValidationFailureClass::InvalidType
            }
            ValidationFailureClass::InvalidValue
                if !matches!(
                    highest_class,
                    ValidationFailureClass::MissingRequiredField
                        | ValidationFailureClass::InvalidType
                ) =>
            {
                highest_class = ValidationFailureClass::InvalidValue
            }
            _ => {}
        }
    }

    highest_class
}

pub fn evaluate_fixture(fixture_path: &Path, compiled_schema: &Validator) -> ValidationResult {
    let fixture_id = fixture_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let content = fs::read_to_string(fixture_path)
        .unwrap_or_else(|_| panic!("Failed to read fixture: {:?}", fixture_path));
    let instance: Value = serde_json::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse fixture JSON: {:?}", fixture_path));

    let schema_version = instance
        .get("schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    if compiled_schema.is_valid(&instance) {
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
        let failure_class = classify_errors(err_vec);
        ValidationResult {
            fixture_id,
            schema_version,
            accepted: false,
            failure_class: Some(failure_class),
        }
    }
}

#[test]
fn test_effect_proposal_valid_fixtures() {
    let schema = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let valid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_004/valid");

    for entry in fs::read_dir(&valid_dir)
        .unwrap_or_else(|_| panic!("Failed to read valid fixtures directory at {:?}", valid_dir))
    {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let result = evaluate_fixture(&path, &schema);
            assert!(
                result.accepted,
                "Valid fixture {:?} was rejected with {:?}",
                path, result.failure_class
            );
        }
    }
}

#[test]
fn test_effect_proposal_invalid_fixtures() {
    let schema = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let invalid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_004/invalid");

    for entry in fs::read_dir(&invalid_dir).unwrap_or_else(|_| {
        panic!(
            "Failed to read invalid fixtures directory at {:?}",
            invalid_dir
        )
    }) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let result = evaluate_fixture(&path, &schema);
            assert!(
                !result.accepted,
                "Invalid authority-bearing or mismatch fixture {:?} was unexpectedly ACCEPTED",
                path
            );
            assert!(
                result.failure_class.is_some(),
                "Rejected fixture {:?} must yield a failure class",
                path
            );
        }
    }
}
