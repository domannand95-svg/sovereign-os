//! Replay mechanics for deterministic ledger reconstruction.

use crate::record::{EventRecord, RECORD_CHECKSUM_LEN, RECORD_HEADER_LEN, PAYLOAD_LEN_OFFSET, PAYLOAD_OFFSET};
use crate::{LedgerConfig, LedgerError, Lsn};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ReplayIterator {
    config: LedgerConfig,
    segment_paths: Vec<(Lsn, PathBuf)>,
    current_segment_idx: usize,
    current_segment_bytes: Vec<u8>,
    offset: usize,
    expected_lsn: Lsn,
    terminated: bool,
}

impl ReplayIterator {
    pub fn bootstrap(config: LedgerConfig, start_from: Lsn) -> Result<Self, LedgerError> {
        config.validate()?;

        let mut segment_paths = Vec::new();

        for entry in fs::read_dir(&config.storage_root).map_err(|_| LedgerError::SegmentCorrupted)? {
            let path = entry.map_err(|_| LedgerError::SegmentCorrupted)?.path();

            if path.is_file() && path.extension() == Some(OsStr::new("seg")) {
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                    if let Ok(base) = u64::from_str_radix(stem, 16) {
                        segment_paths.push((Lsn(base), path));
                    }
                }
            }
        }

        segment_paths.sort_by_key(|(lsn, _)| *lsn);

        let mut replay = Self {
            config,
            segment_paths,
            current_segment_idx: 0,
            current_segment_bytes: Vec::new(),
            offset: 0,
            expected_lsn: start_from,
            terminated: false,
        };

        replay.load_current_segment()?;

        Ok(replay)
    }

    fn load_current_segment(&mut self) -> Result<(), LedgerError> {
        if self.current_segment_idx >= self.segment_paths.len() {
            self.current_segment_bytes.clear();
            return Ok(());
        }

        let path = &self.segment_paths[self.current_segment_idx].1;
        self.current_segment_bytes = fs::read(path).map_err(|_| LedgerError::SegmentCorrupted)?;
        self.offset = 0;

        Ok(())
    }

    pub fn next_record(&mut self) -> Option<Result<EventRecord<'_>, LedgerError>> {
        if self.terminated {
            return None;
        }

        loop {
            if self.current_segment_idx >= self.segment_paths.len() {
                self.terminated = true;
                return None;
            }

            if self.offset >= self.current_segment_bytes.len() {
                self.current_segment_idx += 1;

                if self.current_segment_idx >= self.segment_paths.len() {
                    self.terminated = true;
                    return None;
                }

                if let Err(err) = self.load_current_segment() {
                    self.terminated = true;
                    return Some(Err(err));
                }

                continue;
            }

            break;
        }

        let remaining = self.current_segment_bytes.len() - self.offset;

        if remaining < RECORD_HEADER_LEN + RECORD_CHECKSUM_LEN {
            self.terminated = true;
            return Some(Err(LedgerError::SegmentCorrupted));
        }

        let payload_len = u32::from_be_bytes(
            self.current_segment_bytes[self.offset + PAYLOAD_LEN_OFFSET..self.offset + PAYLOAD_OFFSET]
                .try_into()
                .ok()?,
        ) as usize;

        if payload_len > self.config.max_record_payload_size {
            self.terminated = true;
            return Some(Err(LedgerError::SegmentCorrupted));
        }

        let record_len = RECORD_HEADER_LEN
            .checked_add(payload_len)?
            .checked_add(RECORD_CHECKSUM_LEN)?;

        if remaining < record_len {
            self.terminated = true;
            return Some(Err(LedgerError::SegmentCorrupted));
        }

        let record_start = self.offset;
        let record_end = record_start + record_len;

        let decoded = EventRecord::decode(&self.current_segment_bytes[record_start..record_end]);

        match decoded {
            Ok(record) => {
                if record.lsn != self.expected_lsn {
                    self.terminated = true;
                    return Some(Err(LedgerError::LsnSequenceGap));
                }

                self.offset = record_end;
                self.expected_lsn = match record.lsn.next() {
                    Ok(next) => next,
                    Err(err) => {
                        self.terminated = true;
                        return Some(Err(err));
                    }
                };

                Some(Ok(record))
            }
            Err(err) => {
                self.terminated = true;
                Some(Err(err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventType, LedgerAppendEngine};

    fn test_config(name: &str) -> LedgerConfig {
        let path = std::env::temp_dir().join(format!("sovereign_replay_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);

        let mut config = LedgerConfig::new(path);
        config.max_segment_size = 4096 * 4;
        config.max_record_payload_size = 4096;
        config
    }

    #[test]
    fn clean_stream_replay() {
        let config = test_config("clean");

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer.append(EventType::KernelDirective, b"payload_0").unwrap();
            writer.append(EventType::RegistryMutation, b"payload_1").unwrap();
            writer.flush().unwrap();
        }

        let mut replay = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();

        let rec0 = replay.next_record().unwrap().unwrap();
        assert_eq!(rec0.lsn, Lsn(0));
        assert_eq!(rec0.payload, b"payload_0");

        let rec1 = replay.next_record().unwrap().unwrap();
        assert_eq!(rec1.lsn, Lsn(1));
        assert_eq!(rec1.payload, b"payload_1");

        assert!(replay.next_record().is_none());

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn truncated_record_fails_closed() {
        let config = test_config("truncated");

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer.append(EventType::KernelDirective, b"payload").unwrap();
            writer.flush().unwrap();
        }

        for entry in fs::read_dir(&config.storage_root).unwrap().flatten() {
            if entry.path().extension() == Some(OsStr::new("seg")) {
                fs::OpenOptions::new()
                    .write(true)
                    .open(entry.path())
                    .unwrap()
                    .set_len(10)
                    .unwrap();
            }
        }

        let mut replay = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();
        assert_eq!(replay.next_record().unwrap().unwrap_err(), LedgerError::SegmentCorrupted);

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn lsn_gap_fails_closed() {
        let config = test_config("gap");

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer.append(EventType::KernelDirective, b"payload_0").unwrap();
            writer.flush().unwrap();
        }

        let forged_path = config.storage_root.join(format!("{:016x}.seg", 100));
        let forged = EventRecord {
            lsn: Lsn(100),
            event_type: EventType::KernelDirective,
            payload: b"payload_100",
            checksum: 0,
        };

        let mut buffer = [0_u8; 128];
        let written = forged.encode_into(&mut buffer).unwrap();
        fs::write(forged_path, &buffer[..written]).unwrap();

        let mut replay = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();

        assert_eq!(replay.next_record().unwrap().unwrap().lsn, Lsn(0));
        assert_eq!(replay.next_record().unwrap().unwrap_err(), LedgerError::LsnSequenceGap);

        let _ = fs::remove_dir_all(&config.storage_root);
    }
}
