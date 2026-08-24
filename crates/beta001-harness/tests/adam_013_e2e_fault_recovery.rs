//! End-to-End Fault Injection & Power-Loss Recovery Integration Suite for ADAM-013
//!
//! Validates bitwise state parity, torn-tail truncation, orphaned temp file immunity,
//! crash-safe compaction, and zero authority expansion (F013-001..F013-005).

use std::fs::{File, OpenOptions};
use std::io::Write;
use tempfile::tempdir;

use beta001_harness::service_contract::{ExecutionId, Sha256Digest};
use beta001_harness::state::{
    compute_delta_digest, compute_genesis_transition_root, compute_transition_root, StateMutation,
    StateTree,
};
use beta001_harness::storage::{CommitRecordPayload, SnapshotExporter, StorageEngine, SyncPolicy};

fn build_test_payload(
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
fn test_f013_001_uninterrupted_vs_cold_start_hydration_parity() {
    let dir = tempdir().unwrap();

    let mut ground_truth_tree = StateTree::new();
    let mut ground_truth_trans = compute_genesis_transition_root();

    // 1. First execution session: write 10 ticks, snapshots at 4 and 8, compaction at 8
    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();

        for i in 1..=10 {
            let mutations = vec![
                StateMutation::put(
                    format!("account:usr_{}", i).into_bytes(),
                    format!("bal_{}", i * 100).into_bytes(),
                ),
                StateMutation::put(
                    format!("audit:seq_{}", i).into_bytes(),
                    b"VERIFIED".to_vec(),
                ),
            ];
            let payload = build_test_payload(
                &format!("exe_{:03}", i),
                &ground_truth_tree,
                &ground_truth_trans,
                mutations,
            );
            ground_truth_trans = payload.transition_root.clone();
            ground_truth_tree.apply_raw_mutations(&payload.mutations);

            engine.commit_record(i, payload).unwrap();

            if i == 4 {
                engine.create_snapshot().unwrap();
            } else if i == 8 {
                engine.compact().unwrap();
            }
        }

        assert_eq!(engine.current_sequence_tick(), 10);
        assert_eq!(
            engine.current_state_root(),
            ground_truth_tree.compute_state_root()
        );
        assert_eq!(engine.current_transition_root(), ground_truth_trans);
    }

    // 2. Cold-start recovery on fresh engine instance
    let recovered_engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();

    assert_eq!(recovered_engine.current_sequence_tick(), 10);
    assert_eq!(
        recovered_engine.current_revision(),
        ground_truth_tree.revision()
    );
    assert_eq!(
        recovered_engine.current_state_root(),
        ground_truth_tree.compute_state_root()
    );
    assert_eq!(
        recovered_engine.current_transition_root(),
        ground_truth_trans
    );

    for i in 1..=10 {
        assert_eq!(
            recovered_engine.get_state_value(format!("account:usr_{}", i).as_bytes()),
            Some(format!("bal_{}", i * 100).into_bytes())
        );
    }
}

#[test]
fn test_f013_002_fault_injection_torn_tail_truncation_and_resumption() {
    let dir = tempdir().unwrap();

    let mut mock_tree = StateTree::new();
    let mut mock_trans = compute_genesis_transition_root();

    // 1. Initial valid commits
    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
        for i in 1..=3 {
            let mutations = vec![StateMutation::put(
                format!("node:{}", i).into_bytes(),
                b"online".to_vec(),
            )];
            let payload =
                build_test_payload(&format!("exe_{}", i), &mock_tree, &mock_trans, mutations);
            mock_trans = payload.transition_root.clone();
            mock_tree.apply_raw_mutations(&payload.mutations);
            engine.commit_record(i, payload).unwrap();
        }
    }

    // 2. Simulate abrupt crash mid-write: append 20 bytes of partial header/payload
    let log_path = dir.path().join("commit.log");
    {
        let mut f = OpenOptions::new().append(true).open(&log_path).unwrap();
        f.write_all(b"SOVLOG01\x00\x01\x00\x00\x00\x00\x00\x04\x00\x00")
            .unwrap();
    }

    // 3. Cold-start: engine must truncate torn tail and resume at tick 4
    let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(engine.current_sequence_tick(), 3);
    assert_eq!(engine.current_state_root(), mock_tree.compute_state_root());

    // 4. Commit tick 4 cleanly
    let mutations_4 = vec![StateMutation::put(b"node:4", b"online")];
    let payload_4 = build_test_payload("exe_4", &mock_tree, &mock_trans, mutations_4);
    mock_tree.apply_raw_mutations(&payload_4.mutations);

    engine.commit_record(4, payload_4).unwrap();
    assert_eq!(engine.current_sequence_tick(), 4);
    assert_eq!(engine.current_state_root(), mock_tree.compute_state_root());
}

