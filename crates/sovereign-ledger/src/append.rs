//! Single-writer append manager enforcing linear LSN progression and segment rollover.

use crate::config::{LedgerConfig, MAX_RECORD_OVERHEAD};
use crate::record::{EventType, EVENT_TYPE_OFFSET, LSN_OFFSET, PAYLOAD_LEN_OFFSET, PAYLOAD_OFFSET};
use crate::{checksum::crc32c, LedgerError, LedgerSegment, Lsn};
use std::fs;

#[derive(Debug)]
pub struct LedgerAppendEngine {
    config: LedgerConfig,
    active_segment: Option<LedgerSegment>,
    next_lsn: Lsn,
}

impl LedgerAppendEngine {
    pub fn bootstrap(config: LedgerConfig) -> Result<Self, LedgerError> {
        config.validate()?;

        if !config.storage_root.exists() {
            fs::create_dir_all(&config.storage_root).map_err(|_| LedgerError::WriteViolation)?;
        }

        Ok(Self {
            config,
            active_segment: None,
            next_lsn: Lsn::GENESIS,
        })
    }

    pub fn append(&mut self, event_type: EventType, payload: &[u8]) -> Result<Lsn, LedgerError> {
        if payload.is_empty() || payload.len() > self.config.max_record_payload_size {
            return Err(LedgerError::WriteViolation);
        }

        let total_record_size = MAX_RECORD_OVERHEAD
            .checked_add(payload.len())
            .ok_or(LedgerError::StorageExhausted)?;

        if self
            .active_segment
            .as_ref()
            .is_none_or(|segment| !segment.has_capacity(total_record_size))
        {
            self.rotate_active_segment()?;
        }

        let assigned_lsn = self.next_lsn;

        let mut header = [0_u8; PAYLOAD_OFFSET];
        header[LSN_OFFSET..EVENT_TYPE_OFFSET].copy_from_slice(&assigned_lsn.get().to_be_bytes());
        header[EVENT_TYPE_OFFSET] = event_type.as_u8();
        header[PAYLOAD_LEN_OFFSET..PAYLOAD_OFFSET]
            .copy_from_slice(&(payload.len() as u32).to_be_bytes());

        let mut checksum_input = [0_u8; PAYLOAD_OFFSET];
        checksum_input.copy_from_slice(&header);

        let checksum = ::crc32c::crc32c_append(crc32c(&checksum_input), payload);
        let checksum_bytes = checksum.to_be_bytes();

        let segment = self
            .active_segment
            .as_mut()
            .ok_or(LedgerError::WriteViolation)?;

        segment.write_raw(&header)?;
        segment.write_raw(payload)?;
        segment.write_raw(&checksum_bytes)?;

        self.next_lsn = assigned_lsn.next()?;

        Ok(assigned_lsn)
    }

    pub fn flush(&self) -> Result<(), LedgerError> {
        if let Some(segment) = &self.active_segment {
            segment.synchronize()?;
        }
        Ok(())
    }

    fn rotate_active_segment(&mut self) -> Result<(), LedgerError> {
        if let Some(mut old_segment) = self.active_segment.take() {
            old_segment.freeze()?;
        }

        let new_segment = LedgerSegment::create(
            &self.config.storage_root,
            self.next_lsn,
            self.config.max_segment_size,
        )?;

        self.active_segment = Some(new_segment);
        Ok(())
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
}
