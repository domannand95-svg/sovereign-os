//! ADAM-013-C: Canonical Binary State Snapshotting & Crash-Consistent Atomic Publication
//!
//! Provides deterministic binary snapshot serialization, domain-separated BLAKE3 checksums,
//! and power-loss-consistent publication via atomic rename and directory synchronization.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::state::StateTree;

pub const SNAPSHOT_MAGIC: &[u8; 8] = b"SOVSNAP1";
pub const SNAPSHOT_FORMAT_VERSION_V1: u16 = 1;
pub const SNAPSHOT_DOMAIN_TAG: &str = "SOVEREIGN_OS:STATE_SNAPSHOT:v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    InvalidMagic([u8; 8]),
    UnsupportedVersion(u16),
    ChecksumMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    StateRootMismatch {
        expected: String,
        computed: String,
    },
    IoError(String),
    MalformedPayload(String),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic(m) => write!(f, "Invalid snapshot magic: {:?}", m),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported snapshot version: {}", v),
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Snapshot checksum mismatch: expected {:?}, actual {:?}",
                    expected, actual
                )
            }
            Self::StateRootMismatch { expected, computed } => {
                write!(
                    f,
                    "Snapshot state root mismatch: expected {}, computed {}",
                    expected, computed
                )
            }
            Self::IoError(e) => write!(f, "Snapshot I/O error: {}", e),
            Self::MalformedPayload(e) => write!(f, "Malformed snapshot payload: {}", e),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Verified metadata manifest accompanying a durable state snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotManifest {
    pub sequence_tick: u64,
    pub revision: u64,
    pub state_root: String,
    pub transition_root: String,
    pub entry_count: u64,
    pub checksum: [u8; 32],
}

pub struct StateSnapshot;

impl StateSnapshot {
    /// Computes the domain-separated BLAKE3 checksum over snapshot header and canonical entries payload.
    pub fn compute_checksum(
        sequence_tick: u64,
        revision: u64,
        state_root: &str,
        transition_root: &str,
        entry_count: u64,
        entries_payload: &[u8],
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        // 1. Domain tag
        hasher.update(&(SNAPSHOT_DOMAIN_TAG.len() as u64).to_be_bytes());
        hasher.update(SNAPSHOT_DOMAIN_TAG.as_bytes());

        // 2. Fixed-width header elements
        hasher.update(SNAPSHOT_MAGIC);
        hasher.update(&SNAPSHOT_FORMAT_VERSION_V1.to_be_bytes());
        hasher.update(&sequence_tick.to_be_bytes());
        hasher.update(&revision.to_be_bytes());

        // 3. Length-prefixed roots
        hasher.update(&(state_root.len() as u32).to_be_bytes());
        hasher.update(state_root.as_bytes());

        hasher.update(&(transition_root.len() as u32).to_be_bytes());
        hasher.update(transition_root.as_bytes());

        hasher.update(&entry_count.to_be_bytes());

        // 4. Length-prefixed entries payload
        hasher.update(&(entries_payload.len() as u64).to_be_bytes());
        hasher.update(entries_payload);

        *hasher.finalize().as_bytes()
    }

    /// Serializes and atomically publishes a durable snapshot using temp-file fsync and atomic rename.
    pub fn write_atomic(
        target_path: impl AsRef<Path>,
        tree: &StateTree,
        sequence_tick: u64,
        transition_root: &str,
    ) -> Result<SnapshotManifest, SnapshotError> {
        let target_path_buf = target_path.as_ref().to_path_buf();
        let parent_dir = target_path_buf.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent_dir).map_err(|e| SnapshotError::IoError(e.to_string()))?;

        let tmp_path = target_path_buf.with_extension(format!("tmp.{}", sequence_tick));

        let revision = tree.revision();
        let state_root = tree.compute_state_root();
        let entry_count = tree.len() as u64;

        // 1. Serialize canonical lexicographically sorted key-value pairs
        let mut entries_payload = Vec::new();
        for (k, v) in tree.entries() {
            entries_payload.extend_from_slice(&(k.len() as u32).to_be_bytes());
            entries_payload.extend_from_slice(k);
            entries_payload.extend_from_slice(&(v.len() as u32).to_be_bytes());
            entries_payload.extend_from_slice(v);
        }

        let checksum = Self::compute_checksum(
            sequence_tick,
            revision,
            &state_root,
            transition_root,
            entry_count,
            &entries_payload,
        );

        // 2. Write to temp file
        {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;

            file.write_all(SNAPSHOT_MAGIC)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            file.write_all(&SNAPSHOT_FORMAT_VERSION_V1.to_be_bytes())
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            file.write_all(&sequence_tick.to_be_bytes())
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            file.write_all(&revision.to_be_bytes())
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;

            let sr_bytes = state_root.as_bytes();
            file.write_all(&(sr_bytes.len() as u32).to_be_bytes())
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            file.write_all(sr_bytes)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;

            let tr_bytes = transition_root.as_bytes();
            file.write_all(&(tr_bytes.len() as u32).to_be_bytes())
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            file.write_all(tr_bytes)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;

            file.write_all(&entry_count.to_be_bytes())
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            file.write_all(&entries_payload)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            file.write_all(&checksum)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;

            // 3. Flush userspace buffers & fsync temp file
            file.flush()
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            file.sync_all()
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
        }

