//! Boundary Tests for ADAM-013-B
//!
//! Validates crash recovery, torn-tail truncation, interior corruption fail-closed policy,
//! and causal lineage verification (B013-001..B013-006).

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use tempfile::tempdir;

use beta001_harness::service_contract::{ExecutionId, Sha256Digest};
use beta001_harness::state::{
    compute_delta_digest, compute_genesis_transition_root, compute_transition_root, StateMutation,
    StateTree,
};
use beta001_harness::storage::{
    CommitLogRecovery, CommitLogWriter, CommitRecordPayload, RecoveryError, SyncPolicy,
};

fn create_valid_payload(
    exec_id_str: &str,
    tree: &mut StateTree,
    prior_trans_root: &str,
    mutations: Vec<StateMutation>,
) -> CommitRecordPayload {
    let pre_rev = tree.revision();
    let pre_state_root = tree.compute_state_root();

    let mut staging_tree = tree.clone();
    staging_tree.apply_raw_mutations(&mutations);
    let post_rev = staging_tree.revision();
    let post_state_root = staging_tree.compute_state_root();

    let delta_digest = compute_delta_digest(&mutations);
    let pkg_digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    let transition_root = compute_transition_root(
        prior_trans_root,
        &pre_state_root,
        &post_state_root,
        pkg_digest,
        exec_id_str,
        &delta_digest,
    );

    // Commit mutations to tree
    tree.apply_raw_mutations(&mutations);

    CommitRecordPayload {
        execution_id: ExecutionId::new(exec_id_str.to_string()).unwrap(),
        package_digest: Sha256Digest::new(pkg_digest.to_string()).unwrap(),
        pre_revision: pre_rev,
        post_revision: post_rev,
        pre_state_root,
        post_state_root,
        prior_transition_root: prior_trans_root.to_string(),
        transition_root,
        mutations,
    }
}

#[test]
fn test_b013_001_cold_start_recovery_restores_identical_state_and_lineage() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("commit.log");

    let writer = CommitLogWriter::open(&log_path, SyncPolicy::FsyncOnCommit).unwrap();
    let mut ground_truth_tree = StateTree::new();
    let mut current_trans_root = compute_genesis_transition_root();

    for i in 1..=4 {
        let exec_id = format!("exe_rec_{:03}", i);
        let mutations = vec![StateMutation::put(
            format!("key:{}", i).into_bytes(),
            format!("val:{}", i).into_bytes(),
        )];
        let payload = create_valid_payload(
            &exec_id,
            &mut ground_truth_tree,
            &current_trans_root,
            mutations,
        );
        current_trans_root = payload.transition_root.clone();
        writer.append(i, payload).unwrap();
    }

    // Cold-start recovery on fresh tree
    let mut recovered_tree = StateTree::new();
    let report =
        CommitLogRecovery::recover_from_log(&log_path, &mut recovered_tree, None, true).unwrap();

    assert_eq!(report.recovered_records_count, 4);
    assert_eq!(report.last_sequence_tick, 4);
    assert_eq!(report.recovered_revision, 4);
    assert_eq!(
        recovered_tree.compute_state_root(),
        ground_truth_tree.compute_state_root()
    );
    assert_eq!(report.recovered_transition_root, current_trans_root);
}

#[test]
fn test_b013_002_torn_tail_incomplete_frame_truncated_safely() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("torn_tail.log");

    let writer = CommitLogWriter::open(&log_path, SyncPolicy::FsyncOnCommit).unwrap();
    let mut tree = StateTree::new();
    let trans_root_0 = compute_genesis_transition_root();

    let p1 = create_valid_payload(
        "exe_torn_001",
        &mut tree,
        &trans_root_0,
        vec![StateMutation::put(b"k1", b"v1")],
    );
    writer.append(1, p1).unwrap();

    let p2 = create_valid_payload(
        "exe_torn_002",
        &mut tree,
        &trans_root_0,
        vec![StateMutation::put(b"k2", b"v2")],
    );
    writer.append(2, p2).unwrap();

    let valid_len = File::open(&log_path).unwrap().metadata().unwrap().len();

    // Append 15 bytes of junk representing an interrupted torn write at the tail
    {
        let mut file = OpenOptions::new().append(true).open(&log_path).unwrap();
        file.write_all(b"SOVLOG01_TORN_T").unwrap();
    }

    let mut rec_tree = StateTree::new();
    let report = CommitLogRecovery::recover_from_log(&log_path, &mut rec_tree, None, true).unwrap();

    assert_eq!(report.recovered_records_count, 2);
    assert_eq!(report.last_sequence_tick, 2);
    assert_eq!(report.torn_tail_truncated_bytes, 15);
    assert_eq!(
        File::open(&log_path).unwrap().metadata().unwrap().len(),
        valid_len
    );
}

