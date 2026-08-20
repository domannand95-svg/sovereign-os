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
        .join("../../docs/experiments/local-agent-beta/EFFECT_EVALUATION_RESULT-v1.schema.json");
    let schema_str = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read evaluation schema at {:?}", schema_path));
    let schema_json: Value =
        serde_json::from_str(&schema_str).expect("Failed to parse evaluation schema JSON");
    jsonschema::validator_for(&schema_json)
        .expect("Failed to compile EFFECT_EVALUATION_RESULT-v1 schema")
}

fn classify_errors(instance: &Value, errors: Vec<String>) -> ValidationFailureClass {
    // 1. Check for SchemaVersionMismatch
    for msg in &errors {
        if msg.contains("schema_version")
            && (msg.contains("const") || msg.contains("enum") || msg.contains("value"))
        {
            return ValidationFailureClass::SchemaVersionMismatch;
        }
    }

    if let Some(sv) = instance.get("schema_version").and_then(|v| v.as_str()) {
        if sv != "EFFECT_EVALUATION_RESULT-v1" {
            return ValidationFailureClass::SchemaVersionMismatch;
        }
    }

    // 2. Check for MissingRequiredField
    for msg in &errors {
        if msg.contains("Required") || msg.contains("is missing") {
            return ValidationFailureClass::MissingRequiredField;
        }
    }

    // 3. Check for Semantic State Violation (InvalidStateTransition)
    if let Some(decision) = instance.get("decision").and_then(|v| v.as_str()) {
        if decision == "rejected" {
            if let Some(cand) = instance.get("capability_candidate_reference") {
                if !cand.is_null() {
                    return ValidationFailureClass::InvalidStateTransition;
                }
            }
        }
    }

    // 4. Check for AdditionalProperty
    for msg in &errors {
        if msg.contains("Additional properties") || msg.contains("additionalProperties") {
            return ValidationFailureClass::AdditionalProperty;
        }
    }

    // 5. Default fallback
    ValidationFailureClass::InvalidValue
}

pub fn evaluate_fixture(fixture_path: &Path, compiled_schema: &Validator) -> ValidationResult {
    let fixture_id = fixture_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let content = fs::read_to_string(fixture_path)
        .unwrap_or_else(|_| panic!("Failed to read evaluation fixture: {:?}", fixture_path));
    let instance: Value = serde_json::from_str(&content).unwrap_or_else(|_| {
        panic!(
            "Failed to parse evaluation fixture JSON: {:?}",
            fixture_path
        )
    });

    let schema_version = instance
        .get("schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let structural_valid = compiled_schema.is_valid(&instance);

    // Perform semantic state validation on top of schema validation
    let mut semantic_valid = true;
    if let Some(decision) = instance.get("decision").and_then(|v| v.as_str()) {
        if decision == "rejected" {
            if let Some(cand) = instance.get("capability_candidate_reference") {
                if !cand.is_null() {
                    semantic_valid = false;
                }
            }
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
fn test_effect_evaluation_valid_fixtures() {
    let schema = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let valid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_004/valid"); // reused harness structure path or local
    let eval_valid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_005/valid");

    let target_dir = if eval_valid_dir.exists() {
        eval_valid_dir
    } else {
        valid_dir
    };

    for entry in fs::read_dir(&target_dir).unwrap_or_else(|_| {
        panic!(
            "Failed to read valid evaluation fixtures at {:?}",
            target_dir
        )
    }) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let result = evaluate_fixture(&path, &schema);
            assert!(
                result.accepted,
                "Valid evaluation fixture {:?} was rejected with {:?}",
                path, result.failure_class
            );
        }
    }
}

#[test]
fn test_effect_evaluation_invalid_fixtures() {
    let schema = load_schema();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let eval_invalid_dir = Path::new(manifest_dir).join("tests/fixtures/exp_beta_005/invalid");

    for entry in fs::read_dir(&eval_invalid_dir).unwrap_or_else(|_| {
        panic!(
            "Failed to read invalid evaluation fixtures at {:?}",
            eval_invalid_dir
        )
    }) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let result = evaluate_fixture(&path, &schema);
            assert!(
                !result.accepted,
                "Invalid authority-bearing or corrupted evaluation fixture {:?} was unexpectedly ACCEPTED",
                path
            );
            assert!(
                result.failure_class.is_some(),
                "Rejected evaluation fixture {:?} must yield a failure class",
                path
            );
        }
    }
}
