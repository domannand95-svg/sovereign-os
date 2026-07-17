//! Read-only discovery of the highest fully validated ledger LSN.

use crate::scan::{enumerate_segments_strict, scan_segment_records};
use crate::{LedgerConfig, LedgerError, Lsn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailDiscovery {
    pub tail_lsn: Option<Lsn>,
    pub segments_scanned: usize,
    pub records_validated: usize,
}

pub fn discover_ledger_tail(config: &LedgerConfig) -> Result<TailDiscovery, LedgerError> {
    let segments = enumerate_segments_strict(config)?;
    let mut tail_lsn: Option<Lsn> = None;
    let mut records_validated = 0;

    for (index, (base_lsn, path)) in segments.iter().enumerate() {
        let expected_base = match tail_lsn {
            Some(previous) => previous.next()?,
            None => Lsn(0),
        };
        if *base_lsn != expected_base {
            return Err(LedgerError::LsnSequenceGap);
        }

        let (segment_tail, record_count) = scan_segment_records(path, config, *base_lsn)?;
        if record_count == 0 {
            if index + 1 != segments.len() {
                return Err(LedgerError::SegmentCorrupted);
            }
        } else {
            tail_lsn = segment_tail;
            records_validated += record_count;
        }
    }

    Ok(TailDiscovery {
        tail_lsn,
        segments_scanned: segments.len(),
        records_validated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventRecord, EventType};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn config(name: &str) -> LedgerConfig {
        let root =
            std::env::temp_dir().join(format!("sovereign_tail_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut config = LedgerConfig::new(root);
        config.max_segment_size = 4096 * 4;
        config.max_record_payload_size = 4096;
        config
    }

    fn write_segment(root: &Path, base: u64, lsns: &[u64]) -> PathBuf {
        fs::create_dir_all(root).unwrap();
        let path = root.join(format!("{base:016x}.seg"));
        let mut bytes = Vec::new();
        for lsn in lsns {
            let record = EventRecord {
                lsn: Lsn(*lsn),
                event_type: EventType::KernelDirective,
                payload: b"event",
                checksum: 0,
            };
            let mut encoded = [0_u8; 128];
            let written = record.encode_into(&mut encoded).unwrap();
            bytes.extend_from_slice(&encoded[..written]);
        }
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn missing_and_empty_directories_have_no_tail() {
        let config = config("empty");
        let result = discover_ledger_tail(&config).unwrap();
        assert_eq!(
            result,
            TailDiscovery {
                tail_lsn: None,
                segments_scanned: 0,
                records_validated: 0
            }
        );
        fs::create_dir_all(&config.storage_root).unwrap();
        assert_eq!(discover_ledger_tail(&config).unwrap(), result);
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn empty_genesis_segment_has_no_tail() {
        let config = config("empty_genesis");
        write_segment(&config.storage_root, 0, &[]);
        assert_eq!(
            discover_ledger_tail(&config).unwrap(),
            TailDiscovery {
                tail_lsn: None,
                segments_scanned: 1,
                records_validated: 0
            }
        );
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn populated_segment_reports_last_lsn() {
        let config = config("single");
        write_segment(&config.storage_root, 0, &[0, 1, 2]);
        assert_eq!(
            discover_ledger_tail(&config).unwrap(),
            TailDiscovery {
                tail_lsn: Some(Lsn(2)),
                segments_scanned: 1,
                records_validated: 3
            }
        );
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn trailing_empty_segment_preserves_previous_tail() {
        let config = config("trailing_empty");
        write_segment(&config.storage_root, 0, &[0, 1]);
        write_segment(&config.storage_root, 2, &[]);
        assert_eq!(
            discover_ledger_tail(&config).unwrap(),
            TailDiscovery {
                tail_lsn: Some(Lsn(1)),
                segments_scanned: 2,
                records_validated: 2
            }
        );
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn dense_multiple_segments_are_valid() {
        let config = config("dense");
        write_segment(&config.storage_root, 0, &[0]);
        write_segment(&config.storage_root, 1, &[1, 2]);
        assert_eq!(
            discover_ledger_tail(&config).unwrap(),
            TailDiscovery {
                tail_lsn: Some(Lsn(2)),
                segments_scanned: 2,
                records_validated: 3
            }
        );
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn segment_gap_is_rejected() {
        let config = config("gap");
        write_segment(&config.storage_root, 0, &[0]);
        write_segment(&config.storage_root, 2, &[2]);
        assert_eq!(
            discover_ledger_tail(&config),
            Err(LedgerError::LsnSequenceGap)
        );
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn empty_nonfinal_segment_is_corruption() {
        let config = config("empty_nonfinal");
        write_segment(&config.storage_root, 0, &[]);
        write_segment(&config.storage_root, 1, &[1]);
        assert_eq!(
            discover_ledger_tail(&config),
            Err(LedgerError::SegmentCorrupted)
        );
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn multiple_empty_segments_are_corruption() {
        let config = config("multiple_empty");
        write_segment(&config.storage_root, 0, &[]);
        write_segment(&config.storage_root, 1, &[]);
        assert_eq!(
            discover_ledger_tail(&config),
            Err(LedgerError::SegmentCorrupted)
        );
        let _ = fs::remove_dir_all(&config.storage_root);
    }
}
