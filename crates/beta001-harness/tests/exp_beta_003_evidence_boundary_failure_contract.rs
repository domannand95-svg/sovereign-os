use beta001_harness::injected_inference_runtime::{
    evaluate_injected_inference_evidence_boundary, EvidenceBoundaryBlocker,
    EvidenceBoundaryDisposition,
};
use beta001_harness::raw_output_adapter::{
    NormalizationKind, RawOutputAdmission, RawOutputRejection,
};
use beta001_harness::runtime_profile::{
    ExternalNetworkPolicy, LocalInferenceTransport, NetworkPolicy, T5RuntimeProfile, ToolPolicy,
    T5_RUNTIME_PROFILE_VERSION_V1,
};

const DIRECT_VALID_JSON: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/001_clean_exact_match.txt",
);

const EXACT_FENCED_JSON: &[u8] = include_bytes!(
    "../../../docs/experiments/local-agent-beta/fixtures/raw-output-adapter/002_markdown_fenced_json.txt",
);

const INVALID_UTF8: &[u8] = &[0xff, 0xfe, 0xfd, 0x00];

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

#[test]
fn valid_direct_json_is_admitted_but_not_seal_eligible_without_observations() {
    let profile = minimal_runtime_profile();
    let result = evaluate_injected_inference_evidence_boundary(&profile, DIRECT_VALID_JSON);

    assert_eq!(
        result.runtime_result.adapter_result.raw_output.as_slice(),
        DIRECT_VALID_JSON
    );

    assert!(matches!(
        &result.runtime_result.adapter_result.admission,
        RawOutputAdmission::Admit { .. }
    ));

    assert_eq!(
        result.disposition,
        EvidenceBoundaryDisposition::NotSealEligible {
            blockers: vec![
                EvidenceBoundaryBlocker::MissingIntegrityObservation,
                EvidenceBoundaryBlocker::MissingContainmentObservation,
            ],
        }
    );
}

#[test]
fn exact_permitted_json_fence_is_normalized_but_not_seal_eligible_without_observations() {
    let profile = minimal_runtime_profile();
    let result = evaluate_injected_inference_evidence_boundary(&profile, EXACT_FENCED_JSON);

    assert_eq!(
        result.runtime_result.adapter_result.raw_output.as_slice(),
        EXACT_FENCED_JSON
    );

    assert!(matches!(
        &result.runtime_result.adapter_result.admission,
        RawOutputAdmission::AdmitNormalized {
            normalization: NormalizationKind::ExactOuterJsonMarkdownFenceRemoval,
            ..
        }
    ));

    assert_eq!(
        result.disposition,
        EvidenceBoundaryDisposition::NotSealEligible {
            blockers: vec![
                EvidenceBoundaryBlocker::MissingIntegrityObservation,
                EvidenceBoundaryBlocker::MissingContainmentObservation,
            ],
        }
    );
}

#[test]
fn valid_utf8_invalid_json_is_rejected_without_becoming_evidence_eligible() {
    let profile = minimal_runtime_profile();
    let raw = b"not-json";
    let result = evaluate_injected_inference_evidence_boundary(&profile, raw);

    assert_eq!(
        result.runtime_result.adapter_result.raw_output.as_slice(),
        raw
    );

    assert!(matches!(
        &result.runtime_result.adapter_result.admission,
        RawOutputAdmission::Reject {
            reason: RawOutputRejection::InvalidJson
        }
    ));

    assert_eq!(
        result.disposition,
        EvidenceBoundaryDisposition::NotSealEligible {
            blockers: vec![
                EvidenceBoundaryBlocker::MissingIntegrityObservation,
                EvidenceBoundaryBlocker::MissingContainmentObservation,
            ],
        }
    );
}

#[test]
fn schema_invalid_json_is_rejected_without_becoming_evidence_eligible() {
    let profile = minimal_runtime_profile();
    let raw = b"{}";
    let result = evaluate_injected_inference_evidence_boundary(&profile, raw);

    assert_eq!(
        result.runtime_result.adapter_result.raw_output.as_slice(),
        raw
    );

    assert!(matches!(
        &result.runtime_result.adapter_result.admission,
        RawOutputAdmission::Reject {
            reason: RawOutputRejection::SchemaViolation
        }
    ));

    assert_eq!(
        result.disposition,
        EvidenceBoundaryDisposition::NotSealEligible {
            blockers: vec![
                EvidenceBoundaryBlocker::MissingIntegrityObservation,
                EvidenceBoundaryBlocker::MissingContainmentObservation,
            ],
        }
    );
}

