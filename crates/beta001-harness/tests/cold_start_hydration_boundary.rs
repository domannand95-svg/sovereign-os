//! Boundary Tests for ADAM-013-D
//!
//! Validates cold-start hydration, snapshot candidate selection, crash-safe compaction,
//! and state/lineage invariance (D013-001..D013-006).

use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

use beta001_harness::service_contract::{ExecutionId, Sha256Digest};
use beta001_harness::state::{
    compute_delta_digest, compute_genesis_transition_root, compute_transition_root, StateMutation,
    StateTree,
};
use beta001_harness::storage::{CommitRecordPayload, StorageEngine, SyncPolicy};

fn build_payload(
    exec_id_str: &str,
    tree: &StateTree,
    prior_trans_root: &str,
    mutations: Vec<StateMutation>,
) -> CommitRecordPayload {
    let pre_rev = tree.revision();
    let pre_state_root = tree.compute_state_root();

    let mut staging = tree.clone();
    staging.apply_raw_mutations(&mutations);
    let post_rev = staging.revision();
    let post_state_root = staging.compute_state_root();

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
fn test_d013_001_clean_state_initializes_at_genesis() {
    let dir = tempdir().unwrap();
    let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();

    assert_eq!(engine.current_sequence_tick(), 0);
    assert_eq!(engine.current_revision(), 0);
    assert_eq!(
        engine.current_transition_root(),
        compute_genesis_transition_root()
    );
    assert_eq!(
        engine.current_state_root(),
        StateTree::new().compute_state_root()
    );
}

#[test]
fn test_d013_002_cold_start_hydration_from_log_only() {
    let dir = tempdir().unwrap();

    let ground_truth_tree;
    let ground_truth_trans;

    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
        let mut mock_tree = StateTree::new();
        let mut current_tr = compute_genesis_transition_root();

        for i in 1..=5 {
            let mutations = vec![StateMutation::put(
                format!("k_{}", i).into_bytes(),
                format!("v_{}", i).into_bytes(),
            )];
            let payload = build_payload(&format!("exe_{}", i), &mock_tree, &current_tr, mutations);
            current_tr = payload.transition_root.clone();
            mock_tree.apply_raw_mutations(&payload.mutations);
            engine.commit_record(i, payload).unwrap();
        }

        ground_truth_tree = mock_tree.compute_state_root();
        ground_truth_trans = current_tr;
    }

    // Cold-start re-open
    let engine2 = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(engine2.current_sequence_tick(), 5);
    assert_eq!(engine2.current_revision(), 5);
    assert_eq!(engine2.current_state_root(), ground_truth_tree);
    assert_eq!(engine2.current_transition_root(), ground_truth_trans);
    assert_eq!(engine2.get_state_value(b"k_3"), Some(b"v_3".to_vec()));
}

#[test]
fn test_d013_003_cold_start_hydration_from_snapshot_plus_log_tail() {
    let dir = tempdir().unwrap();

    let expected_state_root;
    let expected_trans_root;

    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
        let mut mock_tree = StateTree::new();
        let mut current_tr = compute_genesis_transition_root();

        // Write 3 records
        for i in 1..=3 {
            let mutations = vec![StateMutation::put(
                format!("k_{}", i).into_bytes(),
                format!("v_{}", i).into_bytes(),
            )];
            let payload = build_payload(&format!("exe_{}", i), &mock_tree, &current_tr, mutations);
            current_tr = payload.transition_root.clone();
            mock_tree.apply_raw_mutations(&payload.mutations);
            engine.commit_record(i, payload).unwrap();
        }

        // Publish snapshot at tick 3
        let snap_manifest = engine.create_snapshot().unwrap();
        assert_eq!(snap_manifest.sequence_tick, 3);

        // Write 2 more records (tail: 4 and 5)
        for i in 4..=5 {
            let mutations = vec![StateMutation::put(
                format!("k_{}", i).into_bytes(),
                format!("v_{}", i).into_bytes(),
            )];
            let payload = build_payload(&format!("exe_{}", i), &mock_tree, &current_tr, mutations);
            current_tr = payload.transition_root.clone();
            mock_tree.apply_raw_mutations(&payload.mutations);
            engine.commit_record(i, payload).unwrap();
        }

        expected_state_root = mock_tree.compute_state_root();
        expected_trans_root = current_tr;
    }

    // Cold-start re-open from Snapshot(3) + LogTail(4..5)
    let engine2 = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(engine2.current_sequence_tick(), 5);
    assert_eq!(engine2.current_revision(), 5);
    assert_eq!(engine2.current_state_root(), expected_state_root);
    assert_eq!(engine2.current_transition_root(), expected_trans_root);
    assert_eq!(engine2.get_state_value(b"k_1"), Some(b"v_1".to_vec()));
    assert_eq!(engine2.get_state_value(b"k_5"), Some(b"v_5".to_vec()));
}