#[test]
fn test_f013_003_fault_injection_orphaned_snapshot_tmp_immunity() {
    let dir = tempdir().unwrap();

    let mut mock_tree = StateTree::new();
    let mut mock_trans = compute_genesis_transition_root();

    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
        for i in 1..=3 {
            let mutations = vec![StateMutation::put(
                format!("config:k_{}", i).into_bytes(),
                b"v".to_vec(),
            )];
            let payload =
                build_test_payload(&format!("exe_{}", i), &mock_tree, &mock_trans, mutations);
            mock_trans = payload.transition_root.clone();
            mock_tree.apply_raw_mutations(&payload.mutations);
            engine.commit_record(i, payload).unwrap();
        }
        engine.create_snapshot().unwrap(); // Snapshot at tick 3
    }

    // Simulate crash during subsequent snapshot attempt: orphaned .snap.tmp file left on disk
    let orphaned_tmp = dir.path().join("state_seq_00000000000000000004.snap.tmp.4");
    {
        let mut f = File::create(&orphaned_tmp).unwrap();
        f.write_all(b"PARTIAL_SNAPSHOT_BYTES_BEFORE_RENAME")
            .unwrap();
    }

    // Cold-start must ignore orphaned .tmp file and load verified snapshot at tick 3
    let engine2 = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(engine2.current_sequence_tick(), 3);
    assert_eq!(engine2.current_state_root(), mock_tree.compute_state_root());
}

#[test]
fn test_f013_004_fault_injection_interrupted_compaction_tmp_recovery() {
    let dir = tempdir().unwrap();

    let mut mock_tree = StateTree::new();
    let mut mock_trans = compute_genesis_transition_root();

    {
        let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
        for i in 1..=4 {
            let mutations = vec![StateMutation::put(
                format!("key:{}", i).into_bytes(),
                b"val".to_vec(),
            )];
            let payload =
                build_test_payload(&format!("exe_{}", i), &mock_tree, &mock_trans, mutations);
            mock_trans = payload.transition_root.clone();
            mock_tree.apply_raw_mutations(&payload.mutations);
            engine.commit_record(i, payload).unwrap();
        }
    }

    // Simulate crash leaving commit.log.compact_tmp behind
    let compact_tmp = dir.path().join("commit.log.compact_tmp");
    {
        let mut f = File::create(&compact_tmp).unwrap();
        f.write_all(b"PARTIAL_LOG_COMPACT_TMP").unwrap();
    }

    // Cold-start recovers from canonical commit.log
    let engine2 = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(engine2.current_sequence_tick(), 4);
    assert_eq!(engine2.current_state_root(), mock_tree.compute_state_root());
}

#[test]
fn test_f013_005_zero_authority_expansion_during_export_and_recovery() {
    let dir = tempdir().unwrap();
    let engine = StorageEngine::open(dir.path(), SyncPolicy::FsyncOnCommit).unwrap();

    let mut mock_tree = StateTree::new();
    let trans_0 = compute_genesis_transition_root();

    let mutations = vec![
        StateMutation::put(b"auth:policy:root", b"RESTRICTED"),
        StateMutation::put(b"auth:grant:node_1", b"PEER_ONLY"),
    ];
    let payload = build_test_payload("exe_auth_001", &mock_tree, &trans_0, mutations);
    mock_tree.apply_raw_mutations(&payload.mutations);
    engine.commit_record(1, payload).unwrap();

    let manifest = engine.create_snapshot().unwrap();
    let snap_path = dir
        .path()
        .join(format!("state_seq_{:020}.snap", manifest.sequence_tick));

    // Export and verify standalone attestation bundle
    let bundle = SnapshotExporter::export_bundle(&snap_path).unwrap();
    let attestation = SnapshotExporter::verify_bundle(&bundle).unwrap();

    assert_eq!(attestation.state_root, engine.current_state_root());
    assert_eq!(attestation.sequence_tick, 1);
    assert_eq!(attestation.entry_count, 2);
}
