//! Boundary Tests for ADAM-014-C, 014-D, 014-E
//!
//! Validates two-node linear state replication, replay shields, malformed frame rejection,
//! cold-start recovery convergence, and zero authority expansion (N014-001..N014-005).

use tempfile::tempdir;

use beta001_harness::network::{
    ReplicationError, StateReplicator, WireFrame, WireMessageType, DEFAULT_MAX_WIRE_PAYLOAD_BYTES,
};
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
fn test_n014_001_two_node_state_convergence_over_wire() {
    let dir_primary = tempdir().unwrap();
    let dir_replica = tempdir().unwrap();

    let primary_engine =
        StorageEngine::open(dir_primary.path(), SyncPolicy::FsyncOnCommit).unwrap();
    let replica_engine =
        StorageEngine::open(dir_replica.path(), SyncPolicy::FsyncOnCommit).unwrap();

    let mut staging_tree = StateTree::new();
    let mut current_trans_root = compute_genesis_transition_root();

    for i in 1..=5 {
        let mutations = vec![
            StateMutation::put(
                format!("key:{}", i).into_bytes(),
                format!("val:{}", i * 10).into_bytes(),
            ),
            StateMutation::put(format!("audit:{}", i).into_bytes(), b"CONFIRMED".to_vec()),
        ];
        let payload = build_payload(
            &format!("exe_{:03}", i),
            &staging_tree,
            &current_trans_root,
            mutations,
        );
        current_trans_root = payload.transition_root.clone();
        staging_tree.apply_raw_mutations(&payload.mutations);

        primary_engine.commit_record(i, payload.clone()).unwrap();

        let wire_frame = StateReplicator::create_commit_wire_frame(i, &payload);

        let mut wire_bytes = Vec::new();
        wire_frame.write_to(&mut wire_bytes).unwrap();
        let decoded_frame =
            WireFrame::read_from(wire_bytes.as_slice(), DEFAULT_MAX_WIRE_PAYLOAD_BYTES).unwrap();

        StateReplicator::ingest_replicated_frame(&replica_engine, &decoded_frame).unwrap();
    }

    assert_eq!(
        replica_engine.current_sequence_tick(),
        primary_engine.current_sequence_tick()
    );
    assert_eq!(
        replica_engine.current_revision(),
        primary_engine.current_revision()
    );
    assert_eq!(
        replica_engine.current_state_root(),
        primary_engine.current_state_root()
    );
    assert_eq!(
        replica_engine.current_transition_root(),
        primary_engine.current_transition_root()
    );

    for i in 1..=5 {
        assert_eq!(
            replica_engine.get_state_value(format!("key:{}", i).as_bytes()),
            primary_engine.get_state_value(format!("key:{}", i).as_bytes())
        );
    }
}

#[test]
fn test_n014_002_replay_shield_rejects_duplicate_or_historical_ticks() {
    let dir_primary = tempdir().unwrap();
    let dir_replica = tempdir().unwrap();

    let primary_engine =
        StorageEngine::open(dir_primary.path(), SyncPolicy::FsyncOnCommit).unwrap();
    let replica_engine =
        StorageEngine::open(dir_replica.path(), SyncPolicy::FsyncOnCommit).unwrap();

    let mut staging_tree = StateTree::new();
    let current_trans_root = compute_genesis_transition_root();

    let payload1 = build_payload(
        "exe_001",
        &staging_tree,
        &current_trans_root,
        vec![StateMutation::put(b"k1", b"v1")],
    );
    staging_tree.apply_raw_mutations(&payload1.mutations);
    primary_engine.commit_record(1, payload1.clone()).unwrap();

    let frame1 = StateReplicator::create_commit_wire_frame(1, &payload1);

    StateReplicator::ingest_replicated_frame(&replica_engine, &frame1).unwrap();
    assert_eq!(replica_engine.current_sequence_tick(), 1);

    let replay_res = StateReplicator::ingest_replicated_frame(&replica_engine, &frame1);
    assert!(matches!(
        replay_res,
        Err(ReplicationError::ReplayDetected {
            current_tick: 1,
            attempted_tick: 1
        })
    ));
}

#[test]
fn test_n014_003_sequence_gap_rejects_out_of_order_frames() {
    let dir_primary = tempdir().unwrap();
    let dir_replica = tempdir().unwrap();

    let primary_engine =
        StorageEngine::open(dir_primary.path(), SyncPolicy::FsyncOnCommit).unwrap();
    let replica_engine =
        StorageEngine::open(dir_replica.path(), SyncPolicy::FsyncOnCommit).unwrap();

    let staging_tree = StateTree::new();
    let current_trans_root = compute_genesis_transition_root();

    let payload = build_payload(
        "exe_001",
        &staging_tree,
        &current_trans_root,
        vec![StateMutation::put(b"k1", b"v1")],
    );
    primary_engine.commit_record(1, payload.clone()).unwrap();

    let frame_gap = StateReplicator::create_commit_wire_frame(3, &payload);

    let gap_res = StateReplicator::ingest_replicated_frame(&replica_engine, &frame_gap);
    assert!(matches!(
        gap_res,
        Err(ReplicationError::SequenceGapDetected {
            expected_tick: 1,
            received_tick: 3
        })
    ));
}

#[test]
fn test_n014_004_replica_cold_start_recovery_preserves_replicated_state() {
    let dir_primary = tempdir().unwrap();
    let dir_replica = tempdir().unwrap();

    let expected_state_root;
    let expected_trans_root;

    {
        let primary_engine =
            StorageEngine::open(dir_primary.path(), SyncPolicy::FsyncOnCommit).unwrap();
        let replica_engine =
            StorageEngine::open(dir_replica.path(), SyncPolicy::FsyncOnCommit).unwrap();

        let mut staging_tree = StateTree::new();
        let mut current_trans_root = compute_genesis_transition_root();

        for i in 1..=3 {
            let payload = build_payload(
                &format!("exe_{}", i),
                &staging_tree,
                &current_trans_root,
                vec![StateMutation::put(
                    format!("k{}", i).into_bytes(),
                    b"v".to_vec(),
                )],
            );
            current_trans_root = payload.transition_root.clone();
            staging_tree.apply_raw_mutations(&payload.mutations);

            primary_engine.commit_record(i, payload.clone()).unwrap();
            let frame = StateReplicator::create_commit_wire_frame(i, &payload);
            StateReplicator::ingest_replicated_frame(&replica_engine, &frame).unwrap();
        }

        expected_state_root = replica_engine.current_state_root();
        expected_trans_root = replica_engine.current_transition_root();
    }

    let replica_reopened =
        StorageEngine::open(dir_replica.path(), SyncPolicy::FsyncOnCommit).unwrap();
    assert_eq!(replica_reopened.current_sequence_tick(), 3);
    assert_eq!(replica_reopened.current_state_root(), expected_state_root);
    assert_eq!(
        replica_reopened.current_transition_root(),
        expected_trans_root
    );
    assert_eq!(replica_reopened.get_state_value(b"k2"), Some(b"v".to_vec()));
}

#[test]
fn test_n014_005_invalid_message_type_fails_closed() {
    let dir_replica = tempdir().unwrap();
    let replica_engine =
        StorageEngine::open(dir_replica.path(), SyncPolicy::FsyncOnCommit).unwrap();

    let invalid_frame = WireFrame::new(WireMessageType::SyncRequest, 1, b"malformed".to_vec());
    let res = StateReplicator::ingest_replicated_frame(&replica_engine, &invalid_frame);
    assert!(matches!(
        res,
        Err(ReplicationError::InvalidMessageType(
            WireMessageType::SyncRequest
        ))
    ));
}
