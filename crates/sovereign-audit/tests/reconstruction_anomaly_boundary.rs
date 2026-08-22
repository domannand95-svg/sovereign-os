use sovereign_audit::{
    AgentIdentityId, AuditEventType, AuditLedgerEntry, AuditReconstructionReport, Digest,
    ReconstructionAnomaly,
};

#[test]
fn test_integrity_failure_remains_observation_only() {
    let mut entry = AuditLedgerEntry::new(
        0,
        Digest("genesis".into()),
        AuditEventType::AuthorizationRecorded,
        Digest("subject".into()),
        Digest("payload".into()),
        "2026-08-22T00:00:00Z".to_string(),
        AgentIdentityId("agent-test".into()),
    );

    // Deliberately corrupt the entry digest.
    // The reconstruction layer must observe the failure, not repair it.
    entry.entry_digest = Digest("tampered".into());

    let entries = vec![entry.clone()];

    let report = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(report.total_entries_inspected, 1);

    assert!(report
        .anomalies
        .contains(&ReconstructionAnomaly::EntryIntegrityFailure { sequence: 0 }));

    // Boundary assertion:
    // the original evidence remains unchanged after reconstruction.
    assert_eq!(entries[0].entry_digest, Digest("tampered".into()));
}
