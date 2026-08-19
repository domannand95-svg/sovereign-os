use std::collections::BTreeMap;

use beta001_harness::injected_inference_runtime::{
    evaluate_governed_observation_contract, ObservationContractBlocker,
    ObservationContractDisposition,
};
use beta001_harness::integrity::{IntegrityReport, INTEGRITY_PLANE_VERSION};
use beta001_harness::telemetry::ContainmentTelemetry;

#[test]
fn legacy_clean_defaults_cannot_satisfy_governed_observation_contract() {
    let telemetry = ContainmentTelemetry::new();

    let integrity = IntegrityReport {
        version: INTEGRITY_PLANE_VERSION,
        pre_snapshots: BTreeMap::new(),
        post_snapshots: BTreeMap::new(),
        mutated_surfaces: Vec::new(),
        is_intact: true,
    };

    let disposition = evaluate_governed_observation_contract(&telemetry, &integrity);

    assert_eq!(
        disposition,
        ObservationContractDisposition::NotSatisfied {
            blockers: vec![
                ObservationContractBlocker::MissingExecutionBinding,
                ObservationContractBlocker::MissingIntegrityObservation,
                ObservationContractBlocker::MissingContainmentObservation,
            ],
        }
    );
}

// --- EXP-BETA-003 RED TEST: POSITIVE OBSERVATION SATISFACTION ---

mod positive_observation_red {
    use std::collections::BTreeMap;

    use beta001_harness::injected_inference_runtime::{
        evaluate_bound_governed_observation_contract, ContainmentCoverageStatus, ExecutionId,
        GovernedContainmentRecord, GovernedIntegrityRecord, ObservationContractDisposition,
    };
    use beta001_harness::integrity::{
        compute_surface_digest, evaluate_integrity, IntegrityReport, StateSurfaceSnapshot,
    };
    use beta001_harness::runtime_profile::{
        DeclaredPath, EvidenceTreatment, ExternalNetworkPolicy, LocalInferenceTransport,
        NetworkPolicy, PersistenceClass, StateSurface, StateSurfaceKind, T5RuntimeProfile,
        ToolPolicy, T5_RUNTIME_PROFILE_VERSION_V1,
    };
    use beta001_harness::telemetry::ContainmentTelemetry;

    fn execution_id() -> ExecutionId {
        ExecutionId::new("EXEC-POSITIVE-001")
            .expect("positive RED fixture execution identity must be valid")
    }

    fn runtime_profile() -> T5RuntimeProfile {
        let repository =
            DeclaredPath::new("C:/sol").expect("positive RED repository path must be valid");

        let ephemeral =
            DeclaredPath::new("C:/tmp").expect("positive RED ephemeral path must be valid");

        let repository_surface = StateSurface::new(
            "repository",
            StateSurfaceKind::Repository,
            Some(repository.clone()),
            PersistenceClass::Persistent,
            EvidenceTreatment::PrePostIntegrity,
        )
        .expect("positive RED repository surface must be valid");

        T5RuntimeProfile::new(
            T5_RUNTIME_PROFILE_VERSION_V1,
            vec![repository.clone()],
            vec![ephemeral],
            vec![repository],
            NetworkPolicy {
                external_network: ExternalNetworkPolicy::Denied,
                local_inference_transport: LocalInferenceTransport::HostLocalOnly,
            },
            ToolPolicy::new(Vec::new(), Vec::new(), Vec::new())
                .expect("positive RED tool policy must be valid"),
            vec![repository_surface],
        )
        .expect("positive RED runtime profile must be valid")
    }

    fn complete_intact_integrity_report() -> IntegrityReport {
        let snapshot = StateSurfaceSnapshot {
            path: "C:/sol".to_owned(),
            kind: StateSurfaceKind::Repository,
            content_hash: Some(compute_surface_digest(b"stable-positive-red-fixture")),
            exists: true,
        };

        let mut pre = BTreeMap::new();
        pre.insert("C:/sol".to_owned(), snapshot.clone());

        let mut post = BTreeMap::new();
        post.insert("C:/sol".to_owned(), snapshot);

        evaluate_integrity(pre, post)
    }

    #[test]
    fn complete_bound_clean_observations_satisfy_observation_contract() {
        let profile = runtime_profile();
        let execution_id = execution_id();

        let integrity =
            GovernedIntegrityRecord::new(execution_id.clone(), complete_intact_integrity_report());

        let containment = GovernedContainmentRecord::new(
            execution_id.clone(),
            ContainmentCoverageStatus::Complete,
            ContainmentTelemetry::new(),
        );

        let disposition = evaluate_bound_governed_observation_contract(
            &profile,
            &execution_id,
            Some(&integrity),
            Some(&containment),
        );

        assert_eq!(disposition, ObservationContractDisposition::Satisfied);
    }
}

