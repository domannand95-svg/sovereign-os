use sovereign_audit::{
    AuditEventType,
    AuditLedgerEntry,
    Digest,
    ReconstructionAnomaly,
    AuditReconstructionReport,
};

fn entry(sequence: u64, previous: Digest) -> AuditLedgerEntry {
    AuditLedgerEntry::new(
        sequence,
        previous,
        AuditEventType::ExecutionCommitted,
        Digest("subject".into()),
        Digest("payload".into()),
        "2026-08-22T00:00:00Z".into(),
        sovereign_audit::AgentIdentityId("agent-test".into()),
    )
}

#[test]
fn valid_linkage_has_no_anomalies() {
    let first = entry(0, Digest("genesis".into()));
    let second = entry(1, first.entry_digest.clone());

    let anomalies =
        AuditReconstructionReport::verify_linkage(&[first, second]);

    assert!(anomalies.is_empty());
}

#[test]
fn sequence_gap_is_detected() {
    let first = entry(0, Digest("genesis".into()));
    let second = entry(2, first.entry_digest.clone());

    let anomalies =
        AuditReconstructionReport::verify_linkage(&[first, second]);

    assert_eq!(
        anomalies,
        vec![ReconstructionAnomaly::SequenceGap {
            expected: 1,
            observed: 2,
        }]
    );
}

#[test]
fn previous_digest_mismatch_is_detected() {
    let first = entry(0, Digest("genesis".into()));
    let second = entry(1, Digest("wrong".into()));

    let anomalies =
        AuditReconstructionReport::verify_linkage(&[first, second]);

    assert_eq!(
        anomalies,
        vec![ReconstructionAnomaly::PreviousDigestMismatch {
            sequence: 1,
        }]
    );
}