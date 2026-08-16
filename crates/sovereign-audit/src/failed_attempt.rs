use crate::{RecordId, MAX_EVIDENCE_PAYLOAD_LEN, MAX_LIST_ITEMS, MAX_TEXT_FIELD_LEN};
use std::collections::HashSet;
use std::fmt;

const ID_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FailureKind {
    InvalidInput = 0x00,
    MethodFailure = 0x01,
    ToolFailure = 0x02,
    Timeout = 0x03,
    Inconclusive = 0x04,
    PolicyDenial = 0x05,
}

impl FailureKind {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self, FailedAttemptError> {
        match value {
            0x00 => Ok(Self::InvalidInput),
            0x01 => Ok(Self::MethodFailure),
            0x02 => Ok(Self::ToolFailure),
            0x03 => Ok(Self::Timeout),
            0x04 => Ok(Self::Inconclusive),
            0x05 => Ok(Self::PolicyDenial),
            _ => Err(FailedAttemptError::UnknownFailureKind(value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailedAttemptPayload {
    objective_id: RecordId,
    method_id: RecordId,
    failure_kind: FailureKind,
    observed_result: String,
    evidence_ids: Vec<RecordId>,
    retry_of: Option<RecordId>,
}

impl FailedAttemptPayload {
    pub fn new(
        objective_id: RecordId,
        method_id: RecordId,
        failure_kind: FailureKind,
        observed_result: String,
        evidence_ids: Vec<RecordId>,
        retry_of: Option<RecordId>,
    ) -> Result<Self, FailedAttemptError> {
        validate_record_id(objective_id)?;
        validate_record_id(method_id)?;
        validate_text(&observed_result)?;
        validate_record_ids(&evidence_ids)?;
        if let Some(retry_of) = retry_of {
            validate_record_id(retry_of)?;
        }
        let payload = Self {
            objective_id,
            method_id,
            failure_kind,
            observed_result,
            evidence_ids,
            retry_of,
        };
        if payload.encoded_len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(FailedAttemptError::PayloadTooLarge);
        }
        Ok(payload)
    }

    pub const fn objective_id(&self) -> RecordId {
        self.objective_id
    }
    pub const fn method_id(&self) -> RecordId {
        self.method_id
    }
    pub const fn failure_kind(&self) -> FailureKind {
        self.failure_kind
    }
    pub fn observed_result(&self) -> &str {
        &self.observed_result
    }
    pub fn evidence_ids(&self) -> &[RecordId] {
        &self.evidence_ids
    }
    pub const fn retry_of(&self) -> Option<RecordId> {
        self.retry_of
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(self.encoded_len());
        encoded.extend_from_slice(self.objective_id.as_bytes());
        encoded.extend_from_slice(self.method_id.as_bytes());
        encoded.push(self.failure_kind.as_u8());
        encode_text(&mut encoded, &self.observed_result);
        encoded.extend_from_slice(&(self.evidence_ids.len() as u16).to_be_bytes());
        for id in &self.evidence_ids {
            encoded.extend_from_slice(id.as_bytes());
        }
        match self.retry_of {
            None => encoded.push(0),
            Some(id) => {
                encoded.push(1);
                encoded.extend_from_slice(id.as_bytes());
            }
        }
        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, FailedAttemptError> {
        if bytes.len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(FailedAttemptError::PayloadTooLarge);
        }
        let mut decoder = Decoder::new(bytes);
        let objective_id = decoder.read_record_id()?;
        let method_id = decoder.read_record_id()?;
        let failure_kind = FailureKind::from_u8(decoder.read_u8()?)?;
        let observed_result = decoder.read_text()?;
        let evidence_ids = decoder.read_record_ids()?;
        let retry_of = match decoder.read_u8()? {
            0 => None,
            1 => Some(decoder.read_record_id()?),
            marker => return Err(FailedAttemptError::InvalidOptionalMarker(marker)),
        };
        if !decoder.is_finished() {
            return Err(FailedAttemptError::TrailingBytes);
        }
        Self::new(
            objective_id,
            method_id,
            failure_kind,
            observed_result,
            evidence_ids,
            retry_of,
        )
    }

    fn encoded_len(&self) -> usize {
        (2 * ID_LEN)
            + 1
            + 4
            + self.observed_result.len()
            + 2
            + (self.evidence_ids.len() * ID_LEN)
            + 1
            + self.retry_of.map_or(0, |_| ID_LEN)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailedAttemptError {
    EmptyText,
    NulText,
    TextTooLarge,
    InvalidUtf8,
    ZeroRecordId,
    DuplicateRecordId,
    TooManyItems,
    UnknownFailureKind(u8),
    InvalidOptionalMarker(u8),
    PayloadTooLarge,
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for FailedAttemptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FailedAttemptError {}

fn validate_text(value: &str) -> Result<(), FailedAttemptError> {
    if value.is_empty() {
        return Err(FailedAttemptError::EmptyText);
    }
    if value.len() > MAX_TEXT_FIELD_LEN {
        return Err(FailedAttemptError::TextTooLarge);
    }
    if value.contains('\0') {
        return Err(FailedAttemptError::NulText);
    }
    Ok(())
}

fn validate_record_id(id: RecordId) -> Result<(), FailedAttemptError> {
    if id.is_zero() {
        Err(FailedAttemptError::ZeroRecordId)
    } else {
        Ok(())
    }
}

fn validate_record_ids(ids: &[RecordId]) -> Result<(), FailedAttemptError> {
    if ids.len() > MAX_LIST_ITEMS {
        return Err(FailedAttemptError::TooManyItems);
    }
    let mut seen = HashSet::with_capacity(ids.len());
    for id in ids {
        validate_record_id(*id)?;
        if !seen.insert(*id) {
            return Err(FailedAttemptError::DuplicateRecordId);
        }
    }
    Ok(())
}

fn encode_text(encoded: &mut Vec<u8>, value: &str) {
    encoded.extend_from_slice(&(value.len() as u32).to_be_bytes());
    encoded.extend_from_slice(value.as_bytes());
}

struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }
    fn read_u8(&mut self) -> Result<u8, FailedAttemptError> {
        let value = *self
            .bytes
            .get(self.cursor)
            .ok_or(FailedAttemptError::TruncatedEncoding)?;
        self.cursor += 1;
        Ok(value)
    }
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], FailedAttemptError> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(FailedAttemptError::TruncatedEncoding)?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(FailedAttemptError::TruncatedEncoding)?;
        let mut value = [0; N];
        value.copy_from_slice(bytes);
        self.cursor = end;
        Ok(value)
    }
    fn read_text(&mut self) -> Result<String, FailedAttemptError> {
        let length = u32::from_be_bytes(self.read_array()?) as usize;
        if length > MAX_TEXT_FIELD_LEN {
            return Err(FailedAttemptError::TextTooLarge);
        }
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(FailedAttemptError::TruncatedEncoding)?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(FailedAttemptError::TruncatedEncoding)?;
        let value = std::str::from_utf8(bytes)
            .map_err(|_| FailedAttemptError::InvalidUtf8)?
            .to_owned();
        self.cursor = end;
        Ok(value)
    }
    fn read_record_id(&mut self) -> Result<RecordId, FailedAttemptError> {
        Ok(RecordId::from_bytes(self.read_array()?))
    }
    fn read_record_ids(&mut self) -> Result<Vec<RecordId>, FailedAttemptError> {
        let count = u16::from_be_bytes(self.read_array()?) as usize;
        if count > MAX_LIST_ITEMS {
            return Err(FailedAttemptError::TooManyItems);
        }
        if self.bytes.len().saturating_sub(self.cursor) < count * ID_LEN {
            return Err(FailedAttemptError::TruncatedEncoding);
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
    use crate::EvidenceRecord;
    use sovereign_registry::IdentityId;

    fn rid(seed: u8) -> RecordId {
        RecordId::from_bytes([seed; ID_LEN])
    }
    fn identity(seed: u8) -> IdentityId {
        IdentityId::from_bytes([seed; ID_LEN])
    }
    fn payload() -> FailedAttemptPayload {
        FailedAttemptPayload::new(
            rid(1),
            rid(2),
            FailureKind::ToolFailure,
            "Tool returned a bounded failure".into(),
            vec![rid(3), rid(4)],
            Some(rid(5)),
        )
        .unwrap()
    }

    #[test]
    fn round_trip_preserves_negative_evidence_order_and_every_kind() {
        let value = payload();
        assert_eq!(
            FailedAttemptPayload::decode(&value.encode()).unwrap(),
            value
        );
        assert_eq!(value.evidence_ids(), &[rid(3), rid(4)]);
        for kind in [
            FailureKind::InvalidInput,
            FailureKind::MethodFailure,
            FailureKind::ToolFailure,
            FailureKind::Timeout,
            FailureKind::Inconclusive,
            FailureKind::PolicyDenial,
        ] {
            let value =
                FailedAttemptPayload::new(rid(1), rid(2), kind, "F".into(), vec![], None).unwrap();
            assert_eq!(
                FailedAttemptPayload::decode(&value.encode()).unwrap(),
                value
            );
        }
    }

    #[test]
    fn minimal_fixed_vector_is_stable_and_big_endian() {
        let value = FailedAttemptPayload::new(
            rid(1),
            rid(2),
            FailureKind::InvalidInput,
            "F".into(),
            vec![],
            None,
        )
        .unwrap();
        let mut expected = vec![1; ID_LEN];
        expected.extend_from_slice(&[2; ID_LEN]);
        expected.extend_from_slice(&[0, 0, 0, 0, 1, b'F', 0, 0, 0]);
        assert_eq!(value.encode(), expected);
        assert_eq!(FailedAttemptPayload::decode(&expected).unwrap(), value);
    }

    #[test]
    fn governed_record_identity_is_stable() {
        let record = EvidenceRecord::new_failed_attempt(
            identity(9),
            identity(10),
            identity(11),
            vec![rid(12)],
            payload(),
        )
        .unwrap();
        assert_eq!(
            record.id().to_string(),
            "aa792bd0335d98430c41b86ac3ea9289c3e09813ef82a4ed15dd4d286d0c4c48"
        );
        assert_eq!(
            EvidenceRecord::decode(&record.encode())
                .unwrap()
                .decode_failed_attempt_payload()
                .unwrap(),
            payload()
        );
    }

    #[test]
    fn required_fields_uniqueness_and_limits_fail_closed() {
        let make = |objective, method, text, evidence, retry| {
            FailedAttemptPayload::new(
                objective,
                method,
                FailureKind::Inconclusive,
                text,
                evidence,
                retry,
            )
        };
        assert_eq!(
            make(
                RecordId::from_bytes([0; ID_LEN]),
                rid(2),
                "F".into(),
                vec![],
                None
            )
            .unwrap_err(),
            FailedAttemptError::ZeroRecordId
        );
        assert_eq!(
            make(
                rid(1),
                RecordId::from_bytes([0; ID_LEN]),
                "F".into(),
                vec![],
                None
            )
            .unwrap_err(),
            FailedAttemptError::ZeroRecordId
        );
        assert_eq!(
            make(rid(1), rid(2), "".into(), vec![], None).unwrap_err(),
            FailedAttemptError::EmptyText
        );
        assert_eq!(
            make(rid(1), rid(2), "F\0".into(), vec![], None).unwrap_err(),
            FailedAttemptError::NulText
        );
        assert_eq!(
            make(
                rid(1),
                rid(2),
                "F".into(),
                vec![RecordId::from_bytes([0; ID_LEN])],
                None
            )
            .unwrap_err(),
            FailedAttemptError::ZeroRecordId
        );
        assert_eq!(
            make(rid(1), rid(2), "F".into(), vec![rid(3), rid(3)], None).unwrap_err(),
            FailedAttemptError::DuplicateRecordId
        );
        assert_eq!(
            make(
                rid(1),
                rid(2),
                "F".into(),
                vec![],
                Some(RecordId::from_bytes([0; ID_LEN]))
            )
            .unwrap_err(),
            FailedAttemptError::ZeroRecordId
        );
        assert!(make(
            rid(1),
            rid(2),
            "x".repeat(MAX_TEXT_FIELD_LEN),
            (1..=MAX_LIST_ITEMS).map(|v| rid(v as u8)).collect(),
            None
        )
        .is_ok());
        assert_eq!(
            make(
                rid(1),
                rid(2),
                "x".repeat(MAX_TEXT_FIELD_LEN + 1),
                vec![],
                None
            )
            .unwrap_err(),
            FailedAttemptError::TextTooLarge
        );
        assert_eq!(
            make(
                rid(1),
                rid(2),
                "F".into(),
                (1..=MAX_LIST_ITEMS + 1).map(|v| rid(v as u8)).collect(),
                None
            )
            .unwrap_err(),
            FailedAttemptError::TooManyItems
        );
    }

    #[test]
    fn decoder_rejects_malformed_discriminants_lengths_and_utf8() {
        let minimal = FailedAttemptPayload::new(
            rid(1),
            rid(2),
            FailureKind::InvalidInput,
            "F".into(),
            vec![],
            None,
        )
        .unwrap();
        let mut kind = minimal.encode();
        kind[2 * ID_LEN] = 0xff;
        assert_eq!(
            FailedAttemptPayload::decode(&kind).unwrap_err(),
            FailedAttemptError::UnknownFailureKind(0xff)
        );
        let mut utf8 = minimal.encode();
        utf8[2 * ID_LEN + 1 + 4] = 0xff;
        assert_eq!(
            FailedAttemptPayload::decode(&utf8).unwrap_err(),
            FailedAttemptError::InvalidUtf8
        );
        let mut text_length = minimal.encode();
        text_length[2 * ID_LEN + 1..2 * ID_LEN + 5]
            .copy_from_slice(&((MAX_TEXT_FIELD_LEN + 1) as u32).to_be_bytes());
        assert_eq!(
            FailedAttemptPayload::decode(&text_length).unwrap_err(),
            FailedAttemptError::TextTooLarge
        );
        let mut marker = minimal.encode();
        *marker.last_mut().unwrap() = 2;
        assert_eq!(
            FailedAttemptPayload::decode(&marker).unwrap_err(),
            FailedAttemptError::InvalidOptionalMarker(2)
        );
        let mut count = minimal.encode();
        let offset = 2 * ID_LEN + 1 + 4 + 1;
        count[offset..offset + 2].copy_from_slice(&((MAX_LIST_ITEMS + 1) as u16).to_be_bytes());
        assert_eq!(
            FailedAttemptPayload::decode(&count).unwrap_err(),
            FailedAttemptError::TooManyItems
        );
    }

    #[test]
    fn every_truncation_trailing_and_oversize_fails_closed() {
        let encoded = payload().encode();
        for end in 0..encoded.len() {
            assert!(
                FailedAttemptPayload::decode(&encoded[..end]).is_err(),
                "{end}"
            );
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            FailedAttemptPayload::decode(&trailing).unwrap_err(),
            FailedAttemptError::TrailingBytes
        );
        assert_eq!(
            FailedAttemptPayload::decode(&vec![0; MAX_EVIDENCE_PAYLOAD_LEN + 1]).unwrap_err(),
            FailedAttemptError::PayloadTooLarge
        );
    }
}
