use sovereign_audit::{AuditReconstructionReport, ReconstructionStatus};

#[test]
fn test_authority_boundary_has_no_execution_surface() {
    let report = AuditReconstructionReport::empty();

    assert_eq!(report.total_entries_inspected, 0);
    assert_eq!(report.status, ReconstructionStatus::Valid);
    assert!(report.anomalies.is_empty());

    assert_eq!(report.genesis_digest, None);
    assert_eq!(report.head_digest, None);
}
