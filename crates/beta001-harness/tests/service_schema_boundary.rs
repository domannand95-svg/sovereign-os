//! ADAM-011-A: Service Schema Boundary Validation Suite
//!
//! Falsifies criteria A011-001 through A011-012 against canonical Draft 2020-12 JSON schemas
//! using jsonschema 0.49 with explicit date-time format validation.

use jsonschema::Validator;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn load_schema(schema_filename: &str) -> (Value, Validator) {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("specifications")
        .join("schemas")
        .join(schema_filename);
    let raw = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", schema_path.display(), e));
    let schema: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("Failed to parse {} as JSON: {}", schema_filename, e));

    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap_or_else(|e| {
            panic!(
                "Failed to compile Draft 2020-12 schema {}: {}",
                schema_filename, e
            )
        });

    (schema, validator)
}

fn valid_proposal_request() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "user_id": "usr_alpha_123",
        "session_id": "ses_beta_456",
        "intent": "Request diagnostic report generation",
        "proposed_operation": "RequestReview",
        "source_evidence_references": [
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        ],
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

fn valid_proposal_response() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "proposal_id": "prop_alpha_789",
        "session_id": "ses_beta_456",
        "lifecycle_state": "AWAITING_APPROVAL",
        "risk_context": {
            "context_id": "risk_ctx_001",
            "risk_level": "LOW",
            "blast_radius": "ISOLATED"
        },
        "required_approval_level": "PEER",
        "authority_delta": 0,
        "provenance_digest": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

fn valid_execution_request() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "user_id": "usr_alpha_123",
        "session_id": "ses_beta_456",
        "proposal_id": "prop_alpha_789",
        "approval_receipt_id": "apr_gamma_321",
        "caller_identity": "governance_agent_01",
        "signature": "sig_ed25519_valid",
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

fn valid_execution_response() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "execution_id": "exe_omega_999",
        "proposal_id": "prop_alpha_789",
        "approval_receipt_id": "apr_gamma_321",
        "claim_state": "EXECUTION_CLAIMED",
        "dispatch_state": "DISPATCHED",
        "outcome_state": "SUCCEEDED",
        "authority_delta": 0,
        "evidence_package_digest": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

fn valid_inference_gateway() -> Value {
    json!({
        "schema_version": "v1.0.0",
        "session_id": "ses_beta_456",
        "provider_identity": "gemini-flash",
        "adapter_identity": "adapter_v1",
        "request_digest": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "response_digest": "ca978112ca1bbdcaf064278e4a1f94f0ec19f70604b0e9cda0704f026f20c37c",
        "transport_status": "SUCCESS",
        "raw_model_output": "{\"intent\":\"review\"}",
        "evidence_seal_digest": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "timestamp": "2026-08-24T00:00:00Z"
    })
}

// A011-001: All 5 schemas conform to Draft 2020-12 meta-schema
#[test]
fn test_a011_001_meta_schema_conformance() {
    let schemas = [
        "PROPOSAL_REQUEST-v1.schema.json",
        "PROPOSAL_RESPONSE-v1.schema.json",
        "EXECUTION_REQUEST-v1.schema.json",
        "EXECUTION_RESPONSE-v1.schema.json",
        "INFERENCE_GATEWAY-v1.schema.json",
    ];
    for name in schemas {
        let (schema, _) = load_schema(name);
        assert_eq!(
            schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
            "{} must declare Draft 2020-12 URI",
            name
        );
        jsonschema::draft202012::meta::validate(&schema).unwrap_or_else(|e| {
            panic!(
                "{} failed Draft 2020-12 meta-schema validation: {}",
                name, e
            )
        });
    }
}

// A011-002: additionalProperties: false enforced across all schemas
#[test]
fn test_a011_002_additional_properties_forbidden() {
    let (_, validator_prop) = load_schema("PROPOSAL_REQUEST-v1.schema.json");
    let mut payload = valid_proposal_request();
    payload["unauthorized_capability"] = json!("root");
    assert!(!validator_prop.is_valid(&payload));

    let (_, validator_exe) = load_schema("EXECUTION_REQUEST-v1.schema.json");
    let mut exe_payload = valid_execution_request();
    exe_payload["smuggled_token"] = json!("admin");
    assert!(!validator_exe.is_valid(&exe_payload));
}

