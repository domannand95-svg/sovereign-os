//! ADAM-014-B: Peer Handshake & Node Identity Binding
//!
//! Enforces cluster domain verification, protocol version alignment,
//! and initial frontier exchange prior to state stream replication.

use std::io::{Read, Write};

pub const HANDSHAKE_PROTOCOL_VERSION_V1: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeError {
    ClusterMismatch { expected: String, received: String },
    UnsupportedProtocolVersion { expected: u16, received: u16 },
    InvalidNodeId(String),
    MalformedPayload(String),
    IoError(String),
}

impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClusterMismatch { expected, received } => {
                write!(
                    f,
                    "Cluster ID mismatch: expected '{}', received '{}'",
                    expected, received
                )
            }
            Self::UnsupportedProtocolVersion { expected, received } => {
                write!(
                    f,
                    "Unsupported protocol version: expected {}, received {}",
                    expected, received
                )
            }
            Self::InvalidNodeId(e) => write!(f, "Invalid node ID: {}", e),
            Self::MalformedPayload(e) => write!(f, "Malformed handshake payload: {}", e),
            Self::IoError(e) => write!(f, "Handshake I/O error: {}", e),
        }
    }
}

impl std::error::Error for HandshakeError {}

/// Canonical handshake payload exchanged during session initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakePayload {
    pub node_id: String,
    pub cluster_id: String,
    pub protocol_version: u16,
    pub sequence_tick: u64,
    pub state_root: String,
    pub transition_root: String,
}

impl HandshakePayload {
    /// Serializes handshake payload into canonical LP binary format.
    pub fn encode_canonical(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        fn write_lp_string(buf: &mut Vec<u8>, s: &str) {
            let bytes = s.as_bytes();
            buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            buf.extend_from_slice(bytes);
        }

        write_lp_string(&mut buf, &self.node_id);
        write_lp_string(&mut buf, &self.cluster_id);
        buf.extend_from_slice(&self.protocol_version.to_be_bytes());
        buf.extend_from_slice(&self.sequence_tick.to_be_bytes());
        write_lp_string(&mut buf, &self.state_root);
        write_lp_string(&mut buf, &self.transition_root);

        buf
    }

    /// Deserializes handshake payload from canonical LP binary format.
    pub fn decode_canonical(mut reader: impl Read) -> Result<Self, HandshakeError> {
        fn read_lp_string(r: &mut impl Read) -> Result<String, HandshakeError> {
            let mut len_buf = [0u8; 4];
            r.read_exact(&mut len_buf)
                .map_err(|e| HandshakeError::IoError(e.to_string()))?;
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0u8; len];
            r.read_exact(&mut buf)
                .map_err(|e| HandshakeError::IoError(e.to_string()))?;
            String::from_utf8(buf).map_err(|e| HandshakeError::MalformedPayload(e.to_string()))
        }

        let node_id = read_lp_string(&mut reader)?;
        let cluster_id = read_lp_string(&mut reader)?;

        let mut ver_buf = [0u8; 2];
        reader
            .read_exact(&mut ver_buf)
            .map_err(|e| HandshakeError::IoError(e.to_string()))?;
        let protocol_version = u16::from_be_bytes(ver_buf);

        let mut seq_buf = [0u8; 8];
        reader
            .read_exact(&mut seq_buf)
            .map_err(|e| HandshakeError::IoError(e.to_string()))?;
        let sequence_tick = u64::from_be_bytes(seq_buf);

        let state_root = read_lp_string(&mut reader)?;
        let transition_root = read_lp_string(&mut reader)?;

        Ok(Self {
            node_id,
            cluster_id,
            protocol_version,
            sequence_tick,
            state_root,
            transition_root,
        })
    }
}

/// Established peer session context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeSession {
    pub peer_node_id: String,
    pub cluster_id: String,
    pub peer_sequence_tick: u64,
    pub peer_state_root: String,
    pub peer_transition_root: String,
}

pub struct HandshakeController;

impl HandshakeController {
    /// Validates an incoming peer handshake against local cluster boundaries and protocol rules.
    pub fn verify_incoming(
        local_cluster_id: &str,
        payload: &HandshakePayload,
    ) -> Result<HandshakeSession, HandshakeError> {
        if payload.node_id.trim().is_empty() {
            return Err(HandshakeError::InvalidNodeId(
                "Node ID cannot be empty".to_string(),
            ));
        }

        if payload.cluster_id != local_cluster_id {
            return Err(HandshakeError::ClusterMismatch {
                expected: local_cluster_id.to_string(),
                received: payload.cluster_id.clone(),
            });
        }

        if payload.protocol_version != HANDSHAKE_PROTOCOL_VERSION_V1 {
            return Err(HandshakeError::UnsupportedProtocolVersion {
                expected: HANDSHAKE_PROTOCOL_VERSION_V1,
                received: payload.protocol_version,
            });
        }

        Ok(HandshakeSession {
            peer_node_id: payload.node_id.clone(),
            cluster_id: payload.cluster_id.clone(),
            peer_sequence_tick: payload.sequence_tick,
            peer_state_root: payload.state_root.clone(),
            peer_transition_root: payload.transition_root.clone(),
        })
    }
}
