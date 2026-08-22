use sovereign_audit::AgentIdentityId;
use sovereign_audit::{AuditEventType, AuditLedgerEntry, AuditReconstructionReport, Digest};

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

    assert_eq!(report.status, sovereign_audit::ReconstructionStatus::Valid);

    assert!(report.anomalies.is_empty());

    assert_eq!(report.total_entries_inspected, 2);

    assert_eq!(report.genesis_digest, Some(entries[0].entry_digest.clone()));

    assert_eq!(report.head_digest, Some(entries[1].entry_digest.clone()));
}

#[test]
fn test_empty_evidence_slice_yields_valid_empty_report() {
    let entries: Vec<AuditLedgerEntry> = Vec::new();

    let report = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(report.status, sovereign_audit::ReconstructionStatus::Valid);

    assert_eq!(report.total_entries_inspected, 0);

    assert_eq!(report.genesis_digest, None);

    assert_eq!(report.head_digest, None);

    assert!(report.anomalies.is_empty());
}

#[test]
fn test_single_entry_slice_yields_valid_report() {
    let entry = fixture_entry(0, genesis_digest());

    let entries = vec![entry];

    let report = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(report.status, sovereign_audit::ReconstructionStatus::Valid);

    assert_eq!(report.total_entries_inspected, 1);

    assert_eq!(report.genesis_digest, Some(entries[0].entry_digest.clone()));

    assert_eq!(report.head_digest, Some(entries[0].entry_digest.clone()));

    assert!(report.anomalies.is_empty());
}

#[test]
fn test_tampered_entry_yields_integrity_failure() {
    let mut entry = fixture_entry(0, genesis_digest());

    entry.payload_digest = Digest("tampered".into());

    let entries = vec![entry];

    let report = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(
        report.status,
        sovereign_audit::ReconstructionStatus::Invalid
    );

    assert!(report
        .anomalies
        .contains(&sovereign_audit::ReconstructionAnomaly::EntryIntegrityFailure { sequence: 0 }));
}

#[test]
fn test_sequence_gap_yields_gap_anomaly() {
    let first = fixture_entry(0, genesis_digest());

    let second = fixture_entry(2, first.entry_digest.clone());

    let entries = vec![first, second];

    let report = AuditReconstructionReport::reconstruct_entries(&entries);

    assert!(report
        .anomalies
        .contains(&sovereign_audit::ReconstructionAnomaly::SequenceGap {
            expected: 1,
            observed: 2
        }));
}

#[test]
fn test_predecessor_hash_mismatch_yields_linkage_failure() {
    let first = fixture_entry(0, genesis_digest());

    let second = fixture_entry(1, Digest("wrong_previous_digest".into()));

    let entries = vec![first, second];

    let report = AuditReconstructionReport::reconstruct_entries(&entries);

    assert!(report
        .anomalies
        .contains(&sovereign_audit::ReconstructionAnomaly::PreviousDigestMismatch { sequence: 1 }));
}

#[test]
fn test_reconstruction_is_strictly_deterministic() {
    let first = fixture_entry(0, genesis_digest());

    let second = fixture_entry(1, first.entry_digest.clone());

    let entries = vec![first, second];

    let report_a = AuditReconstructionReport::reconstruct_entries(&entries);
    let report_b = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(report_a, report_b);
}

#[test]
fn test_reconstruction_does_not_mutate_input() {
    let first = fixture_entry(0, genesis_digest());

    let second = fixture_entry(1, first.entry_digest.clone());

    let entries = vec![first, second];

    let before = entries.clone();

    let _report = AuditReconstructionReport::reconstruct_entries(&entries);

    assert_eq!(entries, before);
}