// --- EXP-BETA-003 RED TESTS: INTEGRITY FAILURE TAXONOMY ---

mod integrity_taxonomy_red {
    use std::collections::BTreeMap;

    use beta001_harness::injected_inference_runtime::{
        evaluate_bound_governed_observation_contract, ContainmentCoverageStatus, ExecutionId,
        GovernedContainmentRecord, GovernedIntegrityRecord, ObservationContractBlocker,
        ObservationContractDisposition,
    };
    use beta001_harness::integrity::{
        compute_surface_digest, evaluate_integrity, IntegrityReport, StateSurfaceSnapshot,
        INTEGRITY_PLANE_VERSION,
    };
    use beta001_harness::runtime_profile::{
        DeclaredPath, EvidenceTreatment, ExternalNetworkPolicy, LocalInferenceTransport,
        NetworkPolicy, PersistenceClass, StateSurface, StateSurfaceKind, T5RuntimeProfile,
        ToolPolicy, T5_RUNTIME_PROFILE_VERSION_V1,
    };
    use beta001_harness::telemetry::ContainmentTelemetry;

    fn execution_id() -> ExecutionId {
        ExecutionId::new("EXEC-INTEGRITY-TAXONOMY-001")
            .expect("integrity taxonomy execution identity must be valid")
    }

    fn runtime_profile() -> T5RuntimeProfile {
        let repository =
            DeclaredPath::new("C:/sol").expect("integrity taxonomy repository path must be valid");
        let ephemeral =
            DeclaredPath::new("C:/tmp").expect("integrity taxonomy ephemeral path must be valid");

        let repository_surface = StateSurface::new(
            "repository",
            StateSurfaceKind::Repository,
            Some(repository.clone()),
            PersistenceClass::Persistent,
            EvidenceTreatment::PrePostIntegrity,
        )
        .expect("integrity taxonomy repository surface must be valid");

        T5RuntimeProfile::new(
            T5_RUNTIME_PROFILE_VERSION_V1,
            vec![repository.clone()],
            vec![ephemeral],
            vec![repository],
            NetworkPolicy {
                external_network: ExternalNetworkPolicy::Denied,
                local_inference_transport: LocalInferenceTransport::HostLocalOnly,
            },
            ToolPolicy::new(Vec::new(), Vec::new(), Vec::new())
                .expect("integrity taxonomy tool policy must be valid"),
            vec![repository_surface],
        )
        .expect("integrity taxonomy runtime profile must be valid")
    }

    fn snapshot(bytes: &[u8]) -> StateSurfaceSnapshot {
        StateSurfaceSnapshot {
            path: "C:/sol".to_owned(),
            kind: StateSurfaceKind::Repository,
            content_hash: Some(compute_surface_digest(bytes)),
            exists: true,
        }
    }

    fn incomplete_integrity_report() -> IntegrityReport {
        IntegrityReport {
            version: INTEGRITY_PLANE_VERSION,
            pre_snapshots: BTreeMap::new(),
            post_snapshots: BTreeMap::new(),
            mutated_surfaces: Vec::new(),
            is_intact: true,
        }
    }

    fn inconsistent_integrity_report() -> IntegrityReport {
        let stable = snapshot(b"stable-integrity-taxonomy-fixture");

        let mut pre = BTreeMap::new();
        pre.insert("C:/sol".to_owned(), stable.clone());

        let mut post = BTreeMap::new();
        post.insert("C:/sol".to_owned(), stable);

        let mut report = evaluate_integrity(pre, post);

        assert!(report.is_intact);
        assert!(report.mutated_surfaces.is_empty());

        report.is_intact = false;
        report
    }

    fn changed_integrity_report() -> IntegrityReport {
        let mut pre = BTreeMap::new();
        pre.insert(
            "C:/sol".to_owned(),
            snapshot(b"pre-integrity-taxonomy-fixture"),
        );

        let mut post = BTreeMap::new();
        post.insert(
            "C:/sol".to_owned(),
            snapshot(b"post-integrity-taxonomy-fixture"),
        );

        let report = evaluate_integrity(pre, post);

        assert!(!report.is_intact);
        assert_eq!(report.mutated_surfaces, vec!["C:/sol".to_owned()]);

        report
    }

