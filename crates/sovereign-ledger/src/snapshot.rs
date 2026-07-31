//! Snapshot primitives for ledger checkpoint metadata.

use crate::{
    checksum::crc32c,
    error::{RejectedSnapshot, RejectionReason},
    state_root::compute_state_root_from_encoded,
    LedgerConfig, LedgerError, Lsn,
};
use sovereign_core_asm::state::StateVector;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::segment::sync_directory;

pub const SNAPSHOT_HEADER_LEN: usize = 44;
pub const SNAPSHOT_CHECKSUM_LEN: usize = 4;

static NEXT_PENDING_SNAPSHOT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SnapshotHeader {
    pub associated_lsn: Lsn,
    pub state_root_hash: [u8; 32],
    pub payload_len: u32,
}

/// A snapshot that passed filename, envelope-length, and CRC validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotCandidate {
    pub lsn: Lsn,
    pub header: SnapshotHeader,
    pub payload: Vec<u8>,
}

/// Snapshot candidates and the recoverable rejections observed during discovery.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapshotDiscovery {
    pub candidates: Vec<SnapshotCandidate>,
    pub rejected: Vec<RejectedSnapshot>,
}

impl SnapshotDiscovery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    pub fn len(&self) -> usize {
        self.candidates.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerSnapshotManager;

impl LedgerSnapshotManager {
    pub fn write_snapshot(
        config: &LedgerConfig,
        associated_lsn: Lsn,
        state_root_hash: [u8; 32],
        payload: &[u8],
    ) -> Result<PathBuf, LedgerError> {
        Self::write_snapshot_with_directory_sync(
            config,
            associated_lsn,
            state_root_hash,
            payload,
            sync_directory,
        )
    }

