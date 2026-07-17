//! Snapshot-coordinated ledger restoration.
//!
//! Every candidate is decoded and every replay is applied to a new, owned
//! `StateVector`. State is returned only after replay reaches the independently
//! discovered ledger tail.

use crate::domain_integration::LedgerEventMapper;
use crate::error::{
    FallbackReason, RejectedSnapshot, RejectionReason, RestorationError, RestorationResult,
};
use crate::replay::{LedgerStateReplayer, ReplayError, ReplayIterator};
use crate::snapshot::snapshot_candidates_descending;
use crate::state_root::compute_state_root_from_encoded;
use crate::tail::discover_ledger_tail;
use crate::{LedgerConfig, Lsn};
use sovereign_core_asm::state::StateVector;

/// Result of a successful restoration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorationOutcome {
    pub state: StateVector,
    pub final_lsn: Option<Lsn>,
    pub source: RestorationSource,
    pub diagnostics: RestorationDiagnostics,
}

/// Source used to produce the restored state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestorationSource {
    Empty,
    FullReplay {
        final_lsn: Lsn,
    },
    SnapshotAtTail {
        checkpoint_lsn: Lsn,
        final_lsn: Lsn,
    },
    SnapshotWithDeltas {
        checkpoint_lsn: Lsn,
        deltas_applied: u64,
        final_lsn: Lsn,
    },
}

/// Diagnostics collected before a successful restoration is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorationDiagnostics {
    pub tail_lsn: Option<Lsn>,
    pub validation: SnapshotValidationCounters,
    pub deltas_attempted: u64,
    pub deltas_applied: u64,
    pub fallback_reason: Option<FallbackReason>,
}

/// Typed counters and rejection details from snapshot validation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapshotValidationCounters {
    pub candidates_checked: u64,
    pub filename_mismatch: u64,
    pub envelope_valid: u64,
    pub payload_valid: u64,
    pub root_valid: u64,
    pub lsn_in_range: u64,
    pub selected: Option<Lsn>,
    pub rejections: Vec<RejectedSnapshot>,
}

fn map_replay_error<MapperError>(error: ReplayError<MapperError>) -> RestorationError<MapperError> {
    match error {
        ReplayError::Ledger(error) => RestorationError::Ledger(error),
        ReplayError::Mapping(error) => RestorationError::Mapping(error),
        ReplayError::StateApplication(error) => RestorationError::StateApplication(error),
    }
}

