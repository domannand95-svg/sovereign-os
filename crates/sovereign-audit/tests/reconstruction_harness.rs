use sovereign_audit::{
    AuditEventType,
    AuditLedgerEntry,
    AuditReconstructionReport,
    Digest,
};
use sovereign_audit::AgentIdentityId;

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
fn test_valid_historical_chain_reconstructs_cleanly() {
    let first = fixture_entry(0, genesis_digest());

    let second = fixture_entry(1, first.entry_digest.clone());

    let entries = vec![first, second];

    let report = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(
        report.status,
        sovereign_audit::ReconstructionStatus::Valid
    );

    assert!(report.anomalies.is_empty());

    assert_eq!(report.total_entries_inspected, 2);

    assert_eq!(
        report.genesis_digest,
        Some(entries[0].entry_digest.clone())
    );

    assert_eq!(
        report.head_digest,
        Some(entries[1].entry_digest.clone())
    );
}