//! Snapshot primitives for ledger checkpoint metadata.

use crate::{checksum::crc32c, LedgerConfig, LedgerError, Lsn};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

pub const SNAPSHOT_HEADER_LEN: usize = 44;
pub const SNAPSHOT_CHECKSUM_LEN: usize = 4;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SnapshotHeader {
    pub associated_lsn: Lsn,
    pub state_root_hash: [u8; 32],
    pub payload_len: u32,
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
        config.validate()?;
        fs::create_dir_all(&config.storage_root).map_err(|_| LedgerError::WriteViolation)?;

        let payload_len: u32 = payload
            .len()
            .try_into()
            .map_err(|_| LedgerError::StorageExhausted)?;

        let path = config
            .storage_root
            .join(format!("{:016x}.snap", associated_lsn.get()));

        let mut header = [0_u8; SNAPSHOT_HEADER_LEN];
        header[0..8].copy_from_slice(&associated_lsn.get().to_be_bytes());
        header[8..40].copy_from_slice(&state_root_hash);
        header[40..44].copy_from_slice(&payload_len.to_be_bytes());

        let checksum = crc32c::crc32c_append(crc32c(&header), payload);
        let checksum_bytes = checksum.to_be_bytes();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|_| LedgerError::WriteViolation)?;

        file.write_all(&header).map_err(|_| LedgerError::StorageExhausted)?;
        file.write_all(payload).map_err(|_| LedgerError::StorageExhausted)?;
        file.write_all(&checksum_bytes).map_err(|_| LedgerError::StorageExhausted)?;
        file.sync_all().map_err(|_| LedgerError::WriteViolation)?;

        Ok(path)
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

        let computed_checksum =
            crc32c::crc32c_append(crc32c(&bytes[..SNAPSHOT_HEADER_LEN]), &bytes[payload_start..payload_end]);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(name: &str) -> LedgerConfig {
        let path = std::env::temp_dir().join(format!(
            "sovereign_snapshot_{name}_{}",
            std::process::id()
        ));
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

        let (header, restored) =
            LedgerSnapshotManager::read_snapshot(&config, Lsn(10)).unwrap();

        assert_eq!(header.associated_lsn, Lsn(10));
        assert_eq!(header.state_root_hash, hash);
        assert_eq!(restored, payload);

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
}
