//! Canonical ledger event record wire format.

use crate::{checksum::crc32c, LedgerError, Lsn};

/// Number of bytes in a serialized record header before payload bytes begin.
pub const RECORD_HEADER_LEN: usize = 13;

/// Number of bytes in the trailing checksum field.
pub const RECORD_CHECKSUM_LEN: usize = 4;

/// Offset of the LSN field.
pub const LSN_OFFSET: usize = 0;

/// Offset of the event type discriminator.
pub const EVENT_TYPE_OFFSET: usize = 8;

/// Offset of the payload length field.
pub const PAYLOAD_LEN_OFFSET: usize = 9;

/// Offset at which payload bytes begin.
pub const PAYLOAD_OFFSET: usize = 13;

/// Explicit categorization of kernel and application-space event vectors.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum EventType {
    KernelDirective = 0x01,
    CapabilityPromotion = 0x02,
    RegistryMutation = 0x03,
    DiscoveryTelemetry = 0x04,
    PolicyEnforcement = 0x05,
}

impl EventType {
    /// Converts a stable byte discriminator into an event type.
    pub fn from_u8(value: u8) -> Result<Self, LedgerError> {
        match value {
            0x01 => Ok(Self::KernelDirective),
            0x02 => Ok(Self::CapabilityPromotion),
            0x03 => Ok(Self::RegistryMutation),
            0x04 => Ok(Self::DiscoveryTelemetry),
            0x05 => Ok(Self::PolicyEnforcement),
            _ => Err(LedgerError::UnsupportedVersion),
        }
    }

    /// Returns the stable byte discriminator.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Immutable zero-copy representation of a committed transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecord<'a> {
    pub lsn: Lsn,
    pub event_type: EventType,
    pub payload: &'a [u8],
    pub checksum: u32,
}

impl<'a> EventRecord<'a> {
    /// Computes the total serialized length for the current record.
    #[must_use]
    pub fn encoded_len(&self) -> usize {
        RECORD_HEADER_LEN + self.payload.len() + RECORD_CHECKSUM_LEN
    }

    /// Encodes this record into the supplied output buffer.
    ///
    /// The caller owns the buffer so the function can remain allocation-free.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, LedgerError> {
        let payload_len: u32 = self
            .payload
            .len()
            .try_into()
            .map_err(|_| LedgerError::StorageExhausted)?;

        let total_len = RECORD_HEADER_LEN
            .checked_add(self.payload.len())
            .and_then(|n| n.checked_add(RECORD_CHECKSUM_LEN))
            .ok_or(LedgerError::StorageExhausted)?;

        if out.len() < total_len {
            return Err(LedgerError::StorageExhausted);
        }

        out[LSN_OFFSET..EVENT_TYPE_OFFSET].copy_from_slice(&self.lsn.get().to_be_bytes());
        out[EVENT_TYPE_OFFSET] = self.event_type.as_u8();
        out[PAYLOAD_LEN_OFFSET..PAYLOAD_OFFSET].copy_from_slice(&payload_len.to_be_bytes());

        let payload_end = PAYLOAD_OFFSET + self.payload.len();
        out[PAYLOAD_OFFSET..payload_end].copy_from_slice(self.payload);

        let checksum = crc32c(&out[..payload_end]);
        out[payload_end..payload_end + RECORD_CHECKSUM_LEN]
            .copy_from_slice(&checksum.to_be_bytes());

