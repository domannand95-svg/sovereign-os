//! CRC32C checksum helpers for ledger wire records.

/// Computes a CRC32C checksum over the supplied byte slice.
#[must_use]
pub fn crc32c(bytes: &[u8]) -> u32 {
    ::crc32c::crc32c(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32c_is_deterministic() {
        let payload = b"sovereign-ledger";
        assert_eq!(crc32c(payload), crc32c(payload));
    }

    #[test]
    fn crc32c_changes_when_payload_changes() {
        assert_ne!(crc32c(b"event-a"), crc32c(b"event-b"));
    }
}
