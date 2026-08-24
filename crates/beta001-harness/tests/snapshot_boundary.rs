//! Boundary Tests for ADAM-013-C
//!
//! Validates canonical binary snapshot serialization, atomic publication,
//! checksum verification, state root recomputation, and determinism (C013-001..C013-006).

use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use tempfile::tempdir;

use beta001_harness::state::{StateMutation, StateTree};
use beta001_harness::storage::{SnapshotError, StateSnapshot, SNAPSHOT_MAGIC};

#[test]
fn test_c013_001_snapshot_write_atomic_and_verified_hydration() {
    let dir = tempdir().unwrap();
    let snap_path = dir.path().join("state_001.snap");

    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[
        StateMutation::put(b"config:node_id", b"node_77"),
        StateMutation::put(b"balance:alice", b"50000"),
        StateMutation::put(b"balance:bob", b"25000"),
    ]);

    let seq_tick = 42;
    let trans_root = "trans_root_canonical_001";

    let manifest = StateSnapshot::write_atomic(&snap_path, &tree, seq_tick, trans_root).unwrap();

    assert_eq!(manifest.sequence_tick, 42);
    assert_eq!(manifest.revision, 1);
    assert_eq!(manifest.entry_count, 3);
    assert_eq!(manifest.state_root, tree.compute_state_root());
    assert_eq!(manifest.transition_root, trans_root);

    let (hydrated_tree, loaded_manifest) = StateSnapshot::load_and_verify(&snap_path).unwrap();
    assert_eq!(loaded_manifest, manifest);
    assert_eq!(
        hydrated_tree.compute_state_root(),
        tree.compute_state_root()
    );
    assert_eq!(hydrated_tree.revision(), tree.revision());
    assert_eq!(
        hydrated_tree.get(b"balance:alice"),
        Some(b"50000".as_slice())
    );
    assert_eq!(
        hydrated_tree.get(b"config:node_id"),
        Some(b"node_77".as_slice())
    );
}

#[test]
fn test_c013_002_corrupted_snapshot_checksum_fails_closed() {
    let dir = tempdir().unwrap();
    let snap_path = dir.path().join("corrupt_checksum.snap");

    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"k1", b"v1")]);

    StateSnapshot::write_atomic(&snap_path, &tree, 1, "tr_1").unwrap();

    let file_len = File::open(&snap_path).unwrap().metadata().unwrap().len();
    {
        let mut file = OpenOptions::new().write(true).open(&snap_path).unwrap();
        file.seek(SeekFrom::Start(file_len - 5)).unwrap();
        file.write_all(&[0xFF]).unwrap();
    }

    let res = StateSnapshot::load_and_verify(&snap_path);
    assert!(matches!(res, Err(SnapshotError::ChecksumMismatch { .. })));
}

#[test]
fn test_c013_003_invalid_magic_fails_closed() {
    let dir = tempdir().unwrap();
    let snap_path = dir.path().join("invalid_magic.snap");

    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"k1", b"v1")]);

    StateSnapshot::write_atomic(&snap_path, &tree, 1, "tr_1").unwrap();

    {
        let mut file = OpenOptions::new().write(true).open(&snap_path).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(b"BADMAGIC").unwrap();
    }

    let res = StateSnapshot::load_and_verify(&snap_path);
    assert!(matches!(res, Err(SnapshotError::InvalidMagic(_))));
}

#[test]
fn test_c013_004_canonical_snapshot_encoding_is_bitwise_deterministic() {
    let dir = tempdir().unwrap();
    let snap_path1 = dir.path().join("state_run1.snap");
    let snap_path2 = dir.path().join("state_run2.snap");

    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[
        StateMutation::put(b"zeta", b"100"),
        StateMutation::put(b"alpha", b"200"),
        StateMutation::put(b"gamma", b"300"),
    ]);

    StateSnapshot::write_atomic(&snap_path1, &tree, 10, "tr_det").unwrap();
    StateSnapshot::write_atomic(&snap_path2, &tree, 10, "tr_det").unwrap();

    let bytes1 = std::fs::read(&snap_path1).unwrap();
    let bytes2 = std::fs::read(&snap_path2).unwrap();

    assert_eq!(bytes1, bytes2);
    assert_eq!(&bytes1[0..8], SNAPSHOT_MAGIC);
}

#[test]
fn test_c013_005_atomic_publication_cleans_up_temp_file() {
    let dir = tempdir().unwrap();
    let snap_path = dir.path().join("atomic_check.snap");

    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"k", b"v")]);

    StateSnapshot::write_atomic(&snap_path, &tree, 5, "tr_5").unwrap();

    assert!(snap_path.exists());
    let tmp_path = snap_path.with_extension("tmp.5");
    assert!(!tmp_path.exists());
}
