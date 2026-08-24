//! Boundary Tests for ADAM-013-E
//!
//! Validates portable snapshot export bundles, standalone attestation verification,
//! and fail-closed cryptographic checks (E013-001..E013-005).

use tempfile::tempdir;

use beta001_harness::state::{StateMutation, StateTree};
use beta001_harness::storage::{ExportError, SnapshotExporter, StateSnapshot};

#[test]
fn test_e013_001_snapshot_export_and_independent_attestation_verification() {
    let dir = tempdir().unwrap();
    let snap_path = dir.path().join("state_export.snap");

    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[
        StateMutation::put(b"config:cluster", b"global_cluster_0"),
        StateMutation::put(b"ledger:node_alpha", b"1000000"),
        StateMutation::put(b"ledger:node_beta", b"500000"),
    ]);

    let seq_tick = 88;
    let trans_root = "trans_root_proof_canonical";

    StateSnapshot::write_atomic(&snap_path, &tree, seq_tick, trans_root).unwrap();

    // Export bundle
    let bundle = SnapshotExporter::export_bundle(&snap_path).unwrap();
    assert_eq!(bundle.manifest_sequence_tick, 88);
    assert_eq!(bundle.manifest_revision, 1);
    assert_eq!(bundle.manifest_entry_count, 3);
    assert_eq!(bundle.manifest_state_root, tree.compute_state_root());
    assert_eq!(bundle.manifest_transition_root, trans_root);

    // Verify bundle independently
    let attestation = SnapshotExporter::verify_bundle(&bundle).unwrap();
    assert_eq!(attestation.sequence_tick, 88);
    assert_eq!(attestation.revision, 1);
    assert_eq!(attestation.entry_count, 3);
    assert_eq!(attestation.state_root, tree.compute_state_root());
    assert_eq!(attestation.transition_root, trans_root);
    assert_eq!(attestation.bundle_digest, bundle.bundle_digest);
}

#[test]
fn test_e013_002_tampered_bundle_bytes_fails_closed() {
    let dir = tempdir().unwrap();
    let snap_path = dir.path().join("tampered.snap");

    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"k", b"v")]);

    StateSnapshot::write_atomic(&snap_path, &tree, 1, "tr_1").unwrap();
    let mut bundle = SnapshotExporter::export_bundle(&snap_path).unwrap();

    // Tamper with payload byte inside raw_snapshot_bytes
    if let Some(byte) = bundle.raw_snapshot_bytes.get_mut(15) {
        *byte ^= 0xFF;
    }

    let res = SnapshotExporter::verify_bundle(&bundle);
    assert!(matches!(res, Err(ExportError::BundleDigestMismatch { .. })));
}

#[test]
fn test_e013_003_tampered_manifest_state_root_fails_closed() {
    let dir = tempdir().unwrap();
    let snap_path = dir.path().join("tampered_manifest.snap");

    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"k", b"v")]);

    StateSnapshot::write_atomic(&snap_path, &tree, 1, "tr_1").unwrap();
    let mut bundle = SnapshotExporter::export_bundle(&snap_path).unwrap();

    // Forge manifest state root while recalculating bundle digest
    bundle.manifest_state_root = "forged_state_root_00000000000000000000000000000000".to_string();
    let manifest = beta001_harness::storage::SnapshotManifest {
        sequence_tick: bundle.manifest_sequence_tick,
        revision: bundle.manifest_revision,
        state_root: bundle.manifest_state_root.clone(),
        transition_root: bundle.manifest_transition_root.clone(),
        entry_count: bundle.manifest_entry_count,
        checksum: bundle.manifest_checksum,
    };
    bundle.bundle_digest =
        SnapshotExporter::compute_bundle_digest(&manifest, &bundle.raw_snapshot_bytes);

    let res = SnapshotExporter::verify_bundle(&bundle);
    assert!(matches!(res, Err(ExportError::ManifestMismatch(_))));
}

#[test]
fn test_e013_004_json_serialization_portability() {
    let dir = tempdir().unwrap();
    let snap_path = dir.path().join("portable.snap");

    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"key:portable", b"val:portable")]);

    StateSnapshot::write_atomic(&snap_path, &tree, 12, "tr_portable").unwrap();
    let bundle = SnapshotExporter::export_bundle(&snap_path).unwrap();

    // Serialize to JSON and deserialize (simulating network transmission)
    let json_str = serde_json::to_string(&bundle).unwrap();
    let deserialized_bundle: beta001_harness::storage::ExportableSnapshotBundle =
        serde_json::from_str(&json_str).unwrap();

    assert_eq!(bundle, deserialized_bundle);
    let attestation = SnapshotExporter::verify_bundle(&deserialized_bundle).unwrap();
    assert_eq!(attestation.sequence_tick, 12);
    assert_eq!(attestation.state_root, tree.compute_state_root());
}
