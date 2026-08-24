//! ADAM-013-A: Canonical Binary Commit Log Frame Encoding & Checksumming
//!
//! Enforces fixed-width, big-endian binary layouts with domain-separated BLAKE3 hashing.

use crate::service_contract::{ExecutionId, Sha256Digest};
use crate::state::StateMutation;
use std::io::{Read, Write};

pub const COMMIT_LOG_MAGIC: &[u8; 8] = b"SOVLOG01";
pub const COMMIT_LOG_FORMAT_VERSION_V1: u16 = 1;
pub const COMMIT_LOG_FRAME_DOMAIN_TAG: &str = "SOVEREIGN_OS:COMMIT_LOG_FRAME:v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    InvalidMagic([u8; 8]),
    UnsupportedVersion(u16),
    PayloadLengthExceeded {
        length: u64,
        max: u64,
    },
    ChecksumMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    IoError(String),
    MalformedPayload(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic(m) => write!(f, "Invalid magic bytes: {:?}", m),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported format version: {}", v),
            Self::PayloadLengthExceeded { length, max } => {
                write!(f, "Payload length {} exceeds limit {}", length, max)
            }
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Checksum mismatch: expected {:?}, actual {:?}",
                    expected, actual
                )
            }
            Self::IoError(e) => write!(f, "Frame I/O error: {}", e),
            Self::MalformedPayload(e) => write!(f, "Malformed payload: {}", e),
        }
    }
}

impl std::error::Error for FrameError {}

/// Semantic payload recording complete execution, lineage, and mutation deltas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRecordPayload {
    pub execution_id: ExecutionId,
    pub package_digest: Sha256Digest,
    pub pre_revision: u64,
    pub post_revision: u64,
    pub pre_state_root: String,
    pub post_state_root: String,
    pub prior_transition_root: String,
    pub transition_root: String,
    pub mutations: Vec<StateMutation>,
}

impl CommitRecordPayload {
    /// Serializes payload into canonical, platform-independent binary format.
    pub fn encode_canonical(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // 1. execution_id (LP string)
        let exec_bytes = self.execution_id.as_str().as_bytes();
        buf.extend_from_slice(&(exec_bytes.len() as u32).to_be_bytes());
        buf.extend_from_slice(exec_bytes);

        // 2. package_digest (LP string)
        let pkg_bytes = self.package_digest.as_str().as_bytes();
        buf.extend_from_slice(&(pkg_bytes.len() as u32).to_be_bytes());
        buf.extend_from_slice(pkg_bytes);

        // 3. Revisions (u64 BE)
        buf.extend_from_slice(&self.pre_revision.to_be_bytes());
        buf.extend_from_slice(&self.post_revision.to_be_bytes());

        // 4. Roots (LP strings)
        let roots = [
            &self.pre_state_root,
            &self.post_state_root,
            &self.prior_transition_root,
            &self.transition_root,
        ];
        for root in roots {
            let r_bytes = root.as_bytes();
            buf.extend_from_slice(&(r_bytes.len() as u32).to_be_bytes());
            buf.extend_from_slice(r_bytes);
        }

        // 5. Mutations
        buf.extend_from_slice(&(self.mutations.len() as u32).to_be_bytes());
        for m in &self.mutations {
            match m {
                StateMutation::Put { key, value } => {
                    buf.extend_from_slice(&(key.len() as u32).to_be_bytes());
                    buf.extend_from_slice(key);
                    buf.push(1u8); // Put operation tag
                    buf.extend_from_slice(&(value.len() as u32).to_be_bytes());
                    buf.extend_from_slice(value);
                }
                StateMutation::Delete { key } => {
                    buf.extend_from_slice(&(key.len() as u32).to_be_bytes());
                    buf.extend_from_slice(key);
                    buf.push(0u8); // Delete operation tag
                }
            }
        }

        buf
    }

    /// Deserializes payload from canonical binary format.
    pub fn decode_canonical(mut reader: impl Read) -> Result<Self, FrameError> {
        fn read_u32(r: &mut impl Read) -> Result<u32, FrameError> {
            let mut buf = [0u8; 4];
            r.read_exact(&mut buf)
                .map_err(|e| FrameError::IoError(e.to_string()))?;
            Ok(u32::from_be_bytes(buf))
        }

        fn read_u64(r: &mut impl Read) -> Result<u64, FrameError> {
            let mut buf = [0u8; 8];
            r.read_exact(&mut buf)
                .map_err(|e| FrameError::IoError(e.to_string()))?;
            Ok(u64::from_be_bytes(buf))
        }

        fn read_string(r: &mut impl Read) -> Result<String, FrameError> {
            let len = read_u32(r)? as usize;
            let mut buf = vec![0u8; len];
            r.read_exact(&mut buf)
                .map_err(|e| FrameError::IoError(e.to_string()))?;
            String::from_utf8(buf).map_err(|e| FrameError::MalformedPayload(e.to_string()))
        }

        let exec_str = read_string(&mut reader)?;
        let exec_id =
            ExecutionId::new(exec_str).map_err(|e| FrameError::MalformedPayload(e.to_string()))?;

        let pkg_str = read_string(&mut reader)?;
        let pkg_digest =
            Sha256Digest::new(pkg_str).map_err(|e| FrameError::MalformedPayload(e.to_string()))?;

        let pre_rev = read_u64(&mut reader)?;
        let post_rev = read_u64(&mut reader)?;

        let pre_state_root = read_string(&mut reader)?;
        let post_state_root = read_string(&mut reader)?;
        let prior_transition_root = read_string(&mut reader)?;
        let transition_root = read_string(&mut reader)?;

        let mutation_count = read_u32(&mut reader)? as usize;
        let mut mutations = Vec::with_capacity(mutation_count);

        for _ in 0..mutation_count {
            let k_len = read_u32(&mut reader)? as usize;
            let mut k_buf = vec![0u8; k_len];
            reader
                .read_exact(&mut k_buf)
                .map_err(|e| FrameError::IoError(e.to_string()))?;

            let mut op_byte = [0u8; 1];
            reader
                .read_exact(&mut op_byte)
                .map_err(|e| FrameError::IoError(e.to_string()))?;

            if op_byte[0] == 1 {
                let v_len = read_u32(&mut reader)? as usize;
                let mut v_buf = vec![0u8; v_len];
                reader
                    .read_exact(&mut v_buf)
                    .map_err(|e| FrameError::IoError(e.to_string()))?;
                mutations.push(StateMutation::put(k_buf, v_buf));
            } else if op_byte[0] == 0 {
                mutations.push(StateMutation::delete(k_buf));
            } else {
                return Err(FrameError::MalformedPayload(format!(
                    "Invalid op byte: {}",
                    op_byte[0]
                )));
            }
        }

        Ok(Self {
            execution_id: exec_id,
            package_digest: pkg_digest,
            pre_revision: pre_rev,
            post_revision: post_rev,
            pre_state_root,
            post_state_root,
            prior_transition_root,
            transition_root,
            mutations,
        })
    }
}

