//! Boundary Tests for ADAM-014-B
//!
//! Validates peer handshake serialization, cluster identity gating,
//! version negotiation, and frontier exchange (H014-001..H014-005).

use beta001_harness::network::{
    HandshakeController, HandshakeError, HandshakePayload, WireFrame, WireMessageType,
    DEFAULT_MAX_WIRE_PAYLOAD_BYTES, HANDSHAKE_PROTOCOL_VERSION_V1,
};

#[test]
fn test_h014_001_handshake_payload_canonical_roundtrip() {
    let payload = HandshakePayload {
        node_id: "node_alpha_01".to_string(),
        cluster_id: "sovereign_prime".to_string(),
        protocol_version: HANDSHAKE_PROTOCOL_VERSION_V1,
        sequence_tick: 42,
        state_root: "sr_canonical_alpha".to_string(),
        transition_root: "tr_canonical_alpha".to_string(),
    };

    let encoded = payload.encode_canonical();
    let decoded = HandshakePayload::decode_canonical(encoded.as_slice()).unwrap();

    assert_eq!(decoded, payload);
}

#[test]
fn test_h014_002_handshake_over_wire_frame_successful_verification() {
    let payload = HandshakePayload {
        node_id: "node_beta_02".to_string(),
        cluster_id: "sovereign_cluster_7".to_string(),
        protocol_version: HANDSHAKE_PROTOCOL_VERSION_V1,
        sequence_tick: 100,
        state_root: "sr_beta_100".to_string(),
        transition_root: "tr_beta_100".to_string(),
    };

    // Encapsulate in WireFrame
    let wire_frame = WireFrame::new(WireMessageType::Handshake, 0, payload.encode_canonical());

    let mut stream = Vec::new();
    wire_frame.write_to(&mut stream).unwrap();

    // Receiver decodes wire frame and verifies handshake
    let decoded_frame =
        WireFrame::read_from(stream.as_slice(), DEFAULT_MAX_WIRE_PAYLOAD_BYTES).unwrap();
    assert_eq!(decoded_frame.msg_type, WireMessageType::Handshake);

    let decoded_payload =
        HandshakePayload::decode_canonical(decoded_frame.payload.as_slice()).unwrap();
    let session =
        HandshakeController::verify_incoming("sovereign_cluster_7", &decoded_payload).unwrap();

    assert_eq!(session.peer_node_id, "node_beta_02");
    assert_eq!(session.cluster_id, "sovereign_cluster_7");
    assert_eq!(session.peer_sequence_tick, 100);
    assert_eq!(session.peer_state_root, "sr_beta_100");
}

#[test]
fn test_h014_003_cluster_mismatch_rejects_immediately() {
    let payload = HandshakePayload {
        node_id: "foreign_node".to_string(),
        cluster_id: "alien_cluster".to_string(),
        protocol_version: HANDSHAKE_PROTOCOL_VERSION_V1,
        sequence_tick: 1,
        state_root: "sr_0".to_string(),
        transition_root: "tr_0".to_string(),
    };

    let res = HandshakeController::verify_incoming("sovereign_cluster_primary", &payload);
    assert!(matches!(res, Err(HandshakeError::ClusterMismatch { .. })));
}

#[test]
fn test_h014_004_unsupported_version_rejects() {
    let payload = HandshakePayload {
        node_id: "legacy_node".to_string(),
        cluster_id: "sovereign_cluster_primary".to_string(),
        protocol_version: 999,
        sequence_tick: 1,
        state_root: "sr_0".to_string(),
        transition_root: "tr_0".to_string(),
    };

    let res = HandshakeController::verify_incoming("sovereign_cluster_primary", &payload);
    assert!(matches!(
        res,
        Err(HandshakeError::UnsupportedProtocolVersion { .. })
    ));
}

#[test]
fn test_h014_005_empty_node_id_rejects() {
    let payload = HandshakePayload {
        node_id: "   ".to_string(),
        cluster_id: "sovereign_cluster_primary".to_string(),
        protocol_version: HANDSHAKE_PROTOCOL_VERSION_V1,
        sequence_tick: 1,
        state_root: "sr_0".to_string(),
        transition_root: "tr_0".to_string(),
    };

    let res = HandshakeController::verify_incoming("sovereign_cluster_primary", &payload);
    assert!(matches!(res, Err(HandshakeError::InvalidNodeId(_))));
}
