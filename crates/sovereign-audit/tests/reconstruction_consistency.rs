use sovereign_audit::AgentIdentityId;
use sovereign_audit::{
    AuditEventType, AuditLedgerEntry, AuditReconstructionReport, Digest, ReconstructionStatus,
};

fn genesis_digest() -> Digest {
    Digest("genesis".into())
}

fn fixture_entry(sequence: u64, previous: Digest) -> AuditLedgerEntry {
    AuditLedgerEntry::new(
        sequence,
        previous,
        AuditEventType::ExecutionCommitted,
        Digest("subject".into()),
        Digest("payload".into()),
        "2026-08-22T00:00:00Z".into(),
        AgentIdentityId("agent-test".into()),
    )
}

#[test]
fn test_reconstruction_yields_consistent_observation() {
    let first = fixture_entry(0, genesis_digest());

    let second = fixture_entry(1, first.entry_digest.clone());

    let entries = vec![first, second];

    let report_a = AuditReconstructionReport::reconstruct_entries(&entries);

    let report_b = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(
        report_a, report_b,
        "identical evidence must yield identical reconstruction results"
    );

    assert_eq!(report_a.status, ReconstructionStatus::Valid);

    assert_eq!(report_a.total_entries_inspected, 2);

    assert_eq!(
        report_a.genesis_digest,
        Some(entries[0].entry_digest.clone())
    );

    assert_eq!(report_a.head_digest, Some(entries[1].entry_digest.clone()));

    assert!(
        report_a.anomalies.is_empty(),
        "deterministic valid reconstruction should contain no anomalies"
    );
}

#[test]
fn test_reconstruction_consistency_does_not_mutate_input() {
    let first = fixture_entry(0, genesis_digest());

    let second = fixture_entry(1, first.entry_digest.clone());

    let entries = vec![first, second];

    let before = entries.clone();

    let _report_a = AuditReconstructionReport::reconstruct_entries(&entries);
    let _report_b = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(
        entries, before,
        "reconstruction consistency observation must not mutate input evidence"
    );
}

#[test]
fn test_reconstruction_consistency_remains_stable_across_iterations() {
    let first = fixture_entry(0, genesis_digest());

    let second = fixture_entry(1, first.entry_digest.clone());

    let entries = vec![first, second];

    let baseline = AuditReconstructionReport::reconstruct_entries(&entries);

    for _ in 0..100 {
        let report = AuditReconstructionReport::reconstruct_entries(&entries);

        assert_eq!(
            report, baseline,
            "identical historical evidence must yield identical reconstruction results"
        );
    }
}
