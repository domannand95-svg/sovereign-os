//! ADAM-014-A: Canonical Binary Wire Framing & Integrity Verification
//!
//! Enforces fixed-width, big-endian binary frame layouts with domain-separated
//! BLAKE3 integrity checksums and strict payload bounds.

use std::io::{Read, Write};

pub const WIRE_MAGIC: &[u8; 8] = b"SOVWIRE1";
pub const WIRE_FORMAT_VERSION_V1: u16 = 1;
pub const WIRE_FRAME_DOMAIN_TAG: &str = "SOVEREIGN_OS:WIRE_FRAME:v1";
pub const DEFAULT_MAX_WIRE_PAYLOAD_BYTES: u64 = 64 * 1024 * 1024; // 64 MB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WireMessageType {
    Handshake = 1,
    SyncRequest = 2,
    SyncResponse = 3,
    CommitFrame = 4,
    SnapshotBundle = 5,
    Reject = 6,
}

impl WireMessageType {
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::Handshake),
            2 => Some(Self::SyncRequest),
            3 => Some(Self::SyncResponse),
            4 => Some(Self::CommitFrame),
            5 => Some(Self::SnapshotBundle),
            6 => Some(Self::Reject),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    InvalidMagic([u8; 8]),
    UnsupportedVersion(u16),
    UnknownMessageType(u8),
    PayloadLengthExceeded {
        length: u64,
        max: u64,
    },
    IntegrityChecksumMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    IoError(String),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic(m) => write!(f, "Invalid wire magic: {:?}", m),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported wire format version: {}", v),
            Self::UnknownMessageType(t) => write!(f, "Unknown wire message type: {}", t),
            Self::PayloadLengthExceeded { length, max } => {
                write!(
                    f,
                    "Wire payload length {} exceeds maximum bound {}",
                    length, max
                )
            }
            Self::IntegrityChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Wire frame integrity mismatch: expected {:?}, found {:?}",
                    expected, actual
                )
            }
            Self::IoError(e) => write!(f, "Wire I/O error: {}", e),
        }
    }
}

impl std::error::Error for WireError {}

/// Canonical wire frame structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireFrame {
    pub msg_type: WireMessageType,
    pub sequence_tick: u64,
    pub payload: Vec<u8>,
    pub checksum: [u8; 32],
}

impl WireFrame {
    pub const HEADER_SIZE: usize = 8 + 2 + 1 + 8 + 8; // magic(8) + ver(2) + type(1) + seq(8) + len(8) = 27 bytes
    pub const CHECKSUM_SIZE: usize = 32;

    /// Computes the domain-separated BLAKE3 integrity checksum over header and payload.
    pub fn compute_checksum(
        msg_type: WireMessageType,
        sequence_tick: u64,
        payload: &[u8],
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        // 1. Domain tag
        hasher.update(&(WIRE_FRAME_DOMAIN_TAG.len() as u64).to_be_bytes());
        hasher.update(WIRE_FRAME_DOMAIN_TAG.as_bytes());

        // 2. LP Header
        let mut header_buf = [0u8; Self::HEADER_SIZE];
        header_buf[0..8].copy_from_slice(WIRE_MAGIC);
        header_buf[8..10].copy_from_slice(&WIRE_FORMAT_VERSION_V1.to_be_bytes());
        header_buf[10] = msg_type as u8;
        header_buf[11..19].copy_from_slice(&sequence_tick.to_be_bytes());
        header_buf[19..27].copy_from_slice(&(payload.len() as u64).to_be_bytes());

        hasher.update(&(Self::HEADER_SIZE as u64).to_be_bytes());
        hasher.update(&header_buf);

        // 3. LP Payload
        hasher.update(&(payload.len() as u64).to_be_bytes());
        hasher.update(payload);

        *hasher.finalize().as_bytes()
    }