/// Restores state from the newest valid snapshot, or replays from genesis.
///
/// The coordinator owns `mapper`. Snapshot validation does not require it;
/// when replay is required, the mapper moves into exactly one replay attempt. All mutation occurs on a
/// temporary state vector owned by this function, so an error never exposes a
/// partially restored state.
pub fn initialize_from_snapshot_or_replay<M>(
    config: &LedgerConfig,
    mapper: M,
) -> RestorationResult<RestorationOutcome, M::Error>
where
    M: LedgerEventMapper,
{
    let tail = discover_ledger_tail(config)
        .map_err(RestorationError::Ledger)?
        .tail_lsn;

    let Some(tail_lsn) = tail else {
        return Ok(RestorationOutcome {
            state: StateVector::new(),
            final_lsn: None,
            source: RestorationSource::Empty,
            diagnostics: RestorationDiagnostics {
                tail_lsn: None,
                validation: SnapshotValidationCounters::default(),
                deltas_attempted: 0,
                deltas_applied: 0,
                fallback_reason: None,
            },
        });
    };

    let discovery = snapshot_candidates_descending(config).map_err(RestorationError::Ledger)?;
    let had_snapshot_files = !discovery.candidates.is_empty() || !discovery.rejected.is_empty();
    let mut validation = SnapshotValidationCounters {
        filename_mismatch: discovery
            .rejected
            .iter()
            .filter(|rejected| rejected.reason == RejectionReason::FilenameMismatch)
            .count() as u64,
        envelope_valid: discovery.candidates.len() as u64,
        rejections: discovery.rejected,
        ..SnapshotValidationCounters::default()
    };
    let mut validation_fallback = None;
    let mut selected = None;

    for candidate in discovery.candidates {
        validation.candidates_checked += 1;

        if candidate.lsn > tail_lsn {
            validation.rejections.push(RejectedSnapshot {
                lsn: Some(candidate.lsn),
                reason: RejectionReason::BeyondTail,
            });
            validation_fallback.get_or_insert(FallbackReason::SnapshotBeyondTail {
                lsn: candidate.lsn,
                tail: tail_lsn,
            });
            continue;
        }
        validation.lsn_in_range += 1;

        let state = match sovereign_core_asm::snapshot::decode(&candidate.payload) {
            Ok(state) => state,
            Err(_) => {
                validation.rejections.push(RejectedSnapshot {
                    lsn: Some(candidate.lsn),
                    reason: RejectionReason::DecodeFailed,
                });
                validation_fallback
                    .get_or_insert(FallbackReason::SnapshotDecodeFailed { lsn: candidate.lsn });
                continue;
            }
        };
        validation.payload_valid += 1;

        if compute_state_root_from_encoded(&candidate.payload) != candidate.header.state_root_hash {
            validation.rejections.push(RejectedSnapshot {
                lsn: Some(candidate.lsn),
                reason: RejectionReason::RootMismatch,
            });
            validation_fallback
                .get_or_insert(FallbackReason::StateRootMismatch { lsn: candidate.lsn });
            continue;
        }
        validation.root_valid += 1;
        validation.selected = Some(candidate.lsn);
        selected = Some((candidate.lsn, state));
        break;
    }

    if let Some((checkpoint_lsn, mut state)) = selected {
        let diagnostics = RestorationDiagnostics {
            tail_lsn: Some(tail_lsn),
            validation,
            deltas_attempted: 0,
            deltas_applied: 0,
            fallback_reason: None,
        };

        if checkpoint_lsn == tail_lsn {
            return Ok(RestorationOutcome {
                state,
                final_lsn: Some(tail_lsn),
                source: RestorationSource::SnapshotAtTail {
                    checkpoint_lsn,
                    final_lsn: tail_lsn,
                },
                diagnostics,
            });
        }

        let mut records = ReplayIterator::from_checkpoint_lsn(config.clone(), checkpoint_lsn)
            .map_err(RestorationError::Ledger)?;
        let replayer = LedgerStateReplayer::new(mapper);
        let summary = replayer
            .replay(&mut records, &mut state)
            .map_err(map_replay_error)?;

        if summary.final_lsn != Some(tail_lsn) {
            return Err(RestorationError::ReplayTailMismatch {
                expected: tail_lsn,
                actual: summary.final_lsn,
            });
        }

        let deltas_applied = summary.records_applied as u64;
        let mut diagnostics = diagnostics;
        diagnostics.deltas_attempted = deltas_applied;
        diagnostics.deltas_applied = deltas_applied;

        return Ok(RestorationOutcome {
            state,
            final_lsn: summary.final_lsn,
            source: RestorationSource::SnapshotWithDeltas {
                checkpoint_lsn,
                deltas_applied,
                final_lsn: tail_lsn,
            },
            diagnostics,
        });
    }

    let fallback_reason = if !had_snapshot_files {
        FallbackReason::NoSnapshotsFound
    } else {
        validation_fallback.unwrap_or(FallbackReason::NoValidSnapshotsFound)
    };
    let mut state = StateVector::new();
    let mut records =
        ReplayIterator::bootstrap(config.clone(), Lsn(0)).map_err(RestorationError::Ledger)?;
    let replayer = LedgerStateReplayer::new(mapper);
    let summary = replayer
        .replay(&mut records, &mut state)
        .map_err(map_replay_error)?;

    if summary.final_lsn != Some(tail_lsn) {
        return Err(RestorationError::ReplayTailMismatch {
            expected: tail_lsn,
            actual: summary.final_lsn,
        });
    }

    Ok(RestorationOutcome {
        state,
        final_lsn: summary.final_lsn,
        source: RestorationSource::FullReplay {
            final_lsn: tail_lsn,
        },
        diagnostics: RestorationDiagnostics {
            tail_lsn: Some(tail_lsn),
            validation,
            deltas_attempted: 0,
            deltas_applied: 0,
            fallback_reason: Some(fallback_reason),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_integration::MappedLedgerWrite;
    use crate::snapshot::{write_snapshot_with_root, LedgerSnapshotManager};
    use crate::{EventRecord, EventType, LedgerAppendEngine};
    use sovereign_core_asm::state::StateCoordinate;
    use std::fs;

    #[derive(Debug, PartialEq, Eq)]
    struct MappingError;

    struct FixedMapper {
        coordinate: StateCoordinate,
    }

    impl LedgerEventMapper for FixedMapper {
        type Error = MappingError;

        fn map<'payload>(
            &self,
            event: &EventRecord<'payload>,
        ) -> Result<MappedLedgerWrite<'payload>, Self::Error> {
            Ok(MappedLedgerWrite::new(self.coordinate, event.payload))
        }
    }

    fn test_config(name: &str) -> LedgerConfig {
        let path =
            std::env::temp_dir().join(format!("sovereign_restore_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        let mut config = LedgerConfig::new(path);
        config.max_segment_size = 4096 * 4;
        config.max_record_payload_size = 4096;
        config
    }

    fn mapper() -> FixedMapper {
        FixedMapper {
            coordinate: StateCoordinate::new(7).unwrap(),
        }
    }

    fn append(config: &LedgerConfig, payloads: &[&[u8]]) {
        let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        for payload in payloads {
            engine.append(EventType::RegistryMutation, payload).unwrap();
        }
        engine.flush().unwrap();
    }

    #[test]
    fn empty_ledger_returns_empty_state() {
        let config = test_config("empty");
        let outcome = initialize_from_snapshot_or_replay(&config, mapper()).unwrap();

        assert_eq!(outcome.final_lsn, None);
        assert_eq!(outcome.source, RestorationSource::Empty);
        assert_eq!(outcome.diagnostics.tail_lsn, None);
    }

    #[test]
    fn no_snapshot_replays_from_genesis() {
        let config = test_config("genesis");
        append(&config, &[b"first", b"second"]);

        let outcome = initialize_from_snapshot_or_replay(&config, mapper()).unwrap();
        let coordinate = StateCoordinate::new(7).unwrap();

        assert_eq!(outcome.final_lsn, Some(Lsn(1)));
        assert_eq!(outcome.state.get(coordinate).read_bytes(), b"second");
        assert_eq!(
            outcome.source,
            RestorationSource::FullReplay { final_lsn: Lsn(1) }
        );
        assert_eq!(
            outcome.diagnostics.fallback_reason,
            Some(FallbackReason::NoSnapshotsFound)
        );
    }

    #[test]
    fn snapshot_at_tail_needs_no_replay() {
        let config = test_config("snapshot_at_tail");
        append(&config, &[b"checkpoint"]);
        let coordinate = StateCoordinate::new(7).unwrap();
        let mut snapshot_state = StateVector::new();
        snapshot_state.write(coordinate, b"checkpoint").unwrap();
        write_snapshot_with_root(&config, Lsn(0), &snapshot_state).unwrap();

        let outcome = initialize_from_snapshot_or_replay(&config, mapper()).unwrap();

        assert_eq!(outcome.state, snapshot_state);
        assert_eq!(
            outcome.source,
            RestorationSource::SnapshotAtTail {
                checkpoint_lsn: Lsn(0),
                final_lsn: Lsn(0),
            }
        );
        assert_eq!(outcome.diagnostics.validation.selected, Some(Lsn(0)));
    }

    #[test]
    fn snapshot_plus_deltas_matches_genesis_replay() {
        let config = test_config("snapshot_deltas");
        append(&config, &[b"checkpoint", b"delta-one", b"delta-two"]);
        let coordinate = StateCoordinate::new(7).unwrap();
        let mut snapshot_state = StateVector::new();
        snapshot_state.write(coordinate, b"checkpoint").unwrap();
        write_snapshot_with_root(&config, Lsn(0), &snapshot_state).unwrap();

        let outcome = initialize_from_snapshot_or_replay(&config, mapper()).unwrap();

        assert_eq!(outcome.state.get(coordinate).read_bytes(), b"delta-two");
        assert_eq!(
            outcome.source,
            RestorationSource::SnapshotWithDeltas {
                checkpoint_lsn: Lsn(0),
                deltas_applied: 2,
                final_lsn: Lsn(2),
            }
        );
        assert_eq!(outcome.diagnostics.deltas_applied, 2);
    }

    #[test]
    fn invalid_root_falls_back_to_genesis() {
        let config = test_config("invalid_root");
        append(&config, &[b"ledger-state"]);
        let payload = sovereign_core_asm::snapshot::encode(&StateVector::new());
        LedgerSnapshotManager::write_snapshot(&config, Lsn(0), [0; 32], &payload).unwrap();

        let outcome = initialize_from_snapshot_or_replay(&config, mapper()).unwrap();
        let coordinate = StateCoordinate::new(7).unwrap();

        assert_eq!(outcome.state.get(coordinate).read_bytes(), b"ledger-state");
        assert_eq!(
            outcome.diagnostics.fallback_reason,
            Some(FallbackReason::StateRootMismatch { lsn: Lsn(0) })
        );
        assert!(outcome
            .diagnostics
            .validation
            .rejections
            .iter()
            .any(|rejected| rejected.reason == RejectionReason::RootMismatch));
    }
}