#[test]
fn test_d013_004_candidate_snapshot_selection_ignores_corrupt_or_tmp_files() {
    let dir = tempdir().unwrap();

    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
        let mut mock_tree = StateTree::new();
        let mut current_tr = compute_genesis_transition_root();

        for i in 1..=2 {
            let mutations = vec![StateMutation::put(
                format!("k_{}", i).into_bytes(),
                format!("v_{}", i).into_bytes(),
            )];
            let payload = build_payload(&format!("exe_{}", i), &mock_tree, &current_tr, mutations);
            current_tr = payload.transition_root.clone();
            mock_tree.apply_raw_mutations(&payload.mutations);
            engine.commit_record(i, payload).unwrap();
        }

        engine.create_snapshot().unwrap(); // Valid snapshot at tick 2
    }

    // Inject invalid / corrupt snapshot candidates
    let tmp_path = dir.path().join("state_seq_00000000000000000099.snap.tmp");
    let mut f_tmp = File::create(&tmp_path).unwrap();
    f_tmp.write_all(b"UNFINISHED_TMP_PAYLOAD").unwrap();

    let corrupt_path = dir.path().join("state_seq_00000000000000000100.snap");
    let mut f_corrupt = File::create(&corrupt_path).unwrap();
    f_corrupt.write_all(b"CORRUPT_MAGIC_AND_BODY").unwrap();

    // Re-open: must safely select valid snapshot at tick 2 and ignore corrupt/tmp artifacts
    let engine2 = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(engine2.current_sequence_tick(), 2);
    assert_eq!(engine2.get_state_value(b"k_2"), Some(b"v_2".to_vec()));
}

#[test]
fn test_d013_005_crash_safe_compaction_prunes_log_prefix() {
    let dir = tempdir().unwrap();

    let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    let mut mock_tree = StateTree::new();
    let mut current_tr = compute_genesis_transition_root();

    for i in 1..=6 {
        let mutations = vec![StateMutation::put(
            format!("account:{}", i).into_bytes(),
            b"active".to_vec(),
        )];
        let payload = build_payload(&format!("exe_{}", i), &mock_tree, &current_tr, mutations);
        current_tr = payload.transition_root.clone();
        mock_tree.apply_raw_mutations(&payload.mutations);
        engine.commit_record(i, payload).unwrap();
    }

    // Compact at tick 6
    let report = engine.compact().unwrap();
    assert_eq!(report.snapshot_manifest.sequence_tick, 6);
    assert_eq!(report.pruned_records_count, 6);
    assert_eq!(report.retained_records_count, 0);

    // Verify commit.log is now pruned
    let log_file_len = File::open(dir.path().join("commit.log"))
        .unwrap()
        .metadata()
        .unwrap()
        .len();
    assert_eq!(log_file_len, 0);

    // Append post-compaction tick 7
    let mutations_7 = vec![StateMutation::put(b"account:7", b"active")];
    let payload_7 = build_payload("exe_7", &mock_tree, &current_tr, mutations_7);
    mock_tree.apply_raw_mutations(&payload_7.mutations);
    engine.commit_record(7, payload_7).unwrap();

    // Re-open and verify seamless recovery across compacted snapshot and new log record
    let engine2 = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(engine2.current_sequence_tick(), 7);
    assert_eq!(engine2.current_state_root(), mock_tree.compute_state_root());
    assert_eq!(
        engine2.get_state_value(b"account:1"),
        Some(b"active".to_vec())
    );
    assert_eq!(
        engine2.get_state_value(b"account:7"),
        Some(b"active".to_vec())
    );
}
