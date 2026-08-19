use beta001_harness::injected_inference_runtime::{
    evaluate_injected_inference, InjectedInferenceRuntimeResult,
};
use beta001_harness::raw_output_adapter::{
    NormalizationKind, RawOutputAdapter, RawOutputAdmission, RawOutputRejection,
};
use beta001_harness::runtime_profile::{
    ExternalNetworkPolicy, LocalInferenceTransport, NetworkPolicy, T5RuntimeProfile, ToolPolicy,
    T5_RUNTIME_PROFILE_VERSION_V1,
};
use beta001_harness::telemetry::ContainmentTelemetry;

const FIXTURE_001: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/001_clean_exact_match.txt",
);

const FIXTURE_002: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/002_markdown_fenced_json.txt",
);

fn minimal_runtime_profile() -> T5RuntimeProfile {
    T5RuntimeProfile::new(
        T5_RUNTIME_PROFILE_VERSION_V1,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        NetworkPolicy {
            external_network: ExternalNetworkPolicy::Denied,
            local_inference_transport: LocalInferenceTransport::HostLocalOnly,
        },
        ToolPolicy::new(Vec::new(), Vec::new(), Vec::new())
            .expect("empty tool policy must be valid"),
        Vec::new(),
    )
    .expect("minimal injected-inference runtime profile must be valid")
}

fn alternate_runtime_profile() -> T5RuntimeProfile {
    T5RuntimeProfile::new(
        T5_RUNTIME_PROFILE_VERSION_V1,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        NetworkPolicy {
            external_network: ExternalNetworkPolicy::Denied,
            local_inference_transport: LocalInferenceTransport::HostLocalOnly,
        },
        ToolPolicy::new(
            vec!["read_only_inspection".to_string()],
            vec!["unavailable_tool".to_string()],
            vec!["prohibited_tool".to_string()],
        )
        .expect("alternate tool policy must be valid"),
        Vec::new(),
    )
    .expect("alternate injected-inference runtime profile must be valid")
}

fn assert_runtime_matches_adapter(
    profile: &T5RuntimeProfile,
    raw: &[u8],
) -> InjectedInferenceRuntimeResult {
    let runtime_result = evaluate_injected_inference(profile, raw);
    let adapter_result = RawOutputAdapter::admit(raw);

    assert_eq!(
        runtime_result.adapter_result, adapter_result,
        "EXP-BETA-003 must delegate raw-output admission exactly to EXP-BETA-002"
    );

    assert_eq!(
        runtime_result.adapter_result.raw_output.as_slice(),
        raw,
        "runtime admission must preserve exact injected inference bytes"
    );

    assert_eq!(
        runtime_result.telemetry,
        ContainmentTelemetry::new(),
        "injected-byte admission alone must not fabricate containment telemetry"
    );

    runtime_result
}

#[test]
fn rejected_injected_inference_is_preserved_and_replays_deterministically() {
    let profile = minimal_runtime_profile();
    let raw = b"not-json";

    let first = evaluate_injected_inference(&profile, raw);
    let second = evaluate_injected_inference(&profile, raw);

    assert_eq!(
        first, second,
        "identical injected bytes and runtime inputs must replay deterministically"
    );

    assert_eq!(
        first.adapter_result.raw_output.as_slice(),
        raw,
        "the runtime boundary must preserve exact injected inference bytes"
    );

    assert_eq!(
        first.adapter_result.admission,
        RawOutputAdmission::Reject {
            reason: RawOutputRejection::InvalidJson,
        },
        "EXP-BETA-002 rejection must remain rejection at the runtime boundary"
    );

    assert_eq!(
        first.telemetry,
        ContainmentTelemetry::new(),
        "injected-byte admission alone must not fabricate filesystem, network, or containment events"
    );
}

#[test]
fn direct_valid_json_admission_is_delegated_without_semantic_drift() {
    let profile = minimal_runtime_profile();

    let runtime_result = assert_runtime_matches_adapter(&profile, FIXTURE_001);

    assert!(
        matches!(
            runtime_result.adapter_result.admission,
            RawOutputAdmission::Admit { .. }
        ),
        "clean valid AGENT_OUTPUT-v1 must remain direct Admit"
    );
}

