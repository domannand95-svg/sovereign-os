//! Shared internal helpers for strict ledger segment scanning.

use crate::record::{
    EventRecord, PAYLOAD_LEN_OFFSET, PAYLOAD_OFFSET, RECORD_CHECKSUM_LEN, RECORD_HEADER_LEN,
};
use crate::{LedgerConfig, LedgerError, Lsn};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct RecordCursor {
    data: Vec<u8>,
    offset: usize,
    expected_lsn: Lsn,
    max_record_payload_size: usize,
    terminated: bool,
}

impl RecordCursor {
    pub(crate) fn open(
        path: &Path,
        config: &LedgerConfig,
        expected_lsn: Lsn,
    ) -> Result<Self, LedgerError> {
        Ok(Self {
            data: fs::read(path).map_err(|_| LedgerError::SegmentCorrupted)?,
            offset: 0,
            expected_lsn,
            max_record_payload_size: config.max_record_payload_size,
            terminated: false,
        })
    }

    pub(crate) fn next_record(&mut self) -> Option<Result<EventRecord<'_>, LedgerError>> {
        if self.terminated {
            return None;
        }
        if self.offset >= self.data.len() {
            self.terminated = true;
            return None;
        }

        let remaining = self.data.len() - self.offset;
        if remaining < RECORD_HEADER_LEN + RECORD_CHECKSUM_LEN {
            self.terminated = true;
            return Some(Err(LedgerError::SegmentCorrupted));
        }

