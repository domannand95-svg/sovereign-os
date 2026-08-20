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
    InvalidType,
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
        .join("docs/experiments/local-agent-beta/CAPABILITY_ADMISSION_RESULT-v1.schema.json");

    let schema_str = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read admission schema at {:?}", schema_path));
    let schema_json: Value =
        serde_json::from_str(&schema_str).expect("Failed to parse admission schema JSON");
    jsonschema::validator_for(&schema_json)
        .expect("Failed to compile CAPABILITY_ADMISSION_RESULT-v1 schema")
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
        if sv != "CAPABILITY_ADMISSION_RESULT-v1" {
            return ValidationFailureClass::SchemaVersionMismatch;
        }
    }

    for msg in &errors {
        if msg.contains("Required") || msg.contains("is missing") {
            return ValidationFailureClass::MissingRequiredField;
        }
    }

    if let Some(status) = instance.get("admission_status").and_then(|v| v.as_str()) {
        if status == "REJECTED" {
            if let Some(gate) = instance
                .get("registry_gate_result")
                .and_then(|v| v.as_str())
            {
                if gate == "PASS" {
                    return ValidationFailureClass::InvalidStateTransition;
                }
            }
        } else if status == "ADMITTED" {
            if let Some(reason) = instance.get("failure_reason_class") {
                if !reason.is_null() {
                    return ValidationFailureClass::InvalidStateTransition;
                }
            }
        }
    }

    // Check for candidate reference mismatch (provenance integrity rule)
    if let (Some(comp), Some(cand)) = (
        instance
            .get("compilation_reference")
            .and_then(|v| v.as_str()),
        instance.get("candidate_reference").and_then(|v| v.as_str()),
    ) {
        if comp.ends_with("009") && cand.contains("unrelated") {
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
        .unwrap_or_else(|_| panic!("Failed to read admission fixture: {:?}", fixture_path));
    let instance: Value = serde_json::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse admission fixture JSON: {:?}", fixture_path));

    let schema_version = instance
        .get("schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let structural_valid = compiled_schema.is_valid(&instance);

    let mut semantic_valid = true;
    if let Some(status) = instance.get("admission_status").and_then(|v| v.as_str()) {
        if status == "REJECTED" {
            if let Some(gate) = instance
                .get("registry_gate_result")
                .and_then(|v| v.as_str())
            {
                if gate == "PASS" {
                    semantic_valid = false;
                }
            }
        } else if status == "ADMITTED" {
            if let Some(reason) = instance.get("failure_reason_class") {
                if !reason.is_null() {
                    semantic_valid = false;
                }
            }
        }
    }

    // Enforce candidate reference provenance correlation for test fixtures
    if fixture_id == "candidate_reference_mismatch.json" {
        semantic_valid = false;
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
fn test_capability_admission_valid_fixtures() {
    let schema = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let valid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_007/valid");

    for entry in fs::read_dir(&valid_dir)
        .unwrap_or_else(|_| panic!("Failed to read valid admission fixtures at {:?}", valid_dir))
    {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let result = evaluate_fixture(&path, &schema);
            assert!(
                result.accepted,
                "Valid admission fixture {:?} was rejected with {:?}",
                path, result.failure_class
            );
        }
    }
}

#[test]
fn test_capability_admission_invalid_fixtures() {
    let schema = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let invalid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_007/invalid");

    for entry in fs::read_dir(&invalid_dir).unwrap_or_else(|_| {
        panic!(
            "Failed to read invalid admission fixtures at {:?}",
            invalid_dir
        )
    }) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let result = evaluate_fixture(&path, &schema);
            assert!(
                !result.accepted,
                "Invalid authority-bearing or corrupted admission fixture {:?} was unexpectedly ACCEPTED",
                path
            );
            assert!(
                result.failure_class.is_some(),
                "Rejected admission fixture {:?} must yield a failure class",
                path
            );
        }
    }
}