#[test]
fn test_b013_003_interior_bit_flip_fails_closed_without_truncation() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("interior_corrupt.log");

    let writer = CommitLogWriter::open(&log_path, SyncPolicy::FsyncOnCommit).unwrap();
    let mut tree = StateTree::new();
    let mut trans_root = compute_genesis_transition_root();

    for i in 1..=3 {
        let p = create_valid_payload(
            &format!("exe_int_{:03}", i),
            &mut tree,
            &trans_root,
            vec![StateMutation::put(format!("k{}", i).into_bytes(), b"v")],
        );
        trans_root = p.transition_root.clone();
        writer.append(i, p).unwrap();
    }

    let original_len = File::open(&log_path).unwrap().metadata().unwrap().len();

    // Flip bit in the first record payload (interior)
    {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&log_path)
            .unwrap();
        file.seek(SeekFrom::Start(40)).unwrap();
        let mut byte = [0u8; 1];
        file.read_exact(&mut byte).unwrap();
        byte[0] ^= 0xAA;
        file.seek(SeekFrom::Start(40)).unwrap();
        file.write_all(&byte).unwrap();
    }

    let mut rec_tree = StateTree::new();
    let res = CommitLogRecovery::recover_from_log(&log_path, &mut rec_tree, None, true);

    assert!(matches!(res, Err(RecoveryError::InteriorCorruption { .. })));
    // Ensure file was NOT truncated on interior corruption
    assert_eq!(
        File::open(&log_path).unwrap().metadata().unwrap().len(),
        original_len
    );
}

#[test]
fn test_b013_004_sequence_gap_fails_closed() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("seq_gap.log");

    let writer = CommitLogWriter::open(&log_path, SyncPolicy::FsyncOnCommit).unwrap();
    let mut tree = StateTree::new();
    let trans_root_0 = compute_genesis_transition_root();

    let p1 = create_valid_payload(
        "exe_gap_001",
        &mut tree,
        &trans_root_0,
        vec![StateMutation::put(b"k1", b"v1")],
    );
    writer.append(1, p1.clone()).unwrap();

    // Append sequence tick 3 (gap skipping tick 2)
    let p2 = create_valid_payload(
        "exe_gap_002",
        &mut tree,
        &p1.transition_root,
        vec![StateMutation::put(b"k2", b"v2")],
    );
    writer.append(3, p2).unwrap();

    let mut rec_tree = StateTree::new();
    let res = CommitLogRecovery::recover_from_log(&log_path, &mut rec_tree, None, true);
    assert!(matches!(
        res,
        Err(RecoveryError::SequenceGap {
            expected: 2,
            actual: 3,
            ..
        })
    ));
}

#[test]
fn test_b013_005_state_root_lineage_mismatch_fails_closed() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("root_mismatch.log");

    let writer = CommitLogWriter::open(&log_path, SyncPolicy::FsyncOnCommit).unwrap();
    let mut tree = StateTree::new();
    let trans_root_0 = compute_genesis_transition_root();

    let mut p1 = create_valid_payload(
        "exe_root_001",
        &mut tree,
        &trans_root_0,
        vec![StateMutation::put(b"k1", b"v1")],
    );
    // Forcibly falsify pre_state_root in record payload
    p1.pre_state_root = "forged_non_matching_root".to_string();
    writer.append(1, p1).unwrap();

    let mut rec_tree = StateTree::new();
    let res = CommitLogRecovery::recover_from_log(&log_path, &mut rec_tree, None, true);
    assert!(matches!(res, Err(RecoveryError::StateRootMismatch { .. })));
}