        Ok(total_len)
    }

    /// Decodes and validates a record from a borrowed byte buffer.
    ///
    /// The payload is borrowed directly from the provided buffer.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, LedgerError> {
        if bytes.len() < RECORD_HEADER_LEN + RECORD_CHECKSUM_LEN {
            return Err(LedgerError::SegmentCorrupted);
        }

        let lsn = Lsn(u64::from_be_bytes(
            bytes[LSN_OFFSET..EVENT_TYPE_OFFSET]
                .try_into()
                .map_err(|_| LedgerError::SegmentCorrupted)?,
        ));

        let event_type = EventType::from_u8(bytes[EVENT_TYPE_OFFSET])?;

        let payload_len = u32::from_be_bytes(
            bytes[PAYLOAD_LEN_OFFSET..PAYLOAD_OFFSET]
                .try_into()
                .map_err(|_| LedgerError::SegmentCorrupted)?,
        ) as usize;

        let payload_end = PAYLOAD_OFFSET
            .checked_add(payload_len)
            .ok_or(LedgerError::SegmentCorrupted)?;

        let checksum_end = payload_end
            .checked_add(RECORD_CHECKSUM_LEN)
            .ok_or(LedgerError::SegmentCorrupted)?;

        if bytes.len() < checksum_end {
            return Err(LedgerError::SegmentCorrupted);
        }

        let expected = u32::from_be_bytes(
            bytes[payload_end..checksum_end]
                .try_into()
                .map_err(|_| LedgerError::SegmentCorrupted)?,
        );

        let actual = crc32c(&bytes[..payload_end]);

        if actual != expected {
            return Err(LedgerError::InvalidChecksum);
        }

        Ok(Self {
            lsn,
            event_type,
            payload: &bytes[PAYLOAD_OFFSET..payload_end],
            checksum: expected,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_expected_offsets() {
        assert_eq!(LSN_OFFSET, 0);
        assert_eq!(EVENT_TYPE_OFFSET, 8);
        assert_eq!(PAYLOAD_LEN_OFFSET, 9);
        assert_eq!(PAYLOAD_OFFSET, 13);
        assert_eq!(RECORD_HEADER_LEN, 13);
        assert_eq!(RECORD_CHECKSUM_LEN, 4);
    }

    #[test]
    fn event_type_discriminants_are_stable() {
        assert_eq!(EventType::KernelDirective.as_u8(), 0x01);
        assert_eq!(EventType::CapabilityPromotion.as_u8(), 0x02);
        assert_eq!(EventType::RegistryMutation.as_u8(), 0x03);
        assert_eq!(EventType::DiscoveryTelemetry.as_u8(), 0x04);
        assert_eq!(EventType::PolicyEnforcement.as_u8(), 0x05);
    }

    #[test]
    fn event_type_rejects_unknown_discriminator() {
        assert_eq!(
            EventType::from_u8(0xff),
            Err(LedgerError::UnsupportedVersion)
        );
    }

    #[test]
    fn encode_decode_round_trip_is_deterministic() {
        let record = EventRecord {
            lsn: Lsn(42),
            event_type: EventType::RegistryMutation,
            payload: b"payload",
            checksum: 0,
        };

        let mut buffer = [0_u8; 64];
        let written = record.encode_into(&mut buffer).unwrap();
        let decoded = EventRecord::decode(&buffer[..written]).unwrap();

        assert_eq!(decoded.lsn, Lsn(42));
        assert_eq!(decoded.event_type, EventType::RegistryMutation);
        assert_eq!(decoded.payload, b"payload");
        assert_eq!(
            decoded.checksum,
            crc32c(&buffer[..PAYLOAD_OFFSET + b"payload".len()])
        );
    }

    #[test]
    fn multi_byte_fields_are_big_endian() {
        let record = EventRecord {
            lsn: Lsn(0x0102_0304_0506_0708),
            event_type: EventType::KernelDirective,
            payload: b"abc",
            checksum: 0,
        };

        let mut buffer = [0_u8; 64];
        let written = record.encode_into(&mut buffer).unwrap();

        assert_eq!(&buffer[0..8], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(&buffer[9..13], &[0, 0, 0, 3]);
        assert_eq!(written, RECORD_HEADER_LEN + 3 + RECORD_CHECKSUM_LEN);
    }

    #[test]
    fn decode_rejects_checksum_mismatch() {
        let record = EventRecord {
            lsn: Lsn(1),
            event_type: EventType::PolicyEnforcement,
            payload: b"secure",
            checksum: 0,
        };

        let mut buffer = [0_u8; 64];
        let written = record.encode_into(&mut buffer).unwrap();

        buffer[PAYLOAD_OFFSET] ^= 0xff;

        assert_eq!(
            EventRecord::decode(&buffer[..written]),
            Err(LedgerError::InvalidChecksum)
        );
    }

    #[test]
    fn decode_rejects_truncated_record() {
        assert_eq!(
            EventRecord::decode(&[0_u8; 4]),
            Err(LedgerError::SegmentCorrupted)
        );
    }
}
