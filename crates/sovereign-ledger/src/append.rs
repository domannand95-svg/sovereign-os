//! Single-writer append manager with crash-atomic record publication.

use crate::config::{LedgerConfig, MAX_RECORD_OVERHEAD};
use crate::record::{EventRecord, EventType};
use crate::scan::enumerate_segments_strict;
use crate::segment::{sync_directory, AtomicPublishStage};
use crate::tail::discover_ledger_tail;
use crate::{LedgerError, LedgerSegment, Lsn};
use std::fs;

/// Observable durability boundaries used by forensic crash tests and telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendCommitStage {
    /// The complete record exists only in a synchronized non-canonical pending file.
    PendingSynced,
    /// The complete record has been atomically published under its canonical name.
    Published,
    /// The containing directory has been synchronized and the append is durable.
    Durable,
}

#[derive(Debug)]
pub struct LedgerAppendEngine {
    config: LedgerConfig,
    next_lsn: Lsn,
}

impl LedgerAppendEngine {
    pub fn bootstrap(config: LedgerConfig) -> Result<Self, LedgerError> {
        config.validate()?;
        if !config.storage_root.exists() {
            fs::create_dir_all(&config.storage_root).map_err(|_| LedgerError::WriteViolation)?;
        }
        Self::cleanup_pending_files(&config)?;

        let segments = enumerate_segments_strict(&config)?;
        let discovery = discover_ledger_tail(&config)?;
        let next_lsn = match discovery.tail_lsn {
            Some(tail) => tail.next()?,
            None => Lsn::GENESIS,
        };

        if let Some((base_lsn, path)) = segments.last() {
            let is_empty_successor = *base_lsn == next_lsn
                && path
                    .metadata()
                    .map_err(|_| LedgerError::SegmentCorrupted)?
                    .len()
                    == 0;
            if is_empty_successor {
                fs::remove_file(path).map_err(|_| LedgerError::WriteViolation)?;
                Self::sync_storage_root(&config)?;
            }
        }

        Ok(Self { config, next_lsn })
    }

    pub fn append(&mut self, event_type: EventType, payload: &[u8]) -> Result<Lsn, LedgerError> {
        self.append_observed(event_type, payload, |_| {})
    }

    /// Appends one record while reporting the crash-relevant publication boundaries.
    pub fn append_observed<F>(
        &mut self,
        event_type: EventType,
        payload: &[u8],
        mut observer: F,
    ) -> Result<Lsn, LedgerError>
    where
        F: FnMut(AppendCommitStage),
    {
        if payload.is_empty() || payload.len() > self.config.max_record_payload_size {
            return Err(LedgerError::WriteViolation);
        }

        let total_record_size = MAX_RECORD_OVERHEAD
            .checked_add(payload.len())
            .ok_or(LedgerError::StorageExhausted)?;
        if total_record_size > self.config.max_segment_size {
            return Err(LedgerError::StorageExhausted);
        }

        let assigned_lsn = self.next_lsn;
        let record = EventRecord {
            lsn: assigned_lsn,
            event_type,
            payload,
            checksum: 0,
        };
        let mut encoded = vec![0_u8; total_record_size];
        let written = record.encode_into(&mut encoded)?;
        encoded.truncate(written);

        LedgerSegment::publish_atomic_record(
            &self.config.storage_root,
            assigned_lsn,
            self.config.max_segment_size,
            &encoded,
            |stage| {
                observer(match stage {
                    AtomicPublishStage::PendingSynced => AppendCommitStage::PendingSynced,
                    AtomicPublishStage::Published => AppendCommitStage::Published,
                    AtomicPublishStage::Durable => AppendCommitStage::Durable,
                });
            },
        )?;

        self.next_lsn = assigned_lsn.next()?;
        Ok(assigned_lsn)
    }

    /// Re-synchronizes the ledger directory. Each successful append is already durable.
    pub fn flush(&self) -> Result<(), LedgerError> {
        Self::sync_storage_root(&self.config)
    }

    fn cleanup_pending_files(config: &LedgerConfig) -> Result<(), LedgerError> {
        let mut removed = false;
        for entry in fs::read_dir(&config.storage_root).map_err(|_| LedgerError::WriteViolation)? {
            let path = entry.map_err(|_| LedgerError::WriteViolation)?.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if path.is_file() && Self::is_pending_filename(name) {
                if Self::pending_owner_pid(name) == Some(std::process::id()) {
                    continue;
                }
                fs::remove_file(path).map_err(|_| LedgerError::WriteViolation)?;
                removed = true;
            }
        }
        if removed {
            Self::sync_storage_root(config)?;
        }
        Ok(())
    }

    fn is_pending_filename(name: &str) -> bool {
        let parts: Vec<_> = name.split('.').collect();
        parts.len() == 5
            && parts[0].is_empty()
            && parts[1].len() == 16
            && parts[1]
                .chars()
                .all(|value| matches!(value, '0'..='9' | 'a'..='f'))
            && !parts[2].is_empty()
            && parts[2].chars().all(|value| value.is_ascii_digit())
            && !parts[3].is_empty()
            && parts[3].chars().all(|value| value.is_ascii_digit())
            && parts[4] == "pending"
    }

    fn pending_owner_pid(name: &str) -> Option<u32> {
        if !Self::is_pending_filename(name) {
            return None;
        }
        name.split('.').nth(2)?.parse().ok()
    }

    fn sync_storage_root(config: &LedgerConfig) -> Result<(), LedgerError> {
        sync_directory(&config.storage_root).map_err(|_| LedgerError::WriteViolation)
    }