        // 4. Atomic rename from .tmp to final .snap
        std::fs::rename(&tmp_path, &target_path_buf)
            .map_err(|e| SnapshotError::IoError(e.to_string()))?;

        // 5. Parent directory fsync barrier
        Self::sync_directory(parent_dir).map_err(|e| SnapshotError::IoError(e.to_string()))?;

        Ok(SnapshotManifest {
            sequence_tick,
            revision,
            state_root,
            transition_root: transition_root.to_string(),
            entry_count,
            checksum,
        })
    }

    /// Reads, verifies cryptographic checksum, recomputes state root, and hydrates StateTree from snapshot.
    pub fn load_and_verify(
        path: impl AsRef<Path>,
    ) -> Result<(StateTree, SnapshotManifest), SnapshotError> {
        let path_buf: PathBuf = path.as_ref().to_path_buf();
        let mut file = File::open(&path_buf).map_err(|e| SnapshotError::IoError(e.to_string()))?;

        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)
            .map_err(|e| SnapshotError::IoError(e.to_string()))?;
        if &magic != SNAPSHOT_MAGIC {
            return Err(SnapshotError::InvalidMagic(magic));
        }

        let mut ver_buf = [0u8; 2];
        file.read_exact(&mut ver_buf)
            .map_err(|e| SnapshotError::IoError(e.to_string()))?;
        let version = u16::from_be_bytes(ver_buf);
        if version != SNAPSHOT_FORMAT_VERSION_V1 {
            return Err(SnapshotError::UnsupportedVersion(version));
        }

        let mut seq_buf = [0u8; 8];
        file.read_exact(&mut seq_buf)
            .map_err(|e| SnapshotError::IoError(e.to_string()))?;
        let sequence_tick = u64::from_be_bytes(seq_buf);

        let mut rev_buf = [0u8; 8];
        file.read_exact(&mut rev_buf)
            .map_err(|e| SnapshotError::IoError(e.to_string()))?;
        let revision = u64::from_be_bytes(rev_buf);

        fn read_string(r: &mut impl Read) -> Result<String, SnapshotError> {
            let mut len_buf = [0u8; 4];
            r.read_exact(&mut len_buf)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0u8; len];
            r.read_exact(&mut buf)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            String::from_utf8(buf).map_err(|e| SnapshotError::MalformedPayload(e.to_string()))
        }

        let state_root = read_string(&mut file)?;
        let transition_root = read_string(&mut file)?;

        let mut count_buf = [0u8; 8];
        file.read_exact(&mut count_buf)
            .map_err(|e| SnapshotError::IoError(e.to_string()))?;
        let entry_count = u64::from_be_bytes(count_buf);

        let mut records = BTreeMap::new();
        let mut entries_payload = Vec::new();

        for _ in 0..entry_count {
            let mut k_len_buf = [0u8; 4];
            file.read_exact(&mut k_len_buf)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            let k_len = u32::from_be_bytes(k_len_buf) as usize;
            let mut k = vec![0u8; k_len];
            file.read_exact(&mut k)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;

            entries_payload.extend_from_slice(&k_len_buf);
            entries_payload.extend_from_slice(&k);

            let mut v_len_buf = [0u8; 4];
            file.read_exact(&mut v_len_buf)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;
            let v_len = u32::from_be_bytes(v_len_buf) as usize;
            let mut v = vec![0u8; v_len];
            file.read_exact(&mut v)
                .map_err(|e| SnapshotError::IoError(e.to_string()))?;

            entries_payload.extend_from_slice(&v_len_buf);
            entries_payload.extend_from_slice(&v);

            records.insert(k, v);
        }

        let mut checksum = [0u8; 32];
        file.read_exact(&mut checksum)
            .map_err(|e| SnapshotError::IoError(e.to_string()))?;

        // 1. Verify cryptographic checksum
        let expected_checksum = Self::compute_checksum(
            sequence_tick,
            revision,
            &state_root,
            &transition_root,
            entry_count,
            &entries_payload,
        );

        if checksum != expected_checksum {
            return Err(SnapshotError::ChecksumMismatch {
                expected: expected_checksum,
                actual: checksum,
            });
        }

        // 2. Hydrate tree and verify state root recomputation
        let tree = StateTree::from_entries(records, revision);
        let computed_root = tree.compute_state_root();

        if computed_root != state_root {
            return Err(SnapshotError::StateRootMismatch {
                expected: state_root,
                computed: computed_root,
            });
        }

        Ok((
            tree,
            SnapshotManifest {
                sequence_tick,
                revision,
                state_root,
                transition_root,
                entry_count,
                checksum,
            },
        ))
    }

    fn sync_directory(dir: impl AsRef<Path>) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            let f = File::open(dir)?;
            f.sync_all()?;
        }
        #[cfg(windows)]
        {
            // Windows NTFS handles directory metadata persistence with file sync barriers
            let _ = dir;
        }
        Ok(())
    }
}
