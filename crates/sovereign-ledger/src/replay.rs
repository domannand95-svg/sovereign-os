//! Replay mechanics for deterministic ledger reconstruction.

use crate::record::{
    EventRecord, PAYLOAD_LEN_OFFSET, PAYLOAD_OFFSET, RECORD_CHECKSUM_LEN, RECORD_HEADER_LEN,
};
use crate::scan::enumerate_segments_strict;
use crate::{
    domain_integration::{LedgerEventMapper, LedgerStateTransition, LedgerTransitionError},
    LedgerConfig, LedgerError, Lsn,
};
use sovereign_core_asm::state::{StateTransition, StateVector};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

/// Error returned while reconstructing state from validated ledger records.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError<MapperError> {
    /// Ledger decoding, integrity, or ordering failure.
    Ledger(LedgerError),

    /// Domain mapping rejected the record.
    Mapping(MapperError),

    /// The mapped transition could not be applied to the state vector.
    StateApplication(LedgerTransitionError),
}

/// Summary returned only after the complete replay succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaySummary {
    /// Number of transitions successfully applied.
    pub records_applied: usize,
    /// LSN of the last transition successfully applied, or none for empty replay.
    pub final_lsn: Option<Lsn>,
}

/// Coordinates deterministic reconstruction of state from ordered ledger data.
pub struct LedgerStateReplayer<M> {
    mapper: M,
}

