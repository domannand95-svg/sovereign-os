//! Boundary Tests for ADAM-013-A
//!
//! Validates commit log framing, domain-separated BLAKE3 checksums,
//! sync policies, and bit-level canonical encoding (A013-001..A013-006).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use tempfile::tempdir;

use beta001_harness::service_contract::{ExecutionId, Sha256Digest};
use beta001_harness::state::StateMutation;
use beta001_harness::storage::{
    CommitLogFrame, CommitLogWriter, CommitRecordPayload, DurabilityAcknowledgement, FrameError,
    SyncPolicy, COMMIT_LOG_MAGIC,
};

fn create_sample_payload(pre_rev: u64, post_rev: u64) -> CommitRecordPayload {
    CommitRecordPayload {
        execution_id: ExecutionId::new("exe_persistence_001".to_string()).unwrap(),
        package_digest: Sha256Digest::new(
            "d3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        )
        .unwrap(),
        pre_revision: pre_rev,
        post_revision: post_rev,
        pre_state_root: "root_alpha_0".to_string(),
        post_state_root: "root_beta_1".to_string(),
        prior_transition_root: "trans_root_0".to_string(),
        transition_root: "trans_root_1".to_string(),
        mutations: vec![
            StateMutation::put(b"config:cluster_name", b"sovereign_prime"),
            StateMutation::delete(b"stale:key"),
        ],
    }
}

#[test]
fn test_a013_001_canonical_binary_frame_roundtrip() {
    let payload = create_sample_payload(0, 1);
    let payload_bytes = payload.encode_canonical();
    let checksum = CommitLogFrame::compute_checksum(1, &payload_bytes);

    let frame = CommitLogFrame {
        sequence_tick: 1,
        payload: payload.clone(),
        checksum,
    };

    let mut encoded = Vec::new();
    let written = frame.write_to(&mut encoded).unwrap();
    assert_eq!(written, encoded.len());

    // Assert Magic
    assert_eq!(&encoded[0..8], COMMIT_LOG_MAGIC);

    // Read back
    let decoded = CommitLogFrame::read_from(encoded.as_slice(), 1024 * 1024).unwrap();
    assert_eq!(decoded, frame);
    assert_eq!(decoded.payload.mutations.len(), 2);
}

#[test]
fn test_a013_002_corrupted_payload_checksum_fails_closed() {
    let payload = create_sample_payload(0, 1);
    let payload_bytes = payload.encode_canonical();
    let checksum = CommitLogFrame::compute_checksum(1, &payload_bytes);

    let frame = CommitLogFrame {
        sequence_tick: 1,
        payload,
        checksum,
    };

    let mut encoded = Vec::new();
    frame.write_to(&mut encoded).unwrap();

    // Corrupt one byte in the payload area
    let corrupt_idx = CommitLogFrame::HEADER_SIZE + 4;
    encoded[corrupt_idx] ^= 0xFF;

    let res = CommitLogFrame::read_from(encoded.as_slice(), 1024 * 1024);
    assert!(matches!(res, Err(FrameError::ChecksumMismatch { .. })));
}

#[test]
fn test_a013_003_invalid_magic_fails_closed() {
    let payload = create_sample_payload(0, 1);
    let payload_bytes = payload.encode_canonical();
    let checksum = CommitLogFrame::compute_checksum(1, &payload_bytes);

    let frame = CommitLogFrame {
        sequence_tick: 1,
        payload,
        checksum,
    };

    let mut encoded = Vec::new();
    frame.write_to(&mut encoded).unwrap();

    // Corrupt magic
    encoded[0] = b'X';

    let res = CommitLogFrame::read_from(encoded.as_slice(), 1024 * 1024);
    assert!(matches!(res, Err(FrameError::InvalidMagic(_))));
}

#[test]
fn test_a013_004_durability_frontier_fsync_on_commit() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("commit.log");

    let writer = CommitLogWriter::open(&log_path, SyncPolicy::FsyncOnCommit).unwrap();
    let payload = create_sample_payload(0, 1);

    let ack = writer.append(1, payload.clone()).unwrap();
    assert!(matches!(
        ack,
        DurabilityAcknowledgement::DurableCommitted {
            sequence_tick: 1,
            ..
        }
    ));

    // Read directly from disk
    let mut file = File::open(&log_path).unwrap();
    let frame = CommitLogFrame::read_from(&mut file, 1024 * 1024).unwrap();
    assert_eq!(frame.sequence_tick, 1);
    assert_eq!(frame.payload, payload);
}

#[test]
fn test_a013_005_durability_frontier_periodic_sync_requires_explicit_sync() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("commit_periodic.log");

    let writer = CommitLogWriter::open(&log_path, SyncPolicy::FsyncPeriodic).unwrap();
    let payload = create_sample_payload(0, 1);

    let ack = writer.append(10, payload).unwrap();
    assert!(matches!(
        ack,
        DurabilityAcknowledgement::CommittedInMemory {
            sequence_tick: 10,
            ..
        }
    ));

    // Explicit fsync completes durability barrier
    assert!(writer.sync().is_ok());
}

#[test]
fn test_a013_006_sequential_multi_frame_log_append() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("multi_frame.log");

    let writer = CommitLogWriter::open(&log_path, SyncPolicy::FsyncOnCommit).unwrap();

    for tick in 1..=5 {
        let payload = create_sample_payload(tick - 1, tick);
        writer.append(tick, payload).unwrap();
    }

    let mut file = File::open(&log_path).unwrap();
    for tick in 1..=5 {
        let frame = CommitLogFrame::read_from(&mut file, 1024 * 1024).unwrap();
        assert_eq!(frame.sequence_tick, tick);
        assert_eq!(frame.payload.pre_revision, tick - 1);
        assert_eq!(frame.payload.post_revision, tick);
    }
}
