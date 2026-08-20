use std::fs;
use std::path::Path;
use jsonschema::Validator;
use serde_json::Value;

fn load_schema() -> Validator {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let schema_path = Path::new(manifest_dir)
        .join("../../../docs/specifications/schemas/CAPABILITY_REQUEST-v1.schema.json");

    let schema_path = schema_path.canonicalize()
        .unwrap_or_else(|_| Path::new("docs/specifications/schemas/CAPABILITY_REQUEST-v1.schema.json")
            .canonicalize()
            .expect("Failed to locate CAPABILITY_REQUEST-v1 schema"));

    let schema_str = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read schema at {:?}", schema_path));
    let schema_json: Value = serde_json::from_str(&schema_str)
        .expect("Failed to parse schema JSON");
    jsonschema::validator_for(&schema_json)
        .expect("Failed to compile CAPABILITY_REQUEST-v1 schema")
}

#[test]
fn test_exp_010_valid_request_passes() {
    let validator = load_schema();
    let instance = serde_json::json!({
        "schema_version": "CAPABILITY_REQUEST-v1",
        "request_id": "req_001",
        "requester_identity": "agent-instance-001",
        "requested_capability": {
            "capability": "filesystem.write"
        },
        "declared_purpose": "Write build output into workspace",
        "required_scope": {
            "resource": "/workspace/project",
            "operations": ["write"]
        },
        "duration_requirement": "10m",
        "risk_classification": "LOW",
        "supporting_evidence_references": ["EVID-001"]
    });

    assert!(validator.is_valid(&instance), "Valid capability request failed validation");
}

#[test]
fn test_exp_010_missing_field_rejected() {
    let validator = load_schema();
    let instance = serde_json::json!({
        "schema_version": "CAPABILITY_REQUEST-v1",
        "request_id": "req_002",
        "requester_identity": "agent-instance-001"
    });

    assert!(!validator.is_valid(&instance), "Malformed request should fail validation");
}

#[test]
fn test_exp_010_embedded_credential_rejected() {
    let validator = load_schema();
    let instance = serde_json::json!({
        "schema_version": "CAPABILITY_REQUEST-v1",
        "request_id": "req_003",
        "requester_identity": "agent-instance-001",
        "requested_capability": { "capability": "filesystem.write" },
        "declared_purpose": "Unauthorized escalation",
        "required_scope": { "resource": "/etc", "operations": ["write"] },
        "duration_requirement": "1h",
        "risk_classification": "CRITICAL",
        "supporting_evidence_references": ["EVID-001"],
        "execution_token": "token_unauthorized_999"
    });

    assert!(!validator.is_valid(&instance), "Request containing execution token must be rejected");
}
