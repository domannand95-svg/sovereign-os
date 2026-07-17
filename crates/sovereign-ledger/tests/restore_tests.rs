use sovereign_core_asm::state::{StateCoordinate, StateVector};
use sovereign_ledger::domain_integration::{LedgerEventMapper, MappedLedgerWrite};
use sovereign_ledger::{
    compute_state_root, initialize_from_snapshot_or_replay, write_snapshot_with_root, EventRecord,
    EventType, LedgerAppendEngine, LedgerConfig, Lsn, RestorationError, RestorationSource,
};
use std::fs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MappingError {
    Rejected(Lsn),
    InvalidCoordinate,
}

#[derive(Clone, Copy)]
struct CoordinateMapper {
    reject_at: Option<Lsn>,
}

impl CoordinateMapper {
    const fn accepting() -> Self {
        Self { reject_at: None }
    }

    const fn rejecting_at(lsn: Lsn) -> Self {
        Self {
            reject_at: Some(lsn),
        }
    }
}

impl LedgerEventMapper for CoordinateMapper {
    type Error = MappingError;

    fn map<'payload>(
        &self,
        event: &EventRecord<'payload>,
    ) -> Result<MappedLedgerWrite<'payload>, Self::Error> {
        if self.reject_at == Some(event.lsn) {
            return Err(MappingError::Rejected(event.lsn));
        }

        let coordinate = StateCoordinate::new(event.lsn.get() as u32)
            .map_err(|_| MappingError::InvalidCoordinate)?;
        Ok(MappedLedgerWrite::new(coordinate, event.payload))
    }
}

fn test_config(name: &str) -> LedgerConfig {
    let root = std::env::temp_dir().join(format!(
        "sovereign_restore_integration_{name}_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let mut config = LedgerConfig::new(root);
    config.max_segment_size = 4096 * 4;
    config.max_record_payload_size = 4096;
    config
}

fn append_history(config: &LedgerConfig, payloads: &[&[u8]]) {
    let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
    for payload in payloads {
        engine
            .append(EventType::RegistryMutation, payload)
            .unwrap();
    }
    engine.flush().unwrap();
}

fn state_through(payloads: &[&[u8]], final_index: usize) -> StateVector {
    let mut state = StateVector::new();
    for (index, payload) in payloads.iter().enumerate().take(final_index + 1) {
        let coordinate = StateCoordinate::new(index as u32).unwrap();
        state.write(coordinate, payload).unwrap();
    }
    state
}

fn seeded_live_state() -> StateVector {
    let mut state = StateVector::new();
    state
        .write(StateCoordinate::new(900).unwrap(), b"live-state-before-restore")
        .unwrap();
    state
}

#[test]
fn snapshot_plus_deltas_is_root_equivalent_to_genesis_replay() {
    let payloads: &[&[u8]] = &[b"zero", b"one", b"two", b"three", b"four"];
    let snapshot_config = test_config("equivalence_snapshot");
    let genesis_config = test_config("equivalence_genesis");
    append_history(&snapshot_config, payloads);
    append_history(&genesis_config, payloads);

    let checkpoint_state = state_through(payloads, 2);
    write_snapshot_with_root(&snapshot_config, Lsn(2), &checkpoint_state).unwrap();

    let snapshot_restore =
        initialize_from_snapshot_or_replay(&snapshot_config, CoordinateMapper::accepting()).unwrap();
    let genesis_restore =
        initialize_from_snapshot_or_replay(&genesis_config, CoordinateMapper::accepting()).unwrap();

    assert_eq!(snapshot_restore.final_lsn, genesis_restore.final_lsn);
    assert_eq!(snapshot_restore.state, genesis_restore.state);
    assert_eq!(
        compute_state_root(&snapshot_restore.state),
        compute_state_root(&genesis_restore.state)
    );
    assert_eq!(
        snapshot_restore.source,
        RestorationSource::SnapshotWithDeltas {
            checkpoint_lsn: Lsn(2),
            deltas_applied: 2,
            final_lsn: Lsn(4),
        }
    );
    assert_eq!(
        genesis_restore.source,
        RestorationSource::FullReplay {
            final_lsn: Lsn(4),
        }
    );
}

#[test]
fn mid_delta_mapping_failure_cannot_mutate_live_state() {
    let payloads: &[&[u8]] = &[b"zero", b"one", b"two", b"three"];
    let config = test_config("atomic_delta_mapping");
    append_history(&config, payloads);
    let checkpoint_state = state_through(payloads, 1);
    write_snapshot_with_root(&config, Lsn(1), &checkpoint_state).unwrap();

    let live_state = seeded_live_state();
    let live_before = live_state.clone();
    let result = initialize_from_snapshot_or_replay(
        &config,
        CoordinateMapper::rejecting_at(Lsn(3)),
    );

    assert_eq!(
        result,
        Err(RestorationError::Mapping(MappingError::Rejected(Lsn(3))))
    );
    assert_eq!(live_state, live_before);
    assert_eq!(compute_state_root(&live_state), compute_state_root(&live_before));
}

#[test]
fn mid_genesis_application_failure_cannot_mutate_live_state() {
    let oversized = [7_u8; 65];
    let config = test_config("atomic_genesis_application");
    append_history(&config, &[b"first-success", &oversized]);

    let live_state = seeded_live_state();
    let live_before = live_state.clone();
    let result = initialize_from_snapshot_or_replay(&config, CoordinateMapper::accepting());

    assert!(matches!(result, Err(RestorationError::StateApplication(_))));
    assert_eq!(live_state, live_before);
    assert_eq!(compute_state_root(&live_state), compute_state_root(&live_before));
}

#[test]
fn structural_corruption_cannot_mutate_live_state() {
    let config = test_config("atomic_corruption");
    append_history(&config, &[b"first", b"second"]);

    let segment = fs::read_dir(&config.storage_root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("seg"))
        .unwrap();
    let mut bytes = fs::read(&segment).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    fs::write(&segment, bytes).unwrap();

    let live_state = seeded_live_state();
    let live_before = live_state.clone();
    let result = initialize_from_snapshot_or_replay(&config, CoordinateMapper::accepting());

    assert!(matches!(result, Err(RestorationError::Ledger(_))));
    assert_eq!(live_state, live_before);
    assert_eq!(compute_state_root(&live_state), compute_state_root(&live_before));
}