    fn complete_clean_containment(execution_id: &ExecutionId) -> GovernedContainmentRecord {
        GovernedContainmentRecord::new(
            execution_id.clone(),
            ContainmentCoverageStatus::Complete,
            ContainmentTelemetry::new(),
        )
    }

    #[test]
    fn incomplete_required_integrity_capture_is_distinct_from_missing_observation() {
        let profile = runtime_profile();
        let execution_id = execution_id();

        let integrity =
            GovernedIntegrityRecord::new(execution_id.clone(), incomplete_integrity_report());
        let containment = complete_clean_containment(&execution_id);

        let disposition = evaluate_bound_governed_observation_contract(
            &profile,
            &execution_id,
            Some(&integrity),
            Some(&containment),
        );

        assert_eq!(
            disposition,
            ObservationContractDisposition::NotSatisfied {
                blockers: vec![ObservationContractBlocker::IntegrityCoverageIncomplete],
            }
        );
    }

    #[test]
    fn supplied_integrity_claim_disagreeing_with_recomputation_is_inconsistent() {
        let profile = runtime_profile();
        let execution_id = execution_id();

        let integrity =
            GovernedIntegrityRecord::new(execution_id.clone(), inconsistent_integrity_report());
        let containment = complete_clean_containment(&execution_id);

        let disposition = evaluate_bound_governed_observation_contract(
            &profile,
            &execution_id,
            Some(&integrity),
            Some(&containment),
        );

        assert_eq!(
            disposition,
            ObservationContractDisposition::NotSatisfied {
                blockers: vec![ObservationContractBlocker::IntegrityReportInconsistent],
            }
        );
    }

    #[test]
    fn deterministically_recomputed_governed_state_mutation_is_integrity_changed() {
        let profile = runtime_profile();
        let execution_id = execution_id();

        let integrity =
            GovernedIntegrityRecord::new(execution_id.clone(), changed_integrity_report());
        let containment = complete_clean_containment(&execution_id);

        let disposition = evaluate_bound_governed_observation_contract(
            &profile,
            &execution_id,
            Some(&integrity),
            Some(&containment),
        );

        assert_eq!(
            disposition,
            ObservationContractDisposition::NotSatisfied {
                blockers: vec![ObservationContractBlocker::IntegrityChanged],
            }
        );
    }

    #[test]
    fn integrity_plane_version_mismatch_is_report_inconsistent() {
        let profile = runtime_profile();
        let execution_id = execution_id();

        let stable = snapshot(b"stable-version-mismatch-fixture");

        let mut pre = BTreeMap::new();
        pre.insert("C:/sol".to_owned(), stable.clone());

        let mut post = BTreeMap::new();
        post.insert("C:/sol".to_owned(), stable);

        let mut report = evaluate_integrity(pre, post);

        assert!(report.is_intact);
        assert!(report.mutated_surfaces.is_empty());

        report.version = INTEGRITY_PLANE_VERSION
            .checked_add(1)
            .expect("integrity plane test version must permit a distinct mismatch value");

        let integrity = GovernedIntegrityRecord::new(execution_id.clone(), report);
        let containment = complete_clean_containment(&execution_id);

        let disposition = evaluate_bound_governed_observation_contract(
            &profile,
            &execution_id,
            Some(&integrity),
            Some(&containment),
        );

        assert_eq!(
            disposition,
            ObservationContractDisposition::NotSatisfied {
                blockers: vec![ObservationContractBlocker::IntegrityReportInconsistent],
            }
        );
    }

    #[test]
    fn supplied_mutated_surfaces_disagreeing_with_recomputation_is_inconsistent() {
        let profile = runtime_profile();
        let execution_id = execution_id();

        let stable = snapshot(b"stable-mutated-surfaces-fixture");

        let mut pre = BTreeMap::new();
        pre.insert("C:/sol".to_owned(), stable.clone());

        let mut post = BTreeMap::new();
        post.insert("C:/sol".to_owned(), stable);

        let mut report = evaluate_integrity(pre, post);

        assert!(report.is_intact);
        assert!(report.mutated_surfaces.is_empty());

        report.mutated_surfaces = vec!["C:/sol".to_owned()];

        let integrity = GovernedIntegrityRecord::new(execution_id.clone(), report);
        let containment = complete_clean_containment(&execution_id);

        let disposition = evaluate_bound_governed_observation_contract(
            &profile,
            &execution_id,
            Some(&integrity),
            Some(&containment),
        );

        assert_eq!(
            disposition,
            ObservationContractDisposition::NotSatisfied {
                blockers: vec![ObservationContractBlocker::IntegrityReportInconsistent],
            }
        );
    }
}