/// Binary commit log frame header and trailer verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitLogFrame {
    pub sequence_tick: u64,
    pub payload: CommitRecordPayload,
    pub checksum: [u8; 32],
}

impl CommitLogFrame {
    pub const HEADER_SIZE: usize = 8 + 2 + 8 + 8; // magic(8) + ver(2) + seq(8) + len(8) = 26 bytes
    pub const CHECKSUM_SIZE: usize = 32;

    /// Computes the canonical BLAKE3 checksum for a given frame header and payload.
    pub fn compute_checksum(sequence_tick: u64, payload_bytes: &[u8]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        // 1. Domain tag
        hasher.update(&(COMMIT_LOG_FRAME_DOMAIN_TAG.len() as u64).to_be_bytes());
        hasher.update(COMMIT_LOG_FRAME_DOMAIN_TAG.as_bytes());

        // 2. LP Header
        let mut header_buf = [0u8; 26];
        header_buf[0..8].copy_from_slice(COMMIT_LOG_MAGIC);
        header_buf[8..10].copy_from_slice(&COMMIT_LOG_FORMAT_VERSION_V1.to_be_bytes());
        header_buf[10..18].copy_from_slice(&sequence_tick.to_be_bytes());
        header_buf[18..26].copy_from_slice(&(payload_bytes.len() as u64).to_be_bytes());

        hasher.update(&26u64.to_be_bytes());
        hasher.update(&header_buf);

        // 3. LP Payload
        hasher.update(&(payload_bytes.len() as u64).to_be_bytes());
        hasher.update(payload_bytes);

        *hasher.finalize().as_bytes()
    }

    /// Serializes a complete frame to byte stream.
    pub fn write_to(&self, mut writer: impl Write) -> Result<usize, FrameError> {
        let payload_bytes = self.payload.encode_canonical();
        let payload_len = payload_bytes.len() as u64;

        writer
            .write_all(COMMIT_LOG_MAGIC)
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        writer
            .write_all(&COMMIT_LOG_FORMAT_VERSION_V1.to_be_bytes())
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        writer
            .write_all(&self.sequence_tick.to_be_bytes())
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        writer
            .write_all(&payload_len.to_be_bytes())
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        writer
            .write_all(&payload_bytes)
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        writer
            .write_all(&self.checksum)
            .map_err(|e| FrameError::IoError(e.to_string()))?;

        Ok(Self::HEADER_SIZE + payload_bytes.len() + Self::CHECKSUM_SIZE)
    }

    /// Deserializes and cryptographically validates a single frame from byte stream.
    pub fn read_from(mut reader: impl Read, max_payload_len: u64) -> Result<Self, FrameError> {
        let mut magic = [0u8; 8];
        reader
            .read_exact(&mut magic)
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        if &magic != COMMIT_LOG_MAGIC {
            return Err(FrameError::InvalidMagic(magic));
        }

        let mut ver_buf = [0u8; 2];
        reader
            .read_exact(&mut ver_buf)
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        let version = u16::from_be_bytes(ver_buf);
        if version != COMMIT_LOG_FORMAT_VERSION_V1 {
            return Err(FrameError::UnsupportedVersion(version));
        }

        let mut seq_buf = [0u8; 8];
        reader
            .read_exact(&mut seq_buf)
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        let sequence_tick = u64::from_be_bytes(seq_buf);

        let mut len_buf = [0u8; 8];
        reader
            .read_exact(&mut len_buf)
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        let payload_len = u64::from_be_bytes(len_buf);

        if payload_len > max_payload_len {
            return Err(FrameError::PayloadLengthExceeded {
                length: payload_len,
                max: max_payload_len,
            });
        }

        let mut payload_bytes = vec![0u8; payload_len as usize];
        reader
            .read_exact(&mut payload_bytes)
            .map_err(|e| FrameError::IoError(e.to_string()))?;

        let mut checksum = [0u8; 32];
        reader
            .read_exact(&mut checksum)
            .map_err(|e| FrameError::IoError(e.to_string()))?;

        let expected_checksum = Self::compute_checksum(sequence_tick, &payload_bytes);
        if checksum != expected_checksum {
            return Err(FrameError::ChecksumMismatch {
                expected: expected_checksum,
                actual: checksum,
            });
        }

        let payload = CommitRecordPayload::decode_canonical(payload_bytes.as_slice())?;

        Ok(Self {
            sequence_tick,
            payload,
            checksum,
        })
    }
}