    pub const fn next_lsn(&self) -> Lsn {
        self.next_lsn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(name: &str) -> LedgerConfig {
        let path =
            std::env::temp_dir().join(format!("sovereign_append_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);

        let mut config = LedgerConfig::new(path);
        config.max_segment_size = 4096 * 4;
        config.max_record_payload_size = 4096;
        config
    }

    #[test]
    fn engine_bootstrap_and_incremental_append() {
        let config = test_config("bootstrap");
        let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();

        assert_eq!(engine.next_lsn(), Lsn(0));

        let payload = b"sovereign_os_telemetry_payload_node_alpha";

        assert_eq!(
            engine.append(EventType::KernelDirective, payload),
            Ok(Lsn(0))
        );
        assert_eq!(engine.next_lsn(), Lsn(1));

        assert_eq!(
            engine.append(EventType::RegistryMutation, payload),
            Ok(Lsn(1))
        );
        assert_eq!(engine.next_lsn(), Lsn(2));

        engine.flush().unwrap();
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn automatic_segment_rotation_boundary() {
        let config = test_config("rotation");
        let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();

        let payload = [0x55_u8; 1024];

        for _ in 0..17 {
            engine
                .append(EventType::CapabilityPromotion, &payload)
                .unwrap();
        }

        let segment_count = fs::read_dir(&config.storage_root)
            .unwrap()
            .flatten()
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "seg"))
            .count();

        assert!(segment_count >= 2);

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let config = test_config("oversized");
        let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();

        let payload = vec![0_u8; config.max_record_payload_size + 1];

        assert_eq!(
            engine.append(EventType::KernelDirective, &payload),
            Err(LedgerError::WriteViolation)
        );

        let _ = fs::remove_dir_all(&config.storage_root);
    }
    #[test]
    fn bootstrap_resumes_after_existing_tail() {
        let config = test_config("restart_tail");
        let payload = b"sovereign_os_restart_payload";
        {
            let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            assert_eq!(
                engine.append(EventType::KernelDirective, payload),
                Ok(Lsn(0))
            );
            engine.flush().unwrap();
        }

        let mut restarted = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        assert_eq!(restarted.next_lsn(), Lsn(1));
        assert_eq!(
            restarted.append(EventType::RegistryMutation, payload),
            Ok(Lsn(1))
        );
        restarted.flush().unwrap();

        let tail = discover_ledger_tail(&config).unwrap();
        assert_eq!(tail.tail_lsn, Some(Lsn(1)));
        assert_eq!(tail.records_validated, 2);
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn bootstrap_reuses_empty_trailing_segment() {
        let config = test_config("restart_empty_trailing");
        let payload = b"sovereign_os_restart_payload";
        {
            let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            assert_eq!(
                engine.append(EventType::KernelDirective, payload),
                Ok(Lsn(0))
            );
            engine.flush().unwrap();
        }
        LedgerSegment::create(&config.storage_root, Lsn(1), config.max_segment_size).unwrap();

        let mut restarted = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        assert_eq!(restarted.next_lsn(), Lsn(1));
        assert_eq!(
            restarted.append(EventType::RegistryMutation, payload),
            Ok(Lsn(1))
        );
        restarted.flush().unwrap();

        let tail = discover_ledger_tail(&config).unwrap();
        assert_eq!(tail.tail_lsn, Some(Lsn(1)));
        assert_eq!(tail.segments_scanned, 2);
        let _ = fs::remove_dir_all(&config.storage_root);
    }
    #[test]
    fn bootstrap_removes_stale_pending_artifact() {
        let config = test_config("stale_pending");
        fs::create_dir_all(&config.storage_root).unwrap();
        let stale_pid = std::process::id().wrapping_add(1);
        let pending = config
            .storage_root
            .join(format!(".0000000000000000.{stale_pid}.7.pending"));
        fs::write(&pending, b"partial-record").unwrap();
        let oversized_pid = config
            .storage_root
            .join(".0000000000000000.42949672960.8.pending");
        fs::write(&oversized_pid, b"partial-record").unwrap();

        let engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        assert_eq!(engine.next_lsn(), Lsn(0));
        assert!(!pending.exists());
        assert!(!oversized_pid.exists());
        let _ = fs::remove_dir_all(&config.storage_root);
    }
    #[test]
    fn bootstrap_preserves_live_pending_file_from_current_process() {
        let config = test_config("live_pending");
        let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        let (pending_tx, pending_rx) = std::sync::mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = std::sync::mpsc::sync_channel(0);

        let writer_handle = std::thread::spawn(move || {
            writer.append_observed(EventType::KernelDirective, b"live-writer", |stage| {
                if stage == AppendCommitStage::PendingSynced {
                    pending_tx.send(()).unwrap();
                    resume_rx.recv().unwrap();
                }
            })
        });

        pending_rx.recv().unwrap();
        let observer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        assert_eq!(observer.next_lsn(), Lsn(0));
        let pending_count = fs::read_dir(&config.storage_root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(LedgerAppendEngine::is_pending_filename)
            })
            .count();
        assert_eq!(pending_count, 1);

        resume_tx.send(()).unwrap();
        assert_eq!(writer_handle.join().unwrap(), Ok(Lsn(0)));
        let tail = discover_ledger_tail(&config).unwrap();
        assert_eq!(tail.tail_lsn, Some(Lsn(0)));
        let _ = fs::remove_dir_all(&config.storage_root);
    }
}
