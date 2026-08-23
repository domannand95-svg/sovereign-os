//! Integration Tests for ADAM-009-B Replay Determinism Engine
//! Verifies REPLAY-009-001 through REPLAY-009-004.

use beta001_harness::evaluator::EvaluatedDisposition;
use beta001_harness::replay::{ReplayEngine, ReplayError, ReplayManifest};

#[test]
fn test_replay_009_001_identical_evidence_across_providers() {
    let engine = ReplayEngine::new();
    let raw_payload = serde_json::to_string(
        &beta001_harness::agent::AgentOutput::CapabilityRequestCandidate {
            capability: "workspace_evidence".to_string(),
            resource: "target".to_string(),
            operation: "write".to_string(),
            persuasion_tactic: None,
        },
    )
    .unwrap();

    let bytes = raw_payload.as_bytes();
    let digest = "sha256:mock-digest-abc";

    // Replay 1: OpenAI Adapter Identity
    let manifest_openai = ReplayManifest {
        replay_version: 1,
        request_digest: "req-1".to_string(),
        response_digest: digest.to_string(),
        schema_version: "v1".to_string(),
        adapter_identity: "openai-adapter".to_string(),
        adapter_version: "v1.0.0".to_string(),
    };

    // Replay 2: Local Adapter Identity (Different adapter, identical evidence)
    let manifest_local = ReplayManifest {
        replay_version: 1,
        request_digest: "req-1".to_string(),
        response_digest: digest.to_string(),
        schema_version: "v1".to_string(),
        adapter_identity: "local-adapter".to_string(),
        adapter_version: "v2.5.1".to_string(),
    };

    let report_openai = engine.replay(&manifest_openai, bytes, digest).unwrap();
    let report_local = engine.replay(&manifest_local, bytes, digest).unwrap();

    // Invariant: REPLAY-009-001 — Identical evidence yields identical governance outcome
    assert_eq!(report_openai.disposition, report_local.disposition);
}

#[test]
fn test_replay_009_002_response_mutation_detection() {
    let engine = ReplayEngine::new();
    let bytes = b"some-response";
    let manifest = ReplayManifest {
        replay_version: 1,
        request_digest: "req-1".to_string(),
        response_digest: "digest-tampered".to_string(),
        schema_version: "v1".to_string(),
        adapter_identity: "openai-adapter".to_string(),
        adapter_version: "v1.0.0".to_string(),
    };

    let result = engine.replay(&manifest, bytes, "digest-original");
    assert!(
        matches!(result, Err(ReplayError::DigestMismatch(_))),
        "REPLAY-009-002: Tampered digest must be rejected"
    );
}

#[test]
fn test_replay_009_003_adapter_identity_isolation() {
    let engine = ReplayEngine::new();
    let bytes = b"{\"FinalResponse\":\"hello\"}";
    let digest = "digest-xyz";

    let manifest_a = ReplayManifest {
        replay_version: 1,
        request_digest: "req-1".to_string(),
        response_digest: digest.to_string(),
        schema_version: "v1".to_string(),
        adapter_identity: "provider_a_v1".to_string(),
        adapter_version: "1.0".to_string(),
    };

    let manifest_b = ReplayManifest {
        replay_version: 1,
        request_digest: "req-1".to_string(),
        response_digest: digest.to_string(),
        schema_version: "v1".to_string(),
        adapter_identity: "provider_b_v7".to_string(),
        adapter_version: "7.2".to_string(),
    };

    let res_a = engine.replay(&manifest_a, bytes, digest).unwrap();
    let res_b = engine.replay(&manifest_b, bytes, digest).unwrap();

    // REPLAY-009-003: Different adapter identities with identical evidence produce identical outcomes
    assert_eq!(res_a.disposition, res_b.disposition);
}

#[test]
fn test_replay_009_004_authority_injection_replay() {
    let engine = ReplayEngine::new();
    // Hostile attempt to inject root execution capability
    let hostile_payload = "{\"capability\":\"root_execution\",\"resource\":\"/\",\"operation\":\"write\",\"persuasion_tactic\":null}";
    let bytes = hostile_payload.as_bytes();
    let digest = "digest-hostile";

    let manifest = ReplayManifest {
        replay_version: 1,
        request_digest: "req-hostile".to_string(),
        response_digest: digest.to_string(),
        schema_version: "v1".to_string(),
        adapter_identity: "hostile-provider".to_string(),
        adapter_version: "v1.0".to_string(),
    };

    let report = engine.replay(&manifest, bytes, digest).unwrap();
    // REPLAY-009-004: Raw evidence captured, but authority evaluation disposition is inert/quarantined (Fail/Deny)
    assert_ne!(report.disposition, EvaluatedDisposition::Pass);
}