        let payload_len = u32::from_be_bytes(
            self.data[self.offset + PAYLOAD_LEN_OFFSET..self.offset + PAYLOAD_OFFSET]
                .try_into()
                .ok()?,
        ) as usize;
        if payload_len > self.max_record_payload_size {
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

        let record_end = self.offset + record_len;
        match EventRecord::decode(&self.data[self.offset..record_end]) {
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

pub(crate) fn enumerate_segments_strict(
    config: &LedgerConfig,
) -> Result<Vec<(Lsn, PathBuf)>, LedgerError> {
    if !config.storage_root.exists() {
        return Ok(Vec::new());
    }
    let mut segments = Vec::new();
    for entry in fs::read_dir(&config.storage_root).map_err(|_| LedgerError::SegmentCorrupted)? {
        let path = entry.map_err(|_| LedgerError::SegmentCorrupted)?.path();
        if !path.is_file() {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(LedgerError::SegmentCorrupted)?;
        if !filename.to_ascii_lowercase().ends_with(".seg") {
            continue;
        }
        if !filename.ends_with(".seg") {
            return Err(LedgerError::SegmentCorrupted);
        }
        let stem = filename
            .strip_suffix(".seg")
            .ok_or(LedgerError::SegmentCorrupted)?;
        if stem.len() != 16 || !stem.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
            return Err(LedgerError::SegmentCorrupted);
        }
        let base = u64::from_str_radix(stem, 16).map_err(|_| LedgerError::SegmentCorrupted)?;
        if filename != format!("{base:016x}.seg") {
            return Err(LedgerError::SegmentCorrupted);
        }
        segments.push((Lsn(base), path));
    }
    segments.sort_by_key(|(lsn, _)| *lsn);
    Ok(segments)
}

pub(crate) fn scan_segment_records(
    path: &Path,
    config: &LedgerConfig,
    start_lsn: Lsn,
) -> Result<(Option<Lsn>, usize), LedgerError> {
    let mut cursor = RecordCursor::open(path, config, start_lsn)?;
    let mut last = None;
    let mut count = 0;
    while let Some(result) = cursor.next_record() {
        let record = result?;
        last = Some(record.lsn);
        count += 1;
    }
    Ok((last, count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventType, LedgerAppendEngine, ReplayIterator};

    fn config(name: &str) -> LedgerConfig {
        let root =
            std::env::temp_dir().join(format!("sovereign_scan_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut config = LedgerConfig::new(root);
        config.max_segment_size = 4096 * 4;
        config.max_record_payload_size = 4096;
        config
    }

    fn segment(config: &LedgerConfig) -> PathBuf {
        enumerate_segments_strict(config).unwrap()[0].1.clone()
    }

    fn parity(config: &LedgerConfig, path: &Path, start: Lsn) {
        let mut cursor = RecordCursor::open(path, config, start).unwrap();
        let mut replay = ReplayIterator::bootstrap(config.clone(), start).unwrap();
        loop {
            match (cursor.next_record(), replay.next_record()) {
                (Some(Ok(a)), Some(Ok(b))) => {
                    assert_eq!(a.lsn, b.lsn);
                    assert_eq!(a.event_type, b.event_type);
                    assert_eq!(a.payload, b.payload);
                }
                (Some(Err(a)), Some(Err(b))) => {
                    assert_eq!(a, b);
                    break;
                }
                (None, None) => break,
                (a, b) => panic!("cursor/replay divergence: {a:?} / {b:?}"),
            }
        }
        assert!(cursor.next_record().is_none());
        assert!(replay.next_record().is_none());
    }

    #[test]
    fn valid_records_match_replay() {
        let config = config("valid");
        let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        writer.append(EventType::KernelDirective, b"zero").unwrap();
        writer.flush().unwrap();
        parity(&config, &segment(&config), Lsn(0));
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn checksum_error_matches_replay_and_terminates() {
        let config = config("checksum");
        let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        writer.append(EventType::KernelDirective, b"data").unwrap();
        writer.flush().unwrap();
        let path = segment(&config);
        let mut bytes = fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        fs::write(&path, bytes).unwrap();
        parity(&config, &path, Lsn(0));
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn truncation_matches_replay() {
        let config = config("truncated");
        fs::create_dir_all(&config.storage_root).unwrap();
        let path = config.storage_root.join("0000000000000000.seg");
        fs::write(&path, [0_u8; 10]).unwrap();
        parity(&config, &path, Lsn(0));
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn lsn_gap_matches_replay() {
        let config = config("gap");
        fs::create_dir_all(&config.storage_root).unwrap();
        let first = EventRecord {
            lsn: Lsn(0),
            event_type: EventType::KernelDirective,
            payload: b"first",
            checksum: 0,
        };
        let third = EventRecord {
            lsn: Lsn(2),
            event_type: EventType::KernelDirective,
            payload: b"third",
            checksum: 0,
        };
        let mut a = [0_u8; 128];
        let mut b = [0_u8; 128];
        let wa = first.encode_into(&mut a).unwrap();
        let wb = third.encode_into(&mut b).unwrap();
        let mut bytes = Vec::with_capacity(wa + wb);
        bytes.extend_from_slice(&a[..wa]);
        bytes.extend_from_slice(&b[..wb]);
        let path = config.storage_root.join("0000000000000000.seg");
        fs::write(&path, bytes).unwrap();
        parity(&config, &path, Lsn(0));
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn strict_enumeration_rejects_case_and_malformed_names() {
        for (name, filename) in [
            ("upper", "0000000000000000.SEG"),
            ("mixed", "0000000000000000.SeG"),
            ("badhex", "000000000000000g.seg"),
        ] {
            let config = config(name);
            fs::create_dir_all(&config.storage_root).unwrap();
            fs::write(config.storage_root.join(filename), []).unwrap();
            assert_eq!(
                enumerate_segments_strict(&config),
                Err(LedgerError::SegmentCorrupted)
            );
            let _ = fs::remove_dir_all(&config.storage_root);
        }
    }

    #[test]
    fn missing_directory_and_empty_segment_are_empty() {
        let config = config("empty");
        assert!(enumerate_segments_strict(&config).unwrap().is_empty());
        fs::create_dir_all(&config.storage_root).unwrap();
        let path = config.storage_root.join("0000000000000000.seg");
        fs::write(&path, []).unwrap();
        assert_eq!(
            scan_segment_records(&path, &config, Lsn(0)).unwrap(),
            (None, 0)
        );
        let _ = fs::remove_dir_all(&config.storage_root);
    }
}