    /// Constructs a new verified wire frame.
    pub fn new(msg_type: WireMessageType, sequence_tick: u64, payload: Vec<u8>) -> Self {
        let checksum = Self::compute_checksum(msg_type, sequence_tick, &payload);
        Self {
            msg_type,
            sequence_tick,
            payload,
            checksum,
        }
    }

    /// Serializes wire frame to stream.
    pub fn write_to(&self, mut writer: impl Write) -> Result<usize, WireError> {
        let payload_len = self.payload.len() as u64;

        writer
            .write_all(WIRE_MAGIC)
            .map_err(|e| WireError::IoError(e.to_string()))?;
        writer
            .write_all(&WIRE_FORMAT_VERSION_V1.to_be_bytes())
            .map_err(|e| WireError::IoError(e.to_string()))?;
        writer
            .write_all(&[self.msg_type as u8])
            .map_err(|e| WireError::IoError(e.to_string()))?;
        writer
            .write_all(&self.sequence_tick.to_be_bytes())
            .map_err(|e| WireError::IoError(e.to_string()))?;
        writer
            .write_all(&payload_len.to_be_bytes())
            .map_err(|e| WireError::IoError(e.to_string()))?;
        writer
            .write_all(&self.payload)
            .map_err(|e| WireError::IoError(e.to_string()))?;
        writer
            .write_all(&self.checksum)
            .map_err(|e| WireError::IoError(e.to_string()))?;

        Ok(Self::HEADER_SIZE + self.payload.len() + Self::CHECKSUM_SIZE)
    }

    /// Reads and verifies a single canonical wire frame from stream.
    pub fn read_from(mut reader: impl Read, max_payload_bytes: u64) -> Result<Self, WireError> {
        let mut magic = [0u8; 8];
        reader
            .read_exact(&mut magic)
            .map_err(|e| WireError::IoError(e.to_string()))?;
        if &magic != WIRE_MAGIC {
            return Err(WireError::InvalidMagic(magic));
        }

        let mut ver_buf = [0u8; 2];
        reader
            .read_exact(&mut ver_buf)
            .map_err(|e| WireError::IoError(e.to_string()))?;
        let version = u16::from_be_bytes(ver_buf);
        if version != WIRE_FORMAT_VERSION_V1 {
            return Err(WireError::UnsupportedVersion(version));
        }

        let mut type_buf = [0u8; 1];
        reader
            .read_exact(&mut type_buf)
            .map_err(|e| WireError::IoError(e.to_string()))?;
        let msg_type = WireMessageType::from_u8(type_buf[0])
            .ok_or(WireError::UnknownMessageType(type_buf[0]))?;

        let mut seq_buf = [0u8; 8];
        reader
            .read_exact(&mut seq_buf)
            .map_err(|e| WireError::IoError(e.to_string()))?;
        let sequence_tick = u64::from_be_bytes(seq_buf);

        let mut len_buf = [0u8; 8];
        reader
            .read_exact(&mut len_buf)
            .map_err(|e| WireError::IoError(e.to_string()))?;
        let payload_len = u64::from_be_bytes(len_buf);

        if payload_len > max_payload_bytes {
            return Err(WireError::PayloadLengthExceeded {
                length: payload_len,
                max: max_payload_bytes,
            });
        }

        let mut payload = vec![0u8; payload_len as usize];
        reader
            .read_exact(&mut payload)
            .map_err(|e| WireError::IoError(e.to_string()))?;

        let mut checksum = [0u8; 32];
        reader
            .read_exact(&mut checksum)
            .map_err(|e| WireError::IoError(e.to_string()))?;

        let expected_checksum = Self::compute_checksum(msg_type, sequence_tick, &payload);
        if checksum != expected_checksum {
            return Err(WireError::IntegrityChecksumMismatch {
                expected: expected_checksum,
                actual: checksum,
            });
        }

        Ok(Self {
            msg_type,
            sequence_tick,
            payload,
            checksum,
        })
    }
}