    fn write_snapshot_with_directory_sync<S>(
        config: &LedgerConfig,
        associated_lsn: Lsn,
        state_root_hash: [u8; 32],
        payload: &[u8],
        mut sync_directory: S,
    ) -> Result<PathBuf, LedgerError>
    where
        S: FnMut(&Path) -> std::io::Result<()>,
    {
        config.validate()?;
        fs::create_dir_all(&config.storage_root).map_err(|_| LedgerError::WriteViolation)?;

        let payload_len: u32 = payload
            .len()
            .try_into()
            .map_err(|_| LedgerError::StorageExhausted)?;

        let path = config
            .storage_root
            .join(format!("{:016x}.snap", associated_lsn.get()));
        if path.exists() {
            return Err(LedgerError::WriteViolation);
        }

        let mut header = [0_u8; SNAPSHOT_HEADER_LEN];
        header[0..8].copy_from_slice(&associated_lsn.get().to_be_bytes());
        header[8..40].copy_from_slice(&state_root_hash);
        header[40..44].copy_from_slice(&payload_len.to_be_bytes());

        let checksum = ::crc32c::crc32c_append(crc32c(&header), payload);
        let checksum_bytes = checksum.to_be_bytes();

        let (pending_path, mut pending) = loop {
            let id = NEXT_PENDING_SNAPSHOT_ID.fetch_add(1, Ordering::Relaxed);
            let pending_path = config.storage_root.join(format!(
                ".{:016x}.{}.{}.snap.pending",
                associated_lsn.get(),
                std::process::id(),
                id
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&pending_path)
            {
                Ok(file) => break (pending_path, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(LedgerError::WriteViolation),
            }
        };

        let mut canonical_published = false;
        let result = (|| {
            pending
                .write_all(&header)
                .map_err(|_| LedgerError::StorageExhausted)?;
            pending
                .write_all(payload)
                .map_err(|_| LedgerError::StorageExhausted)?;
            pending
                .write_all(&checksum_bytes)
                .map_err(|_| LedgerError::StorageExhausted)?;
            pending
                .sync_all()
                .map_err(|_| LedgerError::WriteViolation)?;
            drop(pending);

            // Publish the fully synchronized inode without replacing an existing
            // snapshot. This preserves immutability under concurrent writers.
            fs::hard_link(&pending_path, &path).map_err(|_| LedgerError::WriteViolation)?;
            canonical_published = true;

            // The canonical name now owns the synchronized inode. A stale pending
            // link is harmless and ignored by discovery if cleanup is interrupted.
            let _ = fs::remove_file(&pending_path);

            sync_directory(&config.storage_root).map_err(|_| LedgerError::CommitAmbiguous)?;
            Ok(path.clone())
        })();

        if result.is_err() {
            let _ = fs::remove_file(&pending_path);
            if canonical_published {
                let _ = fs::remove_file(&path);
                let _ = sync_directory(&config.storage_root);
            }
        }

        result
    }

    pub fn read_snapshot(
        config: &LedgerConfig,
        associated_lsn: Lsn,
    ) -> Result<(SnapshotHeader, Vec<u8>), LedgerError> {
        config.validate()?;

        let path = config
            .storage_root
            .join(format!("{:016x}.snap", associated_lsn.get()));

        let mut file = OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|_| LedgerError::SegmentCorrupted)?;

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|_| LedgerError::SegmentCorrupted)?;

        if bytes.len() < SNAPSHOT_HEADER_LEN + SNAPSHOT_CHECKSUM_LEN {
            return Err(LedgerError::SegmentCorrupted);
        }

        let parsed_lsn = Lsn(u64::from_be_bytes(
            bytes[0..8]
                .try_into()
                .map_err(|_| LedgerError::SegmentCorrupted)?,
        ));

        if parsed_lsn != associated_lsn {
            return Err(LedgerError::SegmentCorrupted);
        }

        let mut state_root_hash = [0_u8; 32];
        state_root_hash.copy_from_slice(&bytes[8..40]);

        let payload_len = u32::from_be_bytes(
            bytes[40..44]
                .try_into()
                .map_err(|_| LedgerError::SegmentCorrupted)?,
        ) as usize;

        let expected_len = SNAPSHOT_HEADER_LEN
            .checked_add(payload_len)
            .and_then(|n| n.checked_add(SNAPSHOT_CHECKSUM_LEN))
            .ok_or(LedgerError::SegmentCorrupted)?;

        if bytes.len() != expected_len {
            return Err(LedgerError::SegmentCorrupted);
        }

        let payload_start = SNAPSHOT_HEADER_LEN;
        let payload_end = payload_start + payload_len;

        let embedded_checksum = u32::from_be_bytes(
            bytes[payload_end..payload_end + SNAPSHOT_CHECKSUM_LEN]
                .try_into()
                .map_err(|_| LedgerError::SegmentCorrupted)?,
        );

        let computed_checksum = ::crc32c::crc32c_append(
            crc32c(&bytes[..SNAPSHOT_HEADER_LEN]),
            &bytes[payload_start..payload_end],
        );

        if computed_checksum != embedded_checksum {
            return Err(LedgerError::InvalidChecksum);
        }

        Ok((
            SnapshotHeader {
                associated_lsn: parsed_lsn,
                state_root_hash,
                payload_len: payload_len as u32,
            },
            bytes[payload_start..payload_end].to_vec(),
        ))
    }
}

/// Discovers snapshot envelopes newest-first without decoding state payloads.
pub fn snapshot_candidates_descending(
    config: &LedgerConfig,
) -> Result<SnapshotDiscovery, LedgerError> {
    config.validate()?;

    if !config.storage_root.exists() {
        return Ok(SnapshotDiscovery::new());
    }

    let mut discovery = SnapshotDiscovery::new();
    let entries = fs::read_dir(&config.storage_root).map_err(|_| LedgerError::SegmentCorrupted)?;

    for entry in entries {
        let path = entry.map_err(|_| LedgerError::SegmentCorrupted)?.path();
        if !path.is_file() {
            continue;
        }

        let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !filename.to_ascii_lowercase().ends_with(".snap") {
            continue;
        }

        let stem = match filename.strip_suffix(".snap") {
            Some(stem)
                if stem.len() == 16
                    && stem
                        .chars()
                        .all(|character| matches!(character, '0'..='9' | 'a'..='f')) =>
            {
                stem
            }
            _ => {
                discovery.rejected.push(RejectedSnapshot {
                    lsn: None,
                    reason: RejectionReason::FilenameMismatch,
                });
                continue;
            }
        };

        let lsn = match u64::from_str_radix(stem, 16) {
            Ok(value) => Lsn(value),
            Err(_) => {
                discovery.rejected.push(RejectedSnapshot {
                    lsn: None,
                    reason: RejectionReason::FilenameMismatch,
                });
                continue;
            }
        };

        let embedded_lsn = match fs::read(&path) {
            Ok(bytes) if bytes.len() >= 8 => Lsn(u64::from_be_bytes(
                bytes[..8]
                    .try_into()
                    .map_err(|_| LedgerError::SegmentCorrupted)?,
            )),
            Ok(_) => {
                discovery.rejected.push(RejectedSnapshot {
                    lsn: Some(lsn),
                    reason: RejectionReason::Malformed,
                });
                continue;
            }
            Err(_) => return Err(LedgerError::SegmentCorrupted),
        };

        if embedded_lsn != lsn {
            discovery.rejected.push(RejectedSnapshot {
                lsn: Some(lsn),
                reason: RejectionReason::FilenameMismatch,
            });
            continue;
        }

        match LedgerSnapshotManager::read_snapshot(config, lsn) {
            Ok((header, payload)) => discovery.candidates.push(SnapshotCandidate {
                lsn,
                header,
                payload,
            }),
            Err(LedgerError::InvalidChecksum) => {
                discovery.rejected.push(RejectedSnapshot {
                    lsn: Some(lsn),
                    reason: RejectionReason::ChecksumMismatch,
                });
            }
            Err(LedgerError::SegmentCorrupted) => {
                discovery.rejected.push(RejectedSnapshot {
                    lsn: Some(lsn),
                    reason: RejectionReason::Malformed,
                });
            }
            Err(error) => return Err(error),
        }
    }

    discovery
        .candidates
        .sort_by_key(|candidate| std::cmp::Reverse(candidate.lsn));
    Ok(discovery)
}

/// Encodes a state vector, computes its ADR 0001 root, and writes the snapshot.
pub fn write_snapshot_with_root(
    config: &LedgerConfig,
    associated_lsn: Lsn,
    state: &StateVector,
) -> Result<PathBuf, LedgerError> {
    let payload = sovereign_core_asm::snapshot::encode(state);
    let state_root_hash = compute_state_root_from_encoded(&payload);
    LedgerSnapshotManager::write_snapshot(config, associated_lsn, state_root_hash, &payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(name: &str) -> LedgerConfig {
        let path =
            std::env::temp_dir().join(format!("sovereign_snapshot_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);

        let mut config = LedgerConfig::new(path);
        config.max_segment_size = 4096 * 4;
        config.max_record_payload_size = 4096;
        config
    }

    #[test]
    fn snapshot_round_trip_succeeds() {
        let config = test_config("round_trip");
        let hash = [0xAB_u8; 32];
        let payload = b"snapshot_payload";

        LedgerSnapshotManager::write_snapshot(&config, Lsn(10), hash, payload).unwrap();

        let (header, restored) = LedgerSnapshotManager::read_snapshot(&config, Lsn(10)).unwrap();

        assert_eq!(header.associated_lsn, Lsn(10));
        assert_eq!(header.state_root_hash, hash);
        assert_eq!(restored, payload);

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn snapshot_publication_leaves_no_pending_file() {
        let config = test_config("no_pending_file");

        LedgerSnapshotManager::write_snapshot(&config, Lsn(3), [3; 32], b"complete").unwrap();

        let entries: Vec<_> = fs::read_dir(&config.storage_root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(
            entries,
            vec![std::ffi::OsString::from("0000000000000003.snap")]
        );

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn duplicate_snapshot_write_preserves_original() {
        let config = test_config("duplicate_preserves_original");
        let original_hash = [0x11; 32];
        let original_payload = b"original";

        LedgerSnapshotManager::write_snapshot(&config, Lsn(4), original_hash, original_payload)
            .unwrap();

        assert_eq!(
            LedgerSnapshotManager::write_snapshot(&config, Lsn(4), [0x22; 32], b"replacement")
                .unwrap_err(),
            LedgerError::WriteViolation
        );

        let (header, payload) = LedgerSnapshotManager::read_snapshot(&config, Lsn(4)).unwrap();
        assert_eq!(header.state_root_hash, original_hash);
        assert_eq!(payload, original_payload);

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn directory_sync_failure_rolls_back_snapshot_visibility() {
        let config = test_config("directory_sync_failure");
        let canonical = config.storage_root.join("0000000000000005.snap");
        let mut sync_attempts = 0_u8;

        let result = LedgerSnapshotManager::write_snapshot_with_directory_sync(
            &config,
            Lsn(5),
            [5; 32],
            b"ambiguous",
            |_| {
                sync_attempts += 1;
                if sync_attempts == 1 {
                    Err(std::io::Error::other("injected directory sync failure"))
                } else {
                    Ok(())
                }
            },
        );

        assert_eq!(result.unwrap_err(), LedgerError::CommitAmbiguous);
        assert!(!canonical.exists());
        assert_eq!(sync_attempts, 2);
        assert!(fs::read_dir(&config.storage_root)
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".pending")));

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn missing_snapshot_fails_closed() {
        let config = test_config("missing");

        assert_eq!(
            LedgerSnapshotManager::read_snapshot(&config, Lsn(1)).unwrap_err(),
            LedgerError::SegmentCorrupted
        );

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn corrupted_snapshot_checksum_fails_closed() {
        let config = test_config("corrupt");
        let hash = [0xCD_u8; 32];

        let path =
            LedgerSnapshotManager::write_snapshot(&config, Lsn(7), hash, b"payload").unwrap();

        let mut bytes = fs::read(&path).unwrap();
        bytes[SNAPSHOT_HEADER_LEN] ^= 0xFF;
        fs::write(&path, bytes).unwrap();

        assert_eq!(
            LedgerSnapshotManager::read_snapshot(&config, Lsn(7)).unwrap_err(),
            LedgerError::InvalidChecksum
        );

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn discovery_returns_valid_candidates_newest_first() {
        let config = test_config("discovery_order");
        LedgerSnapshotManager::write_snapshot(&config, Lsn(2), [2; 32], b"two").unwrap();
        LedgerSnapshotManager::write_snapshot(&config, Lsn(9), [9; 32], b"nine").unwrap();
        fs::write(config.storage_root.join("notes.txt"), b"ignored").unwrap();

        let discovery = snapshot_candidates_descending(&config).unwrap();

        assert_eq!(discovery.len(), 2);
        assert_eq!(discovery.candidates[0].lsn, Lsn(9));
        assert_eq!(discovery.candidates[1].lsn, Lsn(2));
        assert!(discovery.rejected.is_empty());

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn discovery_records_filename_and_integrity_rejections() {
        let config = test_config("discovery_rejections");
        fs::create_dir_all(&config.storage_root).unwrap();
        fs::write(
            config.storage_root.join("0000000000000001.SNAP"),
            b"bad name",
        )
        .unwrap();

        let corrupt =
            LedgerSnapshotManager::write_snapshot(&config, Lsn(7), [7; 32], b"payload").unwrap();
        let mut bytes = fs::read(&corrupt).unwrap();
        bytes[SNAPSHOT_HEADER_LEN] ^= 0xFF;
        fs::write(&corrupt, bytes).unwrap();

        fs::write(config.storage_root.join("0000000000000008.snap"), b"short").unwrap();

        let discovery = snapshot_candidates_descending(&config).unwrap();

        assert!(discovery.is_empty());
        assert!(discovery.rejected.contains(&RejectedSnapshot {
            lsn: None,
            reason: RejectionReason::FilenameMismatch,
        }));
        assert!(discovery.rejected.contains(&RejectedSnapshot {
            lsn: Some(Lsn(7)),
            reason: RejectionReason::ChecksumMismatch,
        }));
        assert!(discovery.rejected.contains(&RejectedSnapshot {
            lsn: Some(Lsn(8)),
            reason: RejectionReason::Malformed,
        }));

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn discovery_missing_directory_is_empty() {
        let config = test_config("discovery_missing");

        let discovery = snapshot_candidates_descending(&config).unwrap();

        assert!(discovery.is_empty());
        assert!(discovery.rejected.is_empty());
    }

    #[test]
    fn write_snapshot_with_root_uses_normative_hash() {
        let config = test_config("with_root");
        let state = StateVector::new();

        write_snapshot_with_root(&config, Lsn(11), &state).unwrap();
        let (header, payload) = LedgerSnapshotManager::read_snapshot(&config, Lsn(11)).unwrap();

        assert_eq!(
            header.state_root_hash,
            compute_state_root_from_encoded(&payload)
        );
        assert_eq!(payload, sovereign_core_asm::snapshot::encode(&state));

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn discovery_rejects_header_lsn_that_differs_from_filename() {
        let config = test_config("header_filename_lsn_mismatch");
        let state = StateVector::new();
        let original = write_snapshot_with_root(&config, Lsn(5), &state).unwrap();
        let renamed = config.storage_root.join(format!("{:016x}.snap", 7));
        fs::rename(original, renamed).unwrap();

        let discovery = snapshot_candidates_descending(&config).unwrap();

        assert!(discovery.candidates.is_empty());
        assert!(discovery.rejected.iter().any(|rejected| {
            rejected.lsn == Some(Lsn(7)) && rejected.reason == RejectionReason::FilenameMismatch
        }));
    }
}