// A011-003: Lexical prefixes enforced on identifier namespaces
#[test]
fn test_a011_003_namespace_prefix_enforcement() {
    let (_, validator) = load_schema("PROPOSAL_REQUEST-v1.schema.json");

    let mut invalid_user = valid_proposal_request();
    invalid_user["user_id"] = json!("ses_invalid_prefix");
    assert!(!validator.is_valid(&invalid_user));

    let mut invalid_session = valid_proposal_request();
    invalid_session["session_id"] = json!("usr_invalid_prefix");
    assert!(!validator.is_valid(&invalid_session));
}

// A011-004: Proposal request contains no capability/execution fields; rejects invalid timestamps
#[test]
fn test_a011_004_proposal_request_inertness_and_datetime_validation() {
    let (_, validator) = load_schema("PROPOSAL_REQUEST-v1.schema.json");
    let payload = valid_proposal_request();
    assert!(validator.is_valid(&payload));

    let mut escalated = valid_proposal_request();
    escalated["execute"] = json!(true);
    assert!(!validator.is_valid(&escalated));

    // Explicit falsification of date-time format assertion
    let mut invalid_date = valid_proposal_request();
    invalid_date["timestamp"] = json!("24/08/2026");
    assert!(!validator.is_valid(&invalid_date));
}

// A011-005: Execution request requires distinct proposal_id and approval_receipt_id
#[test]
fn test_a011_005_execution_request_identifier_requirements() {
    let (_, validator) = load_schema("EXECUTION_REQUEST-v1.schema.json");

    let mut missing_apr = valid_execution_request();
    missing_apr["approval_receipt_id"] = json!("prop_substituted_id");
    assert!(!validator.is_valid(&missing_apr));

    let mut missing_prop = valid_execution_request();
    missing_prop["proposal_id"] = json!("apr_substituted_id");
    assert!(!validator.is_valid(&missing_prop));
}

// A011-006: State triad represented independently
#[test]
fn test_a011_006_state_triad_representation() {
    let (_, validator) = load_schema("EXECUTION_RESPONSE-v1.schema.json");
    let payload = valid_execution_response();
    assert!(validator.is_valid(&payload));

    assert_eq!(payload["claim_state"], "EXECUTION_CLAIMED");
    assert_eq!(payload["dispatch_state"], "DISPATCHED");
    assert_eq!(payload["outcome_state"], "SUCCEEDED");
}

// A011-007: NOT_DISPATCHED enforces PENDING; non-PENDING requires DISPATCHED; DISPATCHED + OUTCOME_UNKNOWN is valid
#[test]
fn test_a011_007_dispatch_outcome_state_constraints() {
    let (_, validator) = load_schema("EXECUTION_RESPONSE-v1.schema.json");

    // Valid: NOT_DISPATCHED + PENDING
    let mut valid_not_dispatched = valid_execution_response();
    valid_not_dispatched["dispatch_state"] = json!("NOT_DISPATCHED");
    valid_not_dispatched["outcome_state"] = json!("PENDING");
    assert!(validator.is_valid(&valid_not_dispatched));

    // Invalid: NOT_DISPATCHED + SUCCEEDED
    let mut invalid_success = valid_execution_response();
    invalid_success["dispatch_state"] = json!("NOT_DISPATCHED");
    invalid_success["outcome_state"] = json!("SUCCEEDED");
    assert!(!validator.is_valid(&invalid_success));

    // Invalid: NOT_DISPATCHED + OUTCOME_UNKNOWN
    let mut invalid_unknown = valid_execution_response();
    invalid_unknown["dispatch_state"] = json!("NOT_DISPATCHED");
    invalid_unknown["outcome_state"] = json!("OUTCOME_UNKNOWN");
    assert!(!validator.is_valid(&invalid_unknown));

    // Valid: DISPATCHED + OUTCOME_UNKNOWN
    let mut valid_unknown = valid_execution_response();
    valid_unknown["dispatch_state"] = json!("DISPATCHED");
    valid_unknown["outcome_state"] = json!("OUTCOME_UNKNOWN");
    assert!(validator.is_valid(&valid_unknown));
}

