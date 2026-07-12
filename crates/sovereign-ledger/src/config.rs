//! Runtime limits and storage allocation boundaries for the ledger engine.

use std::path::PathBuf;

use crate::error::LedgerError;

/// Fixed serialized overhead: 13-byte header plus 4-byte CRC32C checksum.
pub const MAX_RECORD_OVERHEAD: usize =
    crate::record::RECORD_HEADER_LEN + crate::record::RECORD_CHECKSUM_LEN;

/// Runtime allocation thresholds and storage root for the append-only ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerConfig {
    /// Root directory for ledger segment files.
    pub storage_root: PathBuf,
    /// Maximum size in bytes of an individual segment.
    pub max_segment_size: usize,
    /// Maximum size in bytes of an individual record payload.
    pub max_record_payload_size: usize,
}

impl LedgerConfig {
    /// Creates a configuration using repair-baseline defaults.
    ///
    /// These defaults were introduced during repository repair because the
    /// historical implementation was committed only as a placeholder.
    pub fn new(storage_root: PathBuf) -> Self {
        Self {
            storage_root,
            max_segment_size: 16_384,
            max_record_payload_size: 4_096,
        }
    }

    /// Validates that one maximum-sized record can fit within a segment.
    pub fn validate(&self) -> Result<(), LedgerError> {
        let record_size = self
            .max_record_payload_size
            .checked_add(MAX_RECORD_OVERHEAD)
            .ok_or(LedgerError::WriteViolation)?;

        if record_size > self.max_segment_size {
            return Err(LedgerError::WriteViolation);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = LedgerConfig::new(PathBuf::from("ledger"));

        assert_eq!(config.max_segment_size, 16_384);
        assert_eq!(config.max_record_payload_size, 4_096);
        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn rejects_record_larger_than_segment() {
        let mut config = LedgerConfig::new(PathBuf::from("ledger"));
        config.max_segment_size = MAX_RECORD_OVERHEAD;
        config.max_record_payload_size = 1;

        assert_eq!(config.validate(), Err(LedgerError::WriteViolation));
    }

    #[test]
    fn rejects_overflowing_record_size() {
        let mut config = LedgerConfig::new(PathBuf::from("ledger"));
        config.max_segment_size = usize::MAX;
        config.max_record_payload_size = usize::MAX;

        assert_eq!(config.validate(), Err(LedgerError::WriteViolation));
    }
}