#[test]
fn invalid_utf8_preserves_bytes_and_freezes_full_ordered_blocker_vector() {
    let profile = minimal_runtime_profile();
    let result = evaluate_injected_inference_evidence_boundary(&profile, INVALID_UTF8);

    assert_eq!(
        result.runtime_result.adapter_result.raw_output.as_slice(),
        INVALID_UTF8
    );

    assert!(matches!(
        &result.runtime_result.adapter_result.admission,
        RawOutputAdmission::Reject {
            reason: RawOutputRejection::InvalidUtf8
        }
    ));

    assert_eq!(
        result.disposition,
        EvidenceBoundaryDisposition::NotSealEligible {
            blockers: vec![
                EvidenceBoundaryBlocker::RawBytesNotRepresentableAsCandidateString,
                EvidenceBoundaryBlocker::MissingIntegrityObservation,
                EvidenceBoundaryBlocker::MissingContainmentObservation,
            ],
        }
    );
}

#[test]
fn identical_inputs_replay_with_identical_adapter_and_ordered_evidence_dispositions() {
    let cases: [&[u8]; 5] = [
        DIRECT_VALID_JSON,
        EXACT_FENCED_JSON,
        b"not-json",
        b"{}",
        INVALID_UTF8,
    ];

    for raw in cases {
        let profile = minimal_runtime_profile();
        let first = evaluate_injected_inference_evidence_boundary(&profile, raw);

        for replay_index in 0..32 {
            let replay = evaluate_injected_inference_evidence_boundary(&profile, raw);

            assert_eq!(
                replay, first,
                "complete evidence-boundary result drifted at replay {replay_index}"
            );
        }
    }
}

// --- EXP-BETA-003 RED TESTS: EXECUTION BINDING & COVERAGE ---

use std::collections::BTreeMap as RedBTreeMap;

use beta001_harness::containment::ContainmentBreach as RedContainmentBreach;
use beta001_harness::injected_inference_runtime::{
    evaluate_bound_governed_observation_contract as red_evaluate_bound_contract,
    ContainmentCoverageStatus as RedContainmentCoverageStatus, ExecutionId as RedExecutionId,
    GovernedContainmentRecord as RedGovernedContainmentRecord,
    GovernedIntegrityRecord as RedGovernedIntegrityRecord,
    ObservationContractBlocker as RedObservationContractBlocker,
    ObservationContractDisposition as RedObservationContractDisposition,
};
use beta001_harness::integrity::{
    compute_surface_digest as red_surface_digest, evaluate_integrity as red_evaluate_integrity,
    IntegrityReport as RedIntegrityReport, StateSurfaceSnapshot as RedStateSurfaceSnapshot,
};
use beta001_harness::runtime_profile::{
    DeclaredPath as RedDeclaredPath, EvidenceTreatment as RedEvidenceTreatment,
    ExternalNetworkPolicy as RedExternalNetworkPolicy,
    LocalInferenceTransport as RedLocalInferenceTransport, NetworkPolicy as RedNetworkPolicy,
    PersistenceClass as RedPersistenceClass, StateSurface as RedStateSurface,
    StateSurfaceKind as RedStateSurfaceKind, T5RuntimeProfile as RedRuntimeProfile,
    ToolPolicy as RedToolPolicy, T5_RUNTIME_PROFILE_VERSION_V1 as RED_RUNTIME_PROFILE_VERSION,
};
use beta001_harness::telemetry::ContainmentTelemetry as RedContainmentTelemetry;

fn red_execution_id(value: &str) -> RedExecutionId {
    RedExecutionId::new(value).expect("RED fixture execution identity must be valid")
}