#[test]
fn exact_outer_json_fence_normalization_is_delegated_once() {
    let profile = minimal_runtime_profile();

    let runtime_result = assert_runtime_matches_adapter(&profile, FIXTURE_002);

    match runtime_result.adapter_result.admission {
        RawOutputAdmission::AdmitNormalized { normalization, .. } => {
            assert_eq!(
                normalization,
                NormalizationKind::ExactOuterJsonMarkdownFenceRemoval,
                "runtime must preserve the exact EXP-BETA-002 normalization classification"
            );
        }
        other => {
            panic!("expected delegated AdmitNormalized result, observed {other:?}");
        }
    }
}

#[test]
fn invalid_utf8_is_preserved_byte_for_byte_and_rejected() {
    let profile = minimal_runtime_profile();
    let raw = [0xff, 0xfe, 0xfd, 0x00];

    let first = assert_runtime_matches_adapter(&profile, &raw);
    let second = evaluate_injected_inference(&profile, &raw);

    assert_eq!(
        first, second,
        "invalid UTF-8 injection must replay deterministically"
    );

    assert_eq!(
        first.adapter_result.raw_output, raw,
        "invalid UTF-8 evidence must remain byte-identical"
    );

    assert_eq!(
        first.adapter_result.admission,
        RawOutputAdmission::Reject {
            reason: RawOutputRejection::InvalidUtf8,
        },
        "invalid UTF-8 must preserve the frozen EXP-BETA-002 rejection taxonomy"
    );
}

#[test]
fn relevant_adapter_dispositions_replay_deterministically_through_runtime() {
    let profile = minimal_runtime_profile();

    let invalid_utf8 = [0xff, 0xfe, 0xfd];
    let invalid_json = b"not-json";
    let schema_violation = b"{}";
    let disallowed_wrapper = b"```text\n{}\n```\n";

    let mut trailing_content = FIXTURE_001.to_vec();
    trailing_content.extend_from_slice(b"\ntrailing");

    let cases: [(&str, &[u8]); 7] = [
        ("direct-admit", FIXTURE_001),
        ("normalized-admit", FIXTURE_002),
        ("invalid-utf8", &invalid_utf8),
        ("invalid-json", invalid_json),
        ("schema-violation", schema_violation),
        ("disallowed-wrapper", disallowed_wrapper),
        ("trailing-content", trailing_content.as_slice()),
    ];

    for (name, raw) in cases {
        let expected = RawOutputAdapter::admit(raw);
        let first = evaluate_injected_inference(&profile, raw);

        assert_eq!(
            first.adapter_result, expected,
            "runtime drifted from EXP-BETA-002 admission for {name}"
        );

        for replay in 0..32 {
            let observed = evaluate_injected_inference(&profile, raw);

            assert_eq!(
                observed, first,
                "runtime replay drift for {name} at replay {replay}"
            );
        }
    }
}

#[test]
fn runtime_profile_variation_does_not_silently_change_raw_output_admission_semantics() {
    let minimal = minimal_runtime_profile();
    let alternate = alternate_runtime_profile();

    let cases: [(&str, &[u8]); 3] = [
        ("direct-admit", FIXTURE_001),
        ("normalized-admit", FIXTURE_002),
        ("reject", b"not-json"),
    ];

    for (name, raw) in cases {
        let expected = RawOutputAdapter::admit(raw);

        let minimal_result = evaluate_injected_inference(&minimal, raw);
        let alternate_result = evaluate_injected_inference(&alternate, raw);

        assert_eq!(
            minimal_result.adapter_result, expected,
            "minimal profile changed frozen admission semantics for {name}"
        );

        assert_eq!(
            alternate_result.adapter_result, expected,
            "alternate valid profile changed frozen admission semantics for {name}"
        );

        assert_eq!(
            minimal_result.adapter_result, alternate_result.adapter_result,
            "profile variation silently changed raw-output admission for {name}"
        );

        assert_eq!(minimal_result.telemetry, ContainmentTelemetry::new());
        assert_eq!(alternate_result.telemetry, ContainmentTelemetry::new());
    }
}
