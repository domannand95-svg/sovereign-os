use sovereign_audit::{
    AuditReconstructionReport,
    ReconstructionAnomaly,
    ReconstructionStatus,
};

#[test]
fn empty_report_can_be_constructed() {
    let report = AuditReconstructionReport::empty();

    assert_eq!(report.total_entries_inspected, 0);
    assert_eq!(report.status, ReconstructionStatus::Valid);
    assert!(report.anomalies.is_empty());
}

#[test]
fn reconstruction_status_variants_are_representable() {
    let _valid = ReconstructionStatus::Valid;
    let _partial = ReconstructionStatus::Partial;
    let _invalid = ReconstructionStatus::Invalid;
}

#[test]
fn anomaly_variants_are_constructible() {
    let _ = ReconstructionAnomaly::SequenceGap {
        expected: 1,
        observed: 3,
    };

    let _ = ReconstructionAnomaly::DuplicateSequence {
        sequence: 1,
    };

    let _ = ReconstructionAnomaly::PreviousDigestMismatch {
        sequence: 2,
    };

    let _ = ReconstructionAnomaly::EntryIntegrityFailure {
        sequence: 3,
    };

    let _ = ReconstructionAnomaly::UnexpectedGenesis;

    let _ = ReconstructionAnomaly::ConflictingEntry {
        sequence: 4,
    };

    let _ = ReconstructionAnomaly::OutOfOrderInput;
}