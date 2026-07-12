use sovereign_ledger::{
    EventType, LedgerAppendEngine, LedgerConfig, LedgerError, LedgerSnapshotManager, Lsn,
    ReplayIterator,
};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

fn workspace(case_name: &str) -> LedgerConfig {
    let path = std::env::temp_dir().join(format!(
        "sovereign_forensic_{}_{}",
        case_name,
        std::process::id()
    ));

    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();

    let mut config = LedgerConfig::new(path);
    config.max_segment_size = 4096 * 4;
    config.max_record_payload_size = 4096;
    config
}

fn first_segment_path(config: &LedgerConfig) -> PathBuf {
    fs::read_dir(&config.storage_root)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "seg"))
        .unwrap()
}

fn flip_byte(path: &PathBuf, offset: u64) {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();

    file.seek(SeekFrom::Start(offset)).unwrap();

    let mut byte = [0_u8; 1];
    file.read_exact(&mut byte).unwrap();

    byte[0] ^= 0xFF;

    file.seek(SeekFrom::Start(offset)).unwrap();
    file.write_all(&byte).unwrap();
    file.sync_all().unwrap();
}

#[test]
fn power_loss_partial_header_fails_closed() {
    let config = workspace("partial_header");

    {
        let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        engine
            .append(EventType::KernelDirective, b"authoritative_genesis_block")
            .unwrap();
        engine.flush().unwrap();
    }

    let segment_path = first_segment_path(&config);

    {
        let mut file = OpenOptions::new().append(true).open(&segment_path).unwrap();

        file.write_all(&[0, 0, 0, 0, 1]).unwrap();
        file.sync_all().unwrap();
    }

    let mut replay = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();

    let first = replay.next_record().unwrap().unwrap();
    assert_eq!(first.payload, b"authoritative_genesis_block");

    assert_eq!(
        replay.next_record().unwrap().unwrap_err(),
        LedgerError::SegmentCorrupted
    );

    let _ = fs::remove_dir_all(&config.storage_root);
}

#[test]
fn power_loss_partial_payload_fails_closed() {
    let config = workspace("partial_payload");

    let segment_path = config.storage_root.join(format!("{:016x}.seg", 0));

    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&segment_path)
            .unwrap();

        let mut header = [0_u8; 13];
        header[0..8].copy_from_slice(&0_u64.to_be_bytes());
        header[8] = EventType::KernelDirective.as_u8();
        header[9..13].copy_from_slice(&1024_u32.to_be_bytes());

        file.write_all(&header).unwrap();
        file.write_all(b"short_payload").unwrap();
        file.sync_all().unwrap();
    }

    let mut replay = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();

    assert_eq!(
        replay.next_record().unwrap().unwrap_err(),
        LedgerError::SegmentCorrupted
    );

    let _ = fs::remove_dir_all(&config.storage_root);
}

#[test]
fn media_degradation_bit_flipped_payload_fails_closed() {
    let config = workspace("bit_flipped_payload");

    {
        let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        engine
            .append(
                EventType::CapabilityPromotion,
                b"uncorrupted_payload_string",
            )
            .unwrap();
        engine.flush().unwrap();
    }

    let segment_path = first_segment_path(&config);

    flip_byte(&segment_path, 15);

    let mut replay = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();

    assert_eq!(
        replay.next_record().unwrap().unwrap_err(),
        LedgerError::InvalidChecksum
    );

    let _ = fs::remove_dir_all(&config.storage_root);
}

#[test]
fn media_degradation_bit_flipped_lsn_fails_closed() {
    let config = workspace("bit_flipped_lsn");

    {
        let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        engine
            .append(EventType::RegistryMutation, b"stable")
            .unwrap();
        engine.flush().unwrap();
    }

    let segment_path = first_segment_path(&config);

    flip_byte(&segment_path, 0);

    let mut replay = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();

    assert_eq!(
        replay.next_record().unwrap().unwrap_err(),
        LedgerError::InvalidChecksum
    );

    let _ = fs::remove_dir_all(&config.storage_root);
}

#[test]
fn media_degradation_bit_flipped_event_type_fails_closed() {
    let config = workspace("bit_flipped_event_type");

    {
        let mut engine = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        engine
            .append(EventType::RegistryMutation, b"system_metadata_stream")
            .unwrap();
        engine.flush().unwrap();
    }

    let segment_path = first_segment_path(&config);

    flip_byte(&segment_path, 8);

    let mut replay = ReplayIterator::bootstrap(config.clone(), Lsn(0)).unwrap();

    assert_eq!(
        replay.next_record().unwrap().unwrap_err(),
        LedgerError::UnsupportedVersion
    );

    let _ = fs::remove_dir_all(&config.storage_root);
}

#[test]
fn snapshot_payload_corruption_fails_closed() {
    let config = workspace("snapshot_payload_corruption");

    let path = LedgerSnapshotManager::write_snapshot(
        &config,
        Lsn(12),
        [0xAA; 32],
        b"trusted_snapshot_payload",
    )
    .unwrap();

    flip_byte(&path, 44);

    assert_eq!(
        LedgerSnapshotManager::read_snapshot(&config, Lsn(12)).unwrap_err(),
        LedgerError::InvalidChecksum
    );

    let _ = fs::remove_dir_all(&config.storage_root);
}

#[test]
fn snapshot_state_root_corruption_fails_closed() {
    let config = workspace("snapshot_state_root_corruption");

    let path = LedgerSnapshotManager::write_snapshot(
        &config,
        Lsn(13),
        [0xBB; 32],
        b"trusted_snapshot_payload",
    )
    .unwrap();

    flip_byte(&path, 8);

    assert_eq!(
        LedgerSnapshotManager::read_snapshot(&config, Lsn(13)).unwrap_err(),
        LedgerError::InvalidChecksum
    );

    let _ = fs::remove_dir_all(&config.storage_root);
}

#[test]
fn snapshot_payload_length_corruption_fails_closed() {
    let config = workspace("snapshot_payload_len_corruption");

    let path = LedgerSnapshotManager::write_snapshot(
        &config,
        Lsn(14),
        [0xCC; 32],
        b"trusted_snapshot_payload",
    )
    .unwrap();

    flip_byte(&path, 40);

    assert_eq!(
        LedgerSnapshotManager::read_snapshot(&config, Lsn(14)).unwrap_err(),
        LedgerError::SegmentCorrupted
    );

    let _ = fs::remove_dir_all(&config.storage_root);
}