impl<M> LedgerStateReplayer<M>
where
    M: LedgerEventMapper,
{
    /// Creates a replay coordinator using the supplied domain mapper.
    pub const fn new(mapper: M) -> Self {
        Self { mapper }
    }

    /// Applies every validated ledger record to `vector` in ledger order.
    ///
    /// Replay is fail-stop. On failure, transitions already applied remain in
    /// `vector`. The caller must not treat the target as successfully
    /// reconstructed unless this method returns `Ok`.
    pub fn replay(
        &self,
        records: &mut ReplayIterator,
        vector: &mut StateVector,
    ) -> Result<ReplaySummary, ReplayError<M::Error>> {
        let mut records_applied = 0;
        let mut final_lsn = None;

        while let Some(record_result) = records.next_record() {
            let record = record_result.map_err(ReplayError::Ledger)?;

            let transition = LedgerStateTransition::from_event(&record, &self.mapper)
                .map_err(ReplayError::Mapping)?;

            let _receipt = transition
                .apply(vector)
                .map_err(ReplayError::StateApplication)?;

            final_lsn = Some(record.lsn);
            records_applied += 1;
        }

        Ok(ReplaySummary {
            records_applied,
            final_lsn,
        })
    }
}

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

        for entry in
            fs::read_dir(&config.storage_root).map_err(|_| LedgerError::SegmentCorrupted)?
        {
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

    /// Creates an iterator positioned at the first record after a validated checkpoint.
    ///
    /// Every record through the checkpoint is decoded and continuity-checked before
    /// the returned iterator can yield a delta. A checkpoint at the ledger tail
    /// returns an exhausted iterator. Missing or future checkpoints fail closed.
    pub fn from_checkpoint_lsn(
        config: LedgerConfig,
        checkpoint_lsn: Lsn,
    ) -> Result<Self, LedgerError> {
        config.validate()?;
        let target_lsn = checkpoint_lsn.next()?;
        let segment_paths = enumerate_segments_strict(&config)?;

        let mut replay = Self {
            config,
            segment_paths,
            current_segment_idx: 0,
            current_segment_bytes: Vec::new(),
            offset: 0,
            expected_lsn: Lsn(0),
            terminated: false,
        };
        replay.load_current_segment()?;

        let mut last_seen = None;

        loop {
            let record_result = match replay.next_record() {
                Some(record_result) => record_result,
                None => break,
            };

            let (record_lsn, encoded_len) = {
                let record = record_result?;
                (record.lsn, record.encoded_len())
            };
            last_seen = Some(record_lsn);

            if record_lsn == target_lsn {
                replay.offset = replay
                    .offset
                    .checked_sub(encoded_len)
                    .ok_or(LedgerError::SegmentCorrupted)?;
                replay.expected_lsn = target_lsn;
                replay.terminated = false;
                return Ok(replay);
            }

            if record_lsn > target_lsn {
                return Err(LedgerError::LsnSequenceGap);
            }
        }

        if last_seen == Some(checkpoint_lsn) {
            return Ok(replay);
        }

        Err(LedgerError::LsnSequenceGap)
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
            self.current_segment_bytes
                [self.offset + PAYLOAD_LEN_OFFSET..self.offset + PAYLOAD_OFFSET]
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
        let path =
            std::env::temp_dir().join(format!("sovereign_replay_{name}_{}", std::process::id()));
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
            writer
                .append(EventType::KernelDirective, b"payload_0")
                .unwrap();
            writer
                .append(EventType::RegistryMutation, b"payload_1")
                .unwrap();
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
            writer
                .append(EventType::KernelDirective, b"payload")
                .unwrap();
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
        assert_eq!(
            replay.next_record().unwrap().unwrap_err(),
            LedgerError::SegmentCorrupted
        );

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn lsn_gap_fails_closed() {
        let config = test_config("gap");

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer
                .append(EventType::KernelDirective, b"payload_0")
                .unwrap();
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
        assert_eq!(
            replay.next_record().unwrap().unwrap_err(),
            LedgerError::LsnSequenceGap
        );

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn checkpoint_seek_starts_at_first_delta() {
        let config = test_config("checkpoint_mid_segment");

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer.append(EventType::KernelDirective, b"zero").unwrap();
            writer.append(EventType::KernelDirective, b"one").unwrap();
            writer.append(EventType::KernelDirective, b"two").unwrap();
            writer.flush().unwrap();
        }

        let mut replay = ReplayIterator::from_checkpoint_lsn(config.clone(), Lsn(0)).unwrap();
        assert_eq!(replay.next_record().unwrap().unwrap().lsn, Lsn(1));
        assert_eq!(replay.next_record().unwrap().unwrap().lsn, Lsn(2));
        assert!(replay.next_record().is_none());

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn checkpoint_seek_crosses_segment_boundary() {
        let mut config = test_config("checkpoint_segment_boundary");
        config.max_record_payload_size = 1;
        config.max_segment_size = RECORD_HEADER_LEN + 1 + RECORD_CHECKSUM_LEN;

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer.append(EventType::KernelDirective, b"a").unwrap();
            writer.append(EventType::KernelDirective, b"b").unwrap();
            writer.flush().unwrap();
        }

        let mut replay = ReplayIterator::from_checkpoint_lsn(config.clone(), Lsn(0)).unwrap();
        assert_eq!(replay.next_record().unwrap().unwrap().lsn, Lsn(1));
        assert!(replay.next_record().is_none());

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn checkpoint_at_tail_returns_empty_iterator() {
        let config = test_config("checkpoint_at_tail");

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer.append(EventType::KernelDirective, b"zero").unwrap();
            writer.append(EventType::KernelDirective, b"one").unwrap();
            writer.flush().unwrap();
        }

        let mut replay = ReplayIterator::from_checkpoint_lsn(config.clone(), Lsn(1)).unwrap();
        assert!(replay.next_record().is_none());

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn missing_or_future_checkpoint_fails_closed() {
        let config = test_config("checkpoint_missing");

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer.append(EventType::KernelDirective, b"zero").unwrap();
            writer.flush().unwrap();
        }

        assert!(matches!(
            ReplayIterator::from_checkpoint_lsn(config.clone(), Lsn(9)),
            Err(LedgerError::LsnSequenceGap)
        ));

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn checkpoint_lsn_overflow_fails_closed() {
        let config = test_config("checkpoint_overflow");

        assert!(matches!(
            ReplayIterator::from_checkpoint_lsn(config.clone(), Lsn(u64::MAX)),
            Err(LedgerError::LsnOverflow)
        ));

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    use crate::domain_integration::MappedLedgerWrite;
    use sovereign_core_asm::state::StateCoordinate;
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ReplayTestMappingError {
        Rejected,
    }

    struct ReplayTestMapper {
        calls: Rc<Cell<usize>>,
        coordinate: StateCoordinate,
        fail_at: Option<usize>,
    }

    impl LedgerEventMapper for ReplayTestMapper {
        type Error = ReplayTestMappingError;

        fn map<'payload>(
            &self,
            event: &EventRecord<'payload>,
        ) -> Result<MappedLedgerWrite<'payload>, Self::Error> {
            let call = self.calls.get();
            self.calls.set(call + 1);

            if self.fail_at == Some(call) {
                return Err(ReplayTestMappingError::Rejected);
            }

            Ok(MappedLedgerWrite::new(self.coordinate, event.payload))
        }
    }

    #[test]
    fn empty_replay_reports_no_final_lsn() {
        let config = test_config("gate6_empty_summary");
        fs::create_dir_all(&config.storage_root).unwrap();
        let coordinate = StateCoordinate::new(9).unwrap();
        let replayer = LedgerStateReplayer::new(ReplayTestMapper {
            calls: Rc::new(Cell::new(0)),
            coordinate,
            fail_at: None,
        });
        let mut records = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();
        let mut vector = StateVector::default();

        let summary = replayer.replay(&mut records, &mut vector).unwrap();

        assert_eq!(summary.records_applied, 0);
        assert_eq!(summary.final_lsn, None);

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn ordered_records_reconstruct_expected_state() {
        let config = test_config("gate6_ordered");
        let coordinate = StateCoordinate::new(10).unwrap();

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer.append(EventType::KernelDirective, b"first").unwrap();
            writer
                .append(EventType::RegistryMutation, b"second")
                .unwrap();
            writer.flush().unwrap();
        }

        let calls = Rc::new(Cell::new(0));
        let mapper = ReplayTestMapper {
            calls: Rc::clone(&calls),
            coordinate,
            fail_at: None,
        };
        let replayer = LedgerStateReplayer::new(mapper);
        let mut records = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();
        let mut vector = StateVector::default();

        let summary = replayer.replay(&mut records, &mut vector).unwrap();

        assert_eq!(summary.records_applied, 2);
        assert_eq!(summary.final_lsn, Some(Lsn(1)));
        assert_eq!(calls.get(), 2);
        assert_eq!(vector.get(coordinate).read_bytes(), b"second");

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn repeat_replay_produces_identical_state() {
        let config = test_config("gate6_parity");
        let coordinate = StateCoordinate::new(11).unwrap();

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer.append(EventType::KernelDirective, b"alpha").unwrap();
            writer
                .append(EventType::RegistryMutation, b"omega")
                .unwrap();
            writer.flush().unwrap();
        }

        let calls_a = Rc::new(Cell::new(0));
        let calls_b = Rc::new(Cell::new(0));

        let replayer_a = LedgerStateReplayer::new(ReplayTestMapper {
            calls: Rc::clone(&calls_a),
            coordinate,
            fail_at: None,
        });
        let replayer_b = LedgerStateReplayer::new(ReplayTestMapper {
            calls: Rc::clone(&calls_b),
            coordinate,
            fail_at: None,
        });

        let mut records_a = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();
        let mut records_b = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();
        let mut vector_a = StateVector::default();
        let mut vector_b = StateVector::default();

        let summary_a = replayer_a.replay(&mut records_a, &mut vector_a).unwrap();
        let summary_b = replayer_b.replay(&mut records_b, &mut vector_b).unwrap();

        assert_eq!(summary_a, summary_b);
        assert_eq!(summary_a.records_applied, 2);
        assert_eq!(summary_a.final_lsn, Some(Lsn(1)));
        assert_eq!(calls_a.get(), 2);
        assert_eq!(calls_b.get(), 2);
        assert_eq!(
            vector_a.get(coordinate).read_bytes(),
            vector_b.get(coordinate).read_bytes(),
        );

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn mapper_failure_preserves_earlier_write_and_stops() {
        let config = test_config("gate6_mapper_failure");
        let coordinate = StateCoordinate::new(12).unwrap();

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer
                .append(EventType::KernelDirective, b"committed")
                .unwrap();
            writer
                .append(EventType::RegistryMutation, b"rejected")
                .unwrap();
            writer
                .append(EventType::KernelDirective, b"unreached")
                .unwrap();
            writer.flush().unwrap();
        }

        let calls = Rc::new(Cell::new(0));
        let mapper = ReplayTestMapper {
            calls: Rc::clone(&calls),
            coordinate,
            fail_at: Some(1),
        };
        let replayer = LedgerStateReplayer::new(mapper);
        let mut records = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();
        let mut vector = StateVector::default();

        let result = replayer.replay(&mut records, &mut vector);

        assert_eq!(
            result,
            Err(ReplayError::Mapping(ReplayTestMappingError::Rejected))
        );
        assert_eq!(calls.get(), 2);
        assert_eq!(vector.get(coordinate).read_bytes(), b"committed");

        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn ledger_error_stops_before_mapping() {
        let config = test_config("gate6_ledger_failure");
        let coordinate = StateCoordinate::new(13).unwrap();

        {
            let mut writer = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
            writer
                .append(EventType::KernelDirective, b"payload")
                .unwrap();
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

        let calls = Rc::new(Cell::new(0));
        let mapper = ReplayTestMapper {
            calls: Rc::clone(&calls),
            coordinate,
            fail_at: None,
        };
        let replayer = LedgerStateReplayer::new(mapper);
        let mut records = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();
        let mut vector = StateVector::default();

        let result = replayer.replay(&mut records, &mut vector);

        assert!(matches!(
            result,
            Err(ReplayError::Ledger(LedgerError::SegmentCorrupted))
        ));
        assert_eq!(calls.get(), 0);
        assert_eq!(vector.get(coordinate).read_bytes(), b"");

        let _ = fs::remove_dir_all(&config.storage_root);
    }
}
