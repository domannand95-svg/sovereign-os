//! Boundary Tests for ADAM-014-A
//!
//! Validates canonical wire framing, domain-separated BLAKE3 checksums,
//! type tags, and bounded payload rejection (W014-001..W014-005).

use beta001_harness::network::{
    WireError, WireFrame, WireMessageType, DEFAULT_MAX_WIRE_PAYLOAD_BYTES, WIRE_MAGIC,
};

#[test]
fn test_w014_001_canonical_wire_frame_roundtrip() {
    let payload = b"{\"action\":\"sync_request\",\"target_tick\":10}".to_vec();
    let frame = WireFrame::new(WireMessageType::SyncRequest, 10, payload.clone());

    let mut buf = Vec::new();
    let written = frame.write_to(&mut buf).unwrap();
    assert_eq!(written, buf.len());
    assert_eq!(&buf[0..8], WIRE_MAGIC);

    let decoded = WireFrame::read_from(buf.as_slice(), DEFAULT_MAX_WIRE_PAYLOAD_BYTES).unwrap();
    assert_eq!(decoded, frame);
    assert_eq!(decoded.msg_type, WireMessageType::SyncRequest);
    assert_eq!(decoded.sequence_tick, 10);
    assert_eq!(decoded.payload, payload);
}

#[test]
fn test_w014_002_corrupted_payload_checksum_fails_closed() {
    let payload = b"state_transition_receipt_bytes".to_vec();
    let frame = WireFrame::new(WireMessageType::CommitFrame, 1, payload);

    let mut buf = Vec::new();
    frame.write_to(&mut buf).unwrap();

    // Corrupt payload byte
    buf[WireFrame::HEADER_SIZE + 2] ^= 0xEE;

    let res = WireFrame::read_from(buf.as_slice(), DEFAULT_MAX_WIRE_PAYLOAD_BYTES);
    assert!(matches!(
        res,
        Err(WireError::IntegrityChecksumMismatch { .. })
    ));
}

#[test]
fn test_w014_003_invalid_magic_fails_closed() {
    let payload = b"handshake_payload".to_vec();
    let frame = WireFrame::new(WireMessageType::Handshake, 0, payload);

    let mut buf = Vec::new();
    frame.write_to(&mut buf).unwrap();

    buf[0] = b'B';
    buf[1] = b'A';
    buf[2] = b'D';

    let res = WireFrame::read_from(buf.as_slice(), DEFAULT_MAX_WIRE_PAYLOAD_BYTES);
    assert!(matches!(res, Err(WireError::InvalidMagic(_))));
}

#[test]
fn test_w014_004_unknown_message_type_fails_closed() {
    let payload = b"payload".to_vec();
    let frame = WireFrame::new(WireMessageType::Handshake, 0, payload);

    let mut buf = Vec::new();
    frame.write_to(&mut buf).unwrap();

    // Overwrite message type byte with unknown value 99
    buf[10] = 99;

    let res = WireFrame::read_from(buf.as_slice(), DEFAULT_MAX_WIRE_PAYLOAD_BYTES);
    assert!(matches!(res, Err(WireError::UnknownMessageType(99))));
}

#[test]
fn test_w014_005_payload_exceeding_max_limit_fails_closed() {
    let payload = vec![0u8; 1024];
    let frame = WireFrame::new(WireMessageType::SnapshotBundle, 5, payload);

    let mut buf = Vec::new();
    frame.write_to(&mut buf).unwrap();

    // Set max bound to 512 bytes
    let res = WireFrame::read_from(buf.as_slice(), 512);
    assert!(matches!(
        res,
        Err(WireError::PayloadLengthExceeded {
            length: 1024,
            max: 512
        })
    ));
}
