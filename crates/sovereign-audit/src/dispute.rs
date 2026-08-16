use crate::{RecordId, MAX_EVIDENCE_PAYLOAD_LEN, MAX_LIST_ITEMS};
use sovereign_registry::IdentityId;
use std::collections::HashSet;
use std::fmt;

const ID_LEN: usize = 32;
const MIN_POSITIONS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DisputeStatus {
    Open = 0x00,
    UnderReview = 0x01,
    Resolved = 0x02,
    Withdrawn = 0x03,
}

impl DisputeStatus {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self, DisputeError> {
        match value {
            0x00 => Ok(Self::Open),
            0x01 => Ok(Self::UnderReview),
            0x02 => Ok(Self::Resolved),
            0x03 => Ok(Self::Withdrawn),
            _ => Err(DisputeError::UnknownStatus(value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisputePayload {
    disputed_id: RecordId,
    position_ids: Vec<RecordId>,
    opened_by: IdentityId,
    status: DisputeStatus,
    resolution_id: Option<RecordId>,
}

impl DisputePayload {
    pub fn new(
        disputed_id: RecordId,
        position_ids: Vec<RecordId>,
        opened_by: IdentityId,
        status: DisputeStatus,
        resolution_id: Option<RecordId>,
    ) -> Result<Self, DisputeError> {
        validate_record_id(disputed_id)?;
        validate_position_ids(&position_ids)?;

        if opened_by.is_zero() {
            return Err(DisputeError::ZeroIdentity);
        }

        match (status, resolution_id) {
            (DisputeStatus::Resolved, None) => {
                return Err(DisputeError::ResolutionRequired);
            }
            (DisputeStatus::Resolved, Some(id)) => {
                validate_record_id(id)?;
            }
            (
                DisputeStatus::Open | DisputeStatus::UnderReview | DisputeStatus::Withdrawn,
                Some(_),
            ) => {
                return Err(DisputeError::ResolutionForbidden);
            }
            (DisputeStatus::Open | DisputeStatus::UnderReview | DisputeStatus::Withdrawn, None) => {
            }
        }

        let payload = Self {
            disputed_id,
            position_ids,
            opened_by,
            status,
            resolution_id,
        };

        if payload.encoded_len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(DisputeError::PayloadTooLarge);
        }

        Ok(payload)
    }

    pub const fn disputed_id(&self) -> RecordId {
        self.disputed_id
    }

    pub fn position_ids(&self) -> &[RecordId] {
        &self.position_ids
    }

    pub const fn opened_by(&self) -> IdentityId {
        self.opened_by
    }

    pub const fn status(&self) -> DisputeStatus {
        self.status
    }

    pub const fn resolution_id(&self) -> Option<RecordId> {
        self.resolution_id
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(self.encoded_len());

        encoded.extend_from_slice(self.disputed_id.as_bytes());

        encoded.extend_from_slice(&(self.position_ids.len() as u16).to_be_bytes());
        for id in &self.position_ids {
            encoded.extend_from_slice(id.as_bytes());
        }

        encoded.extend_from_slice(self.opened_by.as_bytes());
        encoded.push(self.status.as_u8());

        match self.resolution_id {
            None => encoded.push(0),
            Some(id) => {
                encoded.push(1);
                encoded.extend_from_slice(id.as_bytes());
            }
        }

        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DisputeError> {
        if bytes.len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(DisputeError::PayloadTooLarge);
        }

        let mut decoder = Decoder::new(bytes);

        let disputed_id = decoder.read_record_id()?;
        let position_ids = decoder.read_position_ids()?;
        let opened_by = decoder.read_identity_id()?;
        let status = DisputeStatus::from_u8(decoder.read_u8()?)?;

        let resolution_id = match decoder.read_u8()? {
            0 => None,
            1 => Some(decoder.read_record_id()?),
            marker => return Err(DisputeError::InvalidOptionalMarker(marker)),
        };

        if !decoder.is_finished() {
            return Err(DisputeError::TrailingBytes);
        }

        Self::new(disputed_id, position_ids, opened_by, status, resolution_id)
    }

    fn encoded_len(&self) -> usize {
        ID_LEN
            + 2
            + (self.position_ids.len() * ID_LEN)
            + ID_LEN
            + 1
            + 1
            + self.resolution_id.map_or(0, |_| ID_LEN)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisputeError {
    ZeroRecordId,
    ZeroIdentity,
    TooFewPositions,
    DuplicateRecordId,
    TooManyItems,
    UnknownStatus(u8),
    InvalidOptionalMarker(u8),
    ResolutionRequired,
    ResolutionForbidden,
    PayloadTooLarge,
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for DisputeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DisputeError {}

fn validate_record_id(id: RecordId) -> Result<(), DisputeError> {
    if id.is_zero() {
        Err(DisputeError::ZeroRecordId)
    } else {
        Ok(())
    }
}

fn validate_position_ids(ids: &[RecordId]) -> Result<(), DisputeError> {
    if ids.len() < MIN_POSITIONS {
        return Err(DisputeError::TooFewPositions);
    }

    if ids.len() > MAX_LIST_ITEMS {
        return Err(DisputeError::TooManyItems);
    }

    let mut seen = HashSet::with_capacity(ids.len());

    for id in ids {
        validate_record_id(*id)?;

        if !seen.insert(*id) {
            return Err(DisputeError::DuplicateRecordId);
        }
    }

    Ok(())
}

struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn read_u8(&mut self) -> Result<u8, DisputeError> {
        let value = *self
            .bytes
            .get(self.cursor)
            .ok_or(DisputeError::TruncatedEncoding)?;

        self.cursor += 1;
        Ok(value)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], DisputeError> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(DisputeError::TruncatedEncoding)?;

        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(DisputeError::TruncatedEncoding)?;

        let mut value = [0_u8; N];
        value.copy_from_slice(bytes);
        self.cursor = end;

        Ok(value)
    }

    fn read_record_id(&mut self) -> Result<RecordId, DisputeError> {
        Ok(RecordId::from_bytes(self.read_array()?))
    }

    fn read_identity_id(&mut self) -> Result<IdentityId, DisputeError> {
        Ok(IdentityId::from_bytes(self.read_array()?))
    }

    fn read_position_ids(&mut self) -> Result<Vec<RecordId>, DisputeError> {
        let count = u16::from_be_bytes(self.read_array()?) as usize;

        if count > MAX_LIST_ITEMS {
            return Err(DisputeError::TooManyItems);
        }

        if self.bytes.len().saturating_sub(self.cursor) < count * ID_LEN {
            return Err(DisputeError::TruncatedEncoding);
        }

        let mut ids = Vec::with_capacity(count);

        for _ in 0..count {
            ids.push(self.read_record_id()?);
        }

        Ok(ids)
    }

    fn is_finished(&self) -> bool {
        self.cursor == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvidenceRecord, RecordKind};

    fn record_id(seed: u8) -> RecordId {
        RecordId::from_bytes([seed; ID_LEN])
    }

    fn identity(seed: u8) -> IdentityId {
        IdentityId::from_bytes([seed; ID_LEN])
    }

    fn dispute() -> DisputePayload {
        DisputePayload::new(
            record_id(1),
            vec![record_id(2), record_id(3)],
            identity(4),
            DisputeStatus::Resolved,
            Some(record_id(5)),
        )
        .unwrap()
    }

    fn governed_record(payload: DisputePayload) -> EvidenceRecord {
        EvidenceRecord::new(
            RecordKind::Dispute,
            identity(9),
            identity(10),
            identity(11),
            vec![record_id(12)],
            payload.encode(),
        )
        .unwrap()
    }

    #[test]
    fn round_trip_preserves_every_field_and_status() {
        let value = dispute();

        assert_eq!(DisputePayload::decode(&value.encode()).unwrap(), value);
        assert_eq!(value.disputed_id(), record_id(1));
        assert_eq!(value.position_ids(), &[record_id(2), record_id(3)]);
        assert_eq!(value.opened_by(), identity(4));
        assert_eq!(value.status(), DisputeStatus::Resolved);
        assert_eq!(value.resolution_id(), Some(record_id(5)));

        for status in [
            DisputeStatus::Open,
            DisputeStatus::UnderReview,
            DisputeStatus::Withdrawn,
        ] {
            let value = DisputePayload::new(
                record_id(1),
                vec![record_id(2), record_id(3)],
                identity(4),
                status,
                None,
            )
            .unwrap();

            assert_eq!(DisputePayload::decode(&value.encode()).unwrap(), value);
            assert_eq!(value.status(), status);
            assert_eq!(value.resolution_id(), None);
        }

        let resolved = DisputePayload::new(
            record_id(1),
            vec![record_id(2), record_id(3)],
            identity(4),
            DisputeStatus::Resolved,
            Some(record_id(5)),
        )
        .unwrap();

        assert_eq!(
            DisputePayload::decode(&resolved.encode()).unwrap(),
            resolved
        );
    }

    #[test]
    fn minimal_fixed_vector_is_stable_and_big_endian() {
        let value = DisputePayload::new(
            record_id(1),
            vec![record_id(2), record_id(3)],
            identity(4),
            DisputeStatus::Open,
            None,
        )
        .unwrap();

        let mut expected = vec![1; ID_LEN];
        expected.extend_from_slice(&2_u16.to_be_bytes());
        expected.extend_from_slice(&[2; ID_LEN]);
        expected.extend_from_slice(&[3; ID_LEN]);
        expected.extend_from_slice(&[4; ID_LEN]);
        expected.push(DisputeStatus::Open.as_u8());
        expected.push(0);

        assert_eq!(expected.len(), 132);
        assert_eq!(value.encode(), expected);
        assert_eq!(DisputePayload::decode(&expected).unwrap(), value);
    }

    #[test]
    fn governed_record_identity_is_stable() {
        let record = governed_record(dispute());

        assert_eq!(
            record.id().to_string(),
            "29cbf3618d488d0c3e0cc4f025ad76060bc928d3d786b6fad6c7051b85d9c54f"
        );

        let decoded = EvidenceRecord::decode(&record.encode()).unwrap();

        assert_eq!(decoded.id(), record.id());
        assert_eq!(
            DisputePayload::decode(decoded.payload()).unwrap(),
            dispute()
        );
    }

    #[test]
    fn required_fields_position_bounds_and_uniqueness_fail_closed() {
        let make = |disputed_id, position_ids, opened_by, status, resolution_id| {
            DisputePayload::new(disputed_id, position_ids, opened_by, status, resolution_id)
        };

        assert_eq!(
            make(
                RecordId::from_bytes([0; ID_LEN]),
                vec![record_id(2), record_id(3)],
                identity(4),
                DisputeStatus::Open,
                None,
            )
            .unwrap_err(),
            DisputeError::ZeroRecordId
        );

        assert_eq!(
            make(record_id(1), vec![], identity(4), DisputeStatus::Open, None,).unwrap_err(),
            DisputeError::TooFewPositions
        );

        assert_eq!(
            make(
                record_id(1),
                vec![record_id(2)],
                identity(4),
                DisputeStatus::Open,
                None,
            )
            .unwrap_err(),
            DisputeError::TooFewPositions
        );

        assert_eq!(
            make(
                record_id(1),
                vec![RecordId::from_bytes([0; ID_LEN]), record_id(3),],
                identity(4),
                DisputeStatus::Open,
                None,
            )
            .unwrap_err(),
            DisputeError::ZeroRecordId
        );

        assert_eq!(
            make(
                record_id(1),
                vec![record_id(2), record_id(2)],
                identity(4),
                DisputeStatus::Open,
                None,
            )
            .unwrap_err(),
            DisputeError::DuplicateRecordId
        );

        assert_eq!(
            make(
                record_id(1),
                vec![record_id(2), record_id(3)],
                IdentityId::from_bytes([0; ID_LEN]),
                DisputeStatus::Open,
                None,
            )
            .unwrap_err(),
            DisputeError::ZeroIdentity
        );

        assert!(make(
            record_id(1),
            vec![record_id(2), record_id(3)],
            identity(4),
            DisputeStatus::Open,
            None,
        )
        .is_ok());

        assert!(make(
            record_id(1),
            (1..=MAX_LIST_ITEMS)
                .map(|value| record_id(value as u8))
                .collect(),
            identity(4),
            DisputeStatus::UnderReview,
            None,
        )
        .is_ok());

        assert_eq!(
            make(
                record_id(1),
                (1..=MAX_LIST_ITEMS + 1)
                    .map(|value| record_id(value as u8))
                    .collect(),
                identity(4),
                DisputeStatus::UnderReview,
                None,
            )
            .unwrap_err(),
            DisputeError::TooManyItems
        );
    }

    #[test]
    fn declared_position_order_is_preserved() {
        let first = DisputePayload::new(
            record_id(1),
            vec![record_id(2), record_id(3)],
            identity(4),
            DisputeStatus::Open,
            None,
        )
        .unwrap();

        let second = DisputePayload::new(
            record_id(1),
            vec![record_id(3), record_id(2)],
            identity(4),
            DisputeStatus::Open,
            None,
        )
        .unwrap();

        assert_eq!(first.position_ids(), &[record_id(2), record_id(3)]);
        assert_eq!(second.position_ids(), &[record_id(3), record_id(2)]);
        assert_ne!(first.encode(), second.encode());
    }

    #[test]
    fn status_and_resolution_coherence_fail_closed() {
        assert_eq!(
            DisputePayload::new(
                record_id(1),
                vec![record_id(2), record_id(3)],
                identity(4),
                DisputeStatus::Resolved,
                None,
            )
            .unwrap_err(),
            DisputeError::ResolutionRequired
        );

        assert_eq!(
            DisputePayload::new(
                record_id(1),
                vec![record_id(2), record_id(3)],
                identity(4),
                DisputeStatus::Resolved,
                Some(RecordId::from_bytes([0; ID_LEN])),
            )
            .unwrap_err(),
            DisputeError::ZeroRecordId
        );

        for status in [
            DisputeStatus::Open,
            DisputeStatus::UnderReview,
            DisputeStatus::Withdrawn,
        ] {
            assert_eq!(
                DisputePayload::new(
                    record_id(1),
                    vec![record_id(2), record_id(3)],
                    identity(4),
                    status,
                    Some(record_id(5)),
                )
                .unwrap_err(),
                DisputeError::ResolutionForbidden
            );
        }

        assert!(DisputePayload::new(
            record_id(1),
            vec![record_id(2), record_id(3)],
            identity(4),
            DisputeStatus::Resolved,
            Some(record_id(5)),
        )
        .is_ok());
    }

    #[test]
    fn decoder_rejects_malformed_counts_statuses_and_optional_markers() {
        let minimal = DisputePayload::new(
            record_id(1),
            vec![record_id(2), record_id(3)],
            identity(4),
            DisputeStatus::Open,
            None,
        )
        .unwrap();

        let mut excessive_count = minimal.encode();
        excessive_count[ID_LEN..ID_LEN + 2]
            .copy_from_slice(&((MAX_LIST_ITEMS + 1) as u16).to_be_bytes());

        assert_eq!(
            DisputePayload::decode(&excessive_count).unwrap_err(),
            DisputeError::TooManyItems
        );

        let mut too_few = Vec::new();
        too_few.extend_from_slice(record_id(1).as_bytes());
        too_few.extend_from_slice(&1_u16.to_be_bytes());
        too_few.extend_from_slice(record_id(2).as_bytes());
        too_few.extend_from_slice(identity(4).as_bytes());
        too_few.push(DisputeStatus::Open.as_u8());
        too_few.push(0);

        assert_eq!(
            DisputePayload::decode(&too_few).unwrap_err(),
            DisputeError::TooFewPositions
        );

        let status_offset = ID_LEN + 2 + (2 * ID_LEN) + ID_LEN;
        let marker_offset = status_offset + 1;

        let mut unknown_status = minimal.encode();
        unknown_status[status_offset] = 0xff;

        assert_eq!(
            DisputePayload::decode(&unknown_status).unwrap_err(),
            DisputeError::UnknownStatus(0xff)
        );

        let mut invalid_marker = minimal.encode();
        invalid_marker[marker_offset] = 0x02;

        assert_eq!(
            DisputePayload::decode(&invalid_marker).unwrap_err(),
            DisputeError::InvalidOptionalMarker(0x02)
        );

        let mut resolved_without_resolution = minimal.encode();
        resolved_without_resolution[status_offset] = DisputeStatus::Resolved.as_u8();

        assert_eq!(
            DisputePayload::decode(&resolved_without_resolution).unwrap_err(),
            DisputeError::ResolutionRequired
        );

        let mut open_with_resolution = minimal.encode();
        open_with_resolution[marker_offset] = 1;
        open_with_resolution.extend_from_slice(record_id(5).as_bytes());

        assert_eq!(
            DisputePayload::decode(&open_with_resolution).unwrap_err(),
            DisputeError::ResolutionForbidden
        );
    }

    #[test]
    fn every_truncation_trailing_and_oversize_fails_closed() {
        let encoded = dispute().encode();

        for end in 0..encoded.len() {
            assert!(
                DisputePayload::decode(&encoded[..end]).is_err(),
                "accepted truncation at byte {end}"
            );
        }

        let mut trailing = encoded;
        trailing.push(0);

        assert_eq!(
            DisputePayload::decode(&trailing).unwrap_err(),
            DisputeError::TrailingBytes
        );

        assert_eq!(
            DisputePayload::decode(&vec![0; MAX_EVIDENCE_PAYLOAD_LEN + 1]).unwrap_err(),
            DisputeError::PayloadTooLarge
        );
    }

    #[test]
    fn every_committed_payload_field_changes_governed_identity() {
        let make_id = |payload: DisputePayload| governed_record(payload).id();

        let baseline = make_id(
            DisputePayload::new(
                record_id(1),
                vec![record_id(2), record_id(3)],
                identity(4),
                DisputeStatus::Resolved,
                Some(record_id(5)),
            )
            .unwrap(),
        );

        let changed_disputed = make_id(
            DisputePayload::new(
                record_id(6),
                vec![record_id(2), record_id(3)],
                identity(4),
                DisputeStatus::Resolved,
                Some(record_id(5)),
            )
            .unwrap(),
        );

        let changed_position = make_id(
            DisputePayload::new(
                record_id(1),
                vec![record_id(2), record_id(7)],
                identity(4),
                DisputeStatus::Resolved,
                Some(record_id(5)),
            )
            .unwrap(),
        );

        let changed_position_order = make_id(
            DisputePayload::new(
                record_id(1),
                vec![record_id(3), record_id(2)],
                identity(4),
                DisputeStatus::Resolved,
                Some(record_id(5)),
            )
            .unwrap(),
        );

        let changed_opener = make_id(
            DisputePayload::new(
                record_id(1),
                vec![record_id(2), record_id(3)],
                identity(6),
                DisputeStatus::Resolved,
                Some(record_id(5)),
            )
            .unwrap(),
        );

        let changed_resolution = make_id(
            DisputePayload::new(
                record_id(1),
                vec![record_id(2), record_id(3)],
                identity(4),
                DisputeStatus::Resolved,
                Some(record_id(6)),
            )
            .unwrap(),
        );

        assert_ne!(baseline, changed_disputed);
        assert_ne!(baseline, changed_position);
        assert_ne!(baseline, changed_position_order);
        assert_ne!(baseline, changed_opener);
        assert_ne!(baseline, changed_resolution);

        let open = make_id(
            DisputePayload::new(
                record_id(1),
                vec![record_id(2), record_id(3)],
                identity(4),
                DisputeStatus::Open,
                None,
            )
            .unwrap(),
        );

        let under_review = make_id(
            DisputePayload::new(
                record_id(1),
                vec![record_id(2), record_id(3)],
                identity(4),
                DisputeStatus::UnderReview,
                None,
            )
            .unwrap(),
        );

        assert_ne!(open, under_review);
    }
}