// A011-008: Inference transport payload coupling (SUCCESS vs FAILED/TIMEOUT/DISCONNECTED)
#[test]
fn test_a011_008_inference_transport_payload_coupling() {
    let (_, validator) = load_schema("INFERENCE_GATEWAY-v1.schema.json");

    // Valid SUCCESS with digests
    let valid_success = valid_inference_gateway();
    assert!(validator.is_valid(&valid_success));

    // Invalid SUCCESS with null response
    let mut invalid_success = valid_inference_gateway();
    invalid_success["response_digest"] = json!(null);
    invalid_success["raw_model_output"] = json!(null);
    assert!(!validator.is_valid(&invalid_success));

    // Valid TIMEOUT with explicit nulls
    let mut valid_timeout = valid_inference_gateway();
    valid_timeout["transport_status"] = json!("TIMEOUT");
    valid_timeout["response_digest"] = json!(null);
    valid_timeout["raw_model_output"] = json!(null);
    assert!(validator.is_valid(&valid_timeout));

    // Invalid TIMEOUT with smuggled model output
    let mut invalid_timeout = valid_inference_gateway();
    invalid_timeout["transport_status"] = json!("TIMEOUT");
    invalid_timeout["response_digest"] =
        json!("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    invalid_timeout["raw_model_output"] = json!("output");
    assert!(!validator.is_valid(&invalid_timeout));
}

// A011-009: SHA-256 canonical hex enforcement
#[test]
fn test_a011_009_sha256_canonical_hex_enforcement() {
    let (_, validator) = load_schema("PROPOSAL_REQUEST-v1.schema.json");

    // Invalid: uppercase hex
    let mut upper_hex = valid_proposal_request();
    upper_hex["source_evidence_references"] =
        json!(["E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855"]);
    assert!(!validator.is_valid(&upper_hex));

    // Invalid: short length
    let mut short_hex = valid_proposal_request();
    short_hex["source_evidence_references"] = json!(["e3b0c442"]);
    assert!(!validator.is_valid(&short_hex));
}

// A011-010: Provider envelope contains zero authority fields
#[test]
fn test_a011_010_provider_envelope_zero_authority() {
    let (schema_val, validator) = load_schema("INFERENCE_GATEWAY-v1.schema.json");

    // Schema definition must not include authority fields
    let properties = schema_val["properties"].as_object().unwrap();
    assert!(!properties.contains_key("authorized"));
    assert!(!properties.contains_key("approved"));
    assert!(!properties.contains_key("capability"));
    assert!(!properties.contains_key("authority_delta"));

    // Injected payload must fail validation
    let mut injected = valid_inference_gateway();
    injected["authorized"] = json!(true);
    assert!(!validator.is_valid(&injected));
}

// A011-011: Cross-plane identifier substitution fails
#[test]
fn test_a011_011_cross_plane_identifier_substitution_fails() {
    let (_, validator) = load_schema("EXECUTION_REQUEST-v1.schema.json");

    let mut swapped = valid_execution_request();
    swapped["proposal_id"] = json!("exe_swapped_id");
    assert!(!validator.is_valid(&swapped));
}

// A011-012: Governed envelopes enforce authority_delta = 0
#[test]
fn test_a011_012_authority_delta_zero_enforcement() {
    let (_, validator_prop) = load_schema("PROPOSAL_RESPONSE-v1.schema.json");
    let mut escalated_prop = valid_proposal_response();
    escalated_prop["authority_delta"] = json!(1);
    assert!(!validator_prop.is_valid(&escalated_prop));

    let (_, validator_exe) = load_schema("EXECUTION_RESPONSE-v1.schema.json");
    let mut escalated_exe = valid_execution_response();
    escalated_exe["authority_delta"] = json!(1);
    assert!(!validator_exe.is_valid(&escalated_exe));
}
