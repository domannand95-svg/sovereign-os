use sovereign_audit::{
    AuditEventType,
    AuditLedgerEntry,
    AuditReconstructionReport,
    AgentIdentityId,
    Digest,
    ReconstructionStatus,
};

fn valid_entry(sequence: u64, previous: Digest) -> AuditLedgerEntry {
    AuditLedgerEntry::new(
        sequence,
        previous,
        AuditEventType::AuthorizationRecorded,
        Digest("subject".into()),
        Digest("payload".into()),
        "2026-08-22T00:00:00Z".to_string(),
        AgentIdentityId("agent-test".into()),
    )
}

#[test]
fn test_beta017_reconstruction_contract_remains_unchanged() {
    let first = valid_entry(0, Digest("genesis".into()));
    let entries = vec![first];

    let before = AuditReconstructionReport::reconstruct_entries(&entries);
    let after = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(before, after);

    assert_eq!(before.total_entries_inspected, 1);
    assert_eq!(before.status, ReconstructionStatus::Valid);
    assert!(before.anomalies.is_empty());
}