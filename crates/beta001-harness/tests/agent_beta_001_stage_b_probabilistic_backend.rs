use beta001_harness::agent::{AgentBackend, AgentBackendError, AgentInput, RealModelBackend};

#[test]
fn test_stage_b_real_model_backend_credential_isolation() {
    // Enforce INVARIANT-087: Host must require secret via environment without leaking it
    std::env::set_var("TEST_MODEL_API_KEY", "sk-host-secret-test-token");
    let backend = RealModelBackend::new(
        "https://api. sovereign-model.internal/v1",
        "TEST_MODEL_API_KEY",
    );

    let input = AgentInput {
        prompt: "Perform permitted build task".to_string(),
        task_reference: "task_001".to_string(),
    };

    let result = backend.respond(&input);
    assert!(
        result.is_ok(),
        "RealModelBackend failed valid response path"
    );

    // Ensure the token does not appear in debug representation or output types
    let debug_str = format!("{:?}", result.unwrap());
    assert!(
        !debug_str.contains("sk-host-secret-test-token"),
        "Credential leaked into backend output representation!"
    );
}

#[test]
fn test_stage_b_malformed_model_output_fails_safely() {
    // Enforce INVARIANT-089: Malformed responses yield BackendError, never expanded authority
    std::env::set_var("TEST_MODEL_API_KEY", "sk-host-secret-test-token");
    let backend = RealModelBackend::new(
        "https://api. sovereign-model.internal/v1",
        "TEST_MODEL_API_KEY",
    );

    let input = AgentInput {
        prompt: "malformed output request".to_string(),
        task_reference: "task_002".to_string(),
    };

    let result = backend.respond(&input);
    assert!(
        matches!(result, Err(AgentBackendError::MalformedResponse(_))),
        "Malformed output did not fail closed into BackendError"
    );
}

#[test]
fn test_stage_b_missing_credential_fails_closed() {
    // Enforce INVARIANT-087 & 089: Missing host environment credential fails closed immediately
    std::env::remove_var("TEST_MISSING_KEY");
    let backend = RealModelBackend::new(
        "https://api. sovereign-model.internal/v1",
        "TEST_MISSING_KEY",
    );

    let input = AgentInput {
        prompt: "Any prompt".to_string(),
        task_reference: "task_003".to_string(),
    };

    let result = backend.respond(&input);
    assert!(
        matches!(result, Err(AgentBackendError::ProviderUnavailable(_))),
        "Missing credential must fail closed with ProviderUnavailable"
    );
}