fn red_runtime_profile() -> RedRuntimeProfile {
    let repository = RedDeclaredPath::new("C:/sol").expect("repository path must be valid");
    let ephemeral = RedDeclaredPath::new("C:/tmp").expect("ephemeral path must be valid");

    let repository_surface = RedStateSurface::new(
        "repository",
        RedStateSurfaceKind::Repository,
        Some(repository.clone()),
        RedPersistenceClass::Persistent,
        RedEvidenceTreatment::PrePostIntegrity,
    )
    .expect("repository surface must be valid");

    RedRuntimeProfile::new(
        RED_RUNTIME_PROFILE_VERSION,
        vec![repository.clone()],
        vec![ephemeral],
        vec![repository],
        RedNetworkPolicy {
            external_network: RedExternalNetworkPolicy::Denied,
            local_inference_transport: RedLocalInferenceTransport::HostLocalOnly,
        },
        RedToolPolicy::new(Vec::new(), Vec::new(), Vec::new())
            .expect("empty tool policy must be valid"),
        vec![repository_surface],
    )
    .expect("RED runtime profile must be valid")
}

fn red_complete_intact_integrity_report() -> RedIntegrityReport {
    let snapshot = RedStateSurfaceSnapshot {
        path: "C:/sol".to_owned(),
        kind: RedStateSurfaceKind::Repository,
        content_hash: Some(red_surface_digest(b"stable-red-fixture")),
        exists: true,
    };

    let mut pre = RedBTreeMap::new();
    pre.insert("C:/sol".to_owned(), snapshot.clone());

    let mut post = RedBTreeMap::new();
    post.insert("C:/sol".to_owned(), snapshot);

    red_evaluate_integrity(pre, post)
}

#[test]
fn execution_binding_mismatch_is_distinct_from_missing_observation() {
    let profile = red_runtime_profile();
    let candidate_id = red_execution_id("EXEC-A");
    let containment_id = red_execution_id("EXEC-B");

    let integrity = RedGovernedIntegrityRecord::new(
        candidate_id.clone(),
        red_complete_intact_integrity_report(),
    );

    let containment = RedGovernedContainmentRecord::new(
        containment_id,
        RedContainmentCoverageStatus::Complete,
        RedContainmentTelemetry::new(),
    );

    let disposition = red_evaluate_bound_contract(
        &profile,
        &candidate_id,
        Some(&integrity),
        Some(&containment),
    );

    assert_eq!(
        disposition,
        RedObservationContractDisposition::NotSatisfied {
            blockers: vec![RedObservationContractBlocker::ExecutionBindingMismatch],
        }
    );
}

#[test]
fn incomplete_containment_coverage_blocks_even_with_zero_breaches() {
    let profile = red_runtime_profile();
    let execution_id = red_execution_id("EXEC-A");

    let integrity = RedGovernedIntegrityRecord::new(
        execution_id.clone(),
        red_complete_intact_integrity_report(),
    );

    let containment = RedGovernedContainmentRecord::new(
        execution_id.clone(),
        RedContainmentCoverageStatus::Incomplete,
        RedContainmentTelemetry::new(),
    );

    let disposition = red_evaluate_bound_contract(
        &profile,
        &execution_id,
        Some(&integrity),
        Some(&containment),
    );

    assert_eq!(
        disposition,
        RedObservationContractDisposition::NotSatisfied {
            blockers: vec![RedObservationContractBlocker::ContainmentCoverageIncomplete],
        }
    );
}

#[test]
fn breach_and_incomplete_coverage_are_preserved_orthogonally() {
    let profile = red_runtime_profile();
    let execution_id = red_execution_id("EXEC-A");

    let integrity = RedGovernedIntegrityRecord::new(
        execution_id.clone(),
        red_complete_intact_integrity_report(),
    );

    let mut telemetry = RedContainmentTelemetry::new();
    telemetry.record_breach(&RedContainmentBreach::ExternalNetworkAttempt {
        endpoint: "example.invalid:443".to_owned(),
    });

    let containment = RedGovernedContainmentRecord::new(
        execution_id.clone(),
        RedContainmentCoverageStatus::Incomplete,
        telemetry,
    );

    let disposition = red_evaluate_bound_contract(
        &profile,
        &execution_id,
        Some(&integrity),
        Some(&containment),
    );

    assert_eq!(
        disposition,
        RedObservationContractDisposition::NotSatisfied {
            blockers: vec![
                RedObservationContractBlocker::ContainmentCoverageIncomplete,
                RedObservationContractBlocker::ContainmentBreached,
            ],
        }
    );
}
