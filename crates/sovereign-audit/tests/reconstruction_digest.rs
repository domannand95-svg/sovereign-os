use sovereign_audit::{
    AuditEventType,
    AuditLedgerEntry,
    Digest,
    ReconstructionAnomaly,
    ReconstructionStatus,
};

fn fixture_entry() -> AuditLedgerEntry {
    AuditLedgerEntry::new(
        0,
        Digest("genesis".to_owned()),
        AuditEventType::ExecutionAttempted,
        Digest("subject".to_owned()),
        Digest("payload".to_owned()),
        "2026-08-22T00:00:00Z".to_owned(),
        sovereign_audit::AgentIdentityId("agent-test".to_owned()),
    )
}

#[test]
fn valid_entry_passes_digest_verification() {
    let entry = fixture_entry();

    let report = sovereign_audit::AuditReconstructionReport::reconstruct_entries(&[entry]);

    assert_eq!(report.status, ReconstructionStatus::Valid);
    assert!(report.anomalies.is_empty());
}

#[test]
fn tampered_entry_is_classified_as_integrity_failure() {
    let mut entry = fixture_entry();

    entry.entry_digest = Digest("tampered".to_owned());

    let report = sovereign_audit::AuditReconstructionReport::reconstruct_entries(&[entry]);

    assert_eq!(report.status, ReconstructionStatus::Invalid);

    assert_eq!(
        report.anomalies,
        vec![ReconstructionAnomaly::EntryIntegrityFailure {
            sequence: 0
        }]
    );
}