use crate::{RecordId, MAX_EVIDENCE_PAYLOAD_LEN, MAX_LIST_ITEMS, MAX_TEXT_FIELD_LEN};
use sovereign_registry::IdentityId;
use std::collections::HashSet;
use std::fmt;

const ID_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FindingKind {
    Support = 0x00,
    Contradiction = 0x01,
    MethodologicalFlaw = 0x02,
    ProvenanceGap = 0x03,
    Security = 0x04,
    PrivacyLicensing = 0x05,
    Inconclusive = 0x06,
}

impl FindingKind {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self, ReviewerFindingError> {
        match value {
            0x00 => Ok(Self::Support),
            0x01 => Ok(Self::Contradiction),
            0x02 => Ok(Self::MethodologicalFlaw),
            0x03 => Ok(Self::ProvenanceGap),
            0x04 => Ok(Self::Security),
            0x05 => Ok(Self::PrivacyLicensing),
            0x06 => Ok(Self::Inconclusive),
            _ => Err(ReviewerFindingError::UnknownFindingKind(value)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FindingSeverity {
    Informational = 0x00,
    Low = 0x01,
    Medium = 0x02,
    High = 0x03,
    Critical = 0x04,
}

impl FindingSeverity {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self, ReviewerFindingError> {
        match value {
            0x00 => Ok(Self::Informational),
            0x01 => Ok(Self::Low),
            0x02 => Ok(Self::Medium),
            0x03 => Ok(Self::High),
            0x04 => Ok(Self::Critical),
            _ => Err(ReviewerFindingError::UnknownSeverity(value)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum IndependenceResult {
    Established = 0x00,
    NotEstablished = 0x01,
    Conflicted = 0x02,
    Unknown = 0x03,
}

impl IndependenceResult {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self, ReviewerFindingError> {
        match value {
            0x00 => Ok(Self::Established),
            0x01 => Ok(Self::NotEstablished),
            0x02 => Ok(Self::Conflicted),
            0x03 => Ok(Self::Unknown),
            _ => Err(ReviewerFindingError::UnknownIndependenceResult(value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewerFindingPayload {
    reviewed_id: RecordId,
    reviewer_id: IdentityId,
    finding_kind: FindingKind,
    severity: FindingSeverity,
    rationale: String,
    evidence_ids: Vec<RecordId>,
    conflict_declaration: String,
    independence_result: IndependenceResult,
}

impl ReviewerFindingPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        reviewed_id: RecordId,
        reviewer_id: IdentityId,
        finding_kind: FindingKind,
        severity: FindingSeverity,
        rationale: String,
        evidence_ids: Vec<RecordId>,
        conflict_declaration: String,
        independence_result: IndependenceResult,
    ) -> Result<Self, ReviewerFindingError> {
        validate_record_id(reviewed_id)?;
        if reviewer_id.is_zero() {
            return Err(ReviewerFindingError::ZeroIdentity);
        }
        validate_text(&rationale)?;
        validate_record_ids(&evidence_ids)?;
        validate_text(&conflict_declaration)?;

        let payload = Self {
            reviewed_id,
            reviewer_id,
            finding_kind,
            severity,
            rationale,
            evidence_ids,
            conflict_declaration,
            independence_result,
        };
        if payload.encoded_len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(ReviewerFindingError::PayloadTooLarge);
        }
        Ok(payload)
    }

    pub const fn reviewed_id(&self) -> RecordId {
        self.reviewed_id
    }

    pub const fn reviewer_id(&self) -> IdentityId {
        self.reviewer_id
    }

    pub const fn finding_kind(&self) -> FindingKind {
        self.finding_kind
    }

    pub const fn severity(&self) -> FindingSeverity {
        self.severity
    }

    pub fn rationale(&self) -> &str {
        &self.rationale
    }

    pub fn evidence_ids(&self) -> &[RecordId] {
        &self.evidence_ids
    }

    pub fn conflict_declaration(&self) -> &str {
        &self.conflict_declaration
    }

    pub const fn independence_result(&self) -> IndependenceResult {
        self.independence_result
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(self.encoded_len());
        encoded.extend_from_slice(self.reviewed_id.as_bytes());
        encoded.extend_from_slice(self.reviewer_id.as_bytes());
        encoded.push(self.finding_kind.as_u8());
        encoded.push(self.severity.as_u8());
        encode_text(&mut encoded, &self.rationale);
        encoded.extend_from_slice(&(self.evidence_ids.len() as u16).to_be_bytes());
        for id in &self.evidence_ids {
            encoded.extend_from_slice(id.as_bytes());
        }
        encode_text(&mut encoded, &self.conflict_declaration);
        encoded.push(self.independence_result.as_u8());
        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ReviewerFindingError> {
        if bytes.len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(ReviewerFindingError::PayloadTooLarge);
        }
        let mut decoder = Decoder::new(bytes);
        let reviewed_id = decoder.read_record_id()?;
        let reviewer_id = decoder.read_identity_id()?;
        let finding_kind = FindingKind::from_u8(decoder.read_u8()?)?;
        let severity = FindingSeverity::from_u8(decoder.read_u8()?)?;
        let rationale = decoder.read_text()?;
        let evidence_ids = decoder.read_record_ids()?;
        let conflict_declaration = decoder.read_text()?;
        let independence_result = IndependenceResult::from_u8(decoder.read_u8()?)?;
        if !decoder.is_finished() {
            return Err(ReviewerFindingError::TrailingBytes);
        }
        Self::new(
            reviewed_id,
            reviewer_id,
            finding_kind,
            severity,
            rationale,
            evidence_ids,
            conflict_declaration,
            independence_result,
        )
    }

    fn encoded_len(&self) -> usize {
        (2 * ID_LEN)
            + 2
            + 4
            + self.rationale.len()
            + 2
            + (self.evidence_ids.len() * ID_LEN)
            + 4
            + self.conflict_declaration.len()
            + 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewerFindingError {
    EmptyText,
    NulText,
    TextTooLarge,
    InvalidUtf8,
    ZeroRecordId,
    ZeroIdentity,
    DuplicateRecordId,
    TooManyItems,
    UnknownFindingKind(u8),
    UnknownSeverity(u8),
    UnknownIndependenceResult(u8),
    PayloadTooLarge,
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for ReviewerFindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReviewerFindingError {}

fn validate_text(value: &str) -> Result<(), ReviewerFindingError> {
    if value.is_empty() {
        return Err(ReviewerFindingError::EmptyText);
    }
    if value.len() > MAX_TEXT_FIELD_LEN {
        return Err(ReviewerFindingError::TextTooLarge);
    }
    if value.contains('\0') {
        return Err(ReviewerFindingError::NulText);
    }
    Ok(())
}

fn validate_record_id(id: RecordId) -> Result<(), ReviewerFindingError> {
    if id.is_zero() {
        Err(ReviewerFindingError::ZeroRecordId)
    } else {
        Ok(())
    }
}

fn validate_record_ids(ids: &[RecordId]) -> Result<(), ReviewerFindingError> {
    if ids.len() > MAX_LIST_ITEMS {
        return Err(ReviewerFindingError::TooManyItems);
    }
    let mut seen = HashSet::with_capacity(ids.len());
    for id in ids {
        validate_record_id(*id)?;
        if !seen.insert(*id) {
            return Err(ReviewerFindingError::DuplicateRecordId);
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

    fn read_u8(&mut self) -> Result<u8, ReviewerFindingError> {
        let value = *self
            .bytes
            .get(self.cursor)
            .ok_or(ReviewerFindingError::TruncatedEncoding)?;
        self.cursor += 1;
        Ok(value)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ReviewerFindingError> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(ReviewerFindingError::TruncatedEncoding)?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(ReviewerFindingError::TruncatedEncoding)?;
        let mut value = [0_u8; N];
        value.copy_from_slice(bytes);
        self.cursor = end;
        Ok(value)
    }

    fn read_text(&mut self) -> Result<String, ReviewerFindingError> {
        let length = u32::from_be_bytes(self.read_array()?) as usize;
        if length > MAX_TEXT_FIELD_LEN {
            return Err(ReviewerFindingError::TextTooLarge);
        }
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(ReviewerFindingError::TruncatedEncoding)?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(ReviewerFindingError::TruncatedEncoding)?;
        let value = std::str::from_utf8(bytes)
            .map_err(|_| ReviewerFindingError::InvalidUtf8)?
            .to_owned();
        self.cursor = end;
        Ok(value)
    }

    fn read_record_id(&mut self) -> Result<RecordId, ReviewerFindingError> {
        Ok(RecordId::from_bytes(self.read_array()?))
    }

    fn read_identity_id(&mut self) -> Result<IdentityId, ReviewerFindingError> {
        Ok(IdentityId::from_bytes(self.read_array()?))
    }

    fn read_record_ids(&mut self) -> Result<Vec<RecordId>, ReviewerFindingError> {
        let count = u16::from_be_bytes(self.read_array()?) as usize;
        if count > MAX_LIST_ITEMS {
            return Err(ReviewerFindingError::TooManyItems);
        }
        if self.bytes.len().saturating_sub(self.cursor) < count * ID_LEN {
            return Err(ReviewerFindingError::TruncatedEncoding);
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

    fn record_id(seed: u8) -> RecordId {
        RecordId::from_bytes([seed; ID_LEN])
    }

    fn identity(seed: u8) -> IdentityId {
        IdentityId::from_bytes([seed; ID_LEN])
    }

    fn finding() -> ReviewerFindingPayload {
        ReviewerFindingPayload::new(
            record_id(1),
            identity(2),
            FindingKind::MethodologicalFlaw,
            FindingSeverity::High,
            "The declared method does not control the observed confounder".into(),
            vec![record_id(3), record_id(4)],
            "NONE_DECLARED".into(),
            IndependenceResult::Established,
        )
        .unwrap()
    }

    #[test]
    fn round_trip_preserves_every_field_and_discriminant() {
        let value = finding();
        assert_eq!(
            ReviewerFindingPayload::decode(&value.encode()).unwrap(),
            value
        );
        assert_eq!(value.evidence_ids(), &[record_id(3), record_id(4)]);

        for kind in [
            FindingKind::Support,
            FindingKind::Contradiction,
            FindingKind::MethodologicalFlaw,
            FindingKind::ProvenanceGap,
            FindingKind::Security,
            FindingKind::PrivacyLicensing,
            FindingKind::Inconclusive,
        ] {
            let value = ReviewerFindingPayload::new(
                record_id(1),
                identity(2),
                kind,
                FindingSeverity::Informational,
                "R".into(),
                vec![],
                "NONE_DECLARED".into(),
                IndependenceResult::Unknown,
            )
            .unwrap();
            assert_eq!(
                ReviewerFindingPayload::decode(&value.encode()).unwrap(),
                value
            );
        }

        for severity in [
            FindingSeverity::Informational,
            FindingSeverity::Low,
            FindingSeverity::Medium,
            FindingSeverity::High,
            FindingSeverity::Critical,
        ] {
            let value = ReviewerFindingPayload::new(
                record_id(1),
                identity(2),
                FindingKind::Support,
                severity,
                "R".into(),
                vec![],
                "NONE_DECLARED".into(),
                IndependenceResult::Unknown,
            )
            .unwrap();
            assert_eq!(
                ReviewerFindingPayload::decode(&value.encode()).unwrap(),
                value
            );
        }

        for independence in [
            IndependenceResult::Established,
            IndependenceResult::NotEstablished,
            IndependenceResult::Conflicted,
            IndependenceResult::Unknown,
        ] {
            let value = ReviewerFindingPayload::new(
                record_id(1),
                identity(2),
                FindingKind::Support,
                FindingSeverity::Informational,
                "R".into(),
                vec![],
                "NONE_DECLARED".into(),
                independence,
            )
            .unwrap();
            assert_eq!(
                ReviewerFindingPayload::decode(&value.encode()).unwrap(),
                value
            );
        }
    }

    #[test]
    fn minimal_fixed_vector_is_stable_and_big_endian() {
        let value = ReviewerFindingPayload::new(
            record_id(1),
            identity(2),
            FindingKind::Support,
            FindingSeverity::Informational,
            "R".into(),
            vec![],
            "NONE_DECLARED".into(),
            IndependenceResult::Established,
        )
        .unwrap();
        let mut expected = vec![1; ID_LEN];
        expected.extend_from_slice(&[2; ID_LEN]);
        expected.extend_from_slice(&[0, 0, 0, 0, 0, 1, b'R', 0, 0, 0, 0, 0, 13]);
        expected.extend_from_slice(b"NONE_DECLARED");
        expected.push(0);
        assert_eq!(value.encode(), expected);
        assert_eq!(ReviewerFindingPayload::decode(&expected).unwrap(), value);
    }

    #[test]
    fn governed_record_identity_is_stable() {
        let record = EvidenceRecord::new_reviewer_finding(
            identity(9),
            identity(10),
            identity(11),
            vec![record_id(12)],
            finding(),
        )
        .unwrap();
        assert_eq!(
            record.id().to_string(),
            "5399568236fad9e71dcc88f498e3ebdaa7a2929d1a642c1d587a2b0d0593f609"
        );
        assert_eq!(
            EvidenceRecord::decode(&record.encode())
                .unwrap()
                .decode_reviewer_finding_payload()
                .unwrap(),
            finding()
        );
    }

    #[test]
    fn required_fields_uniqueness_and_limits_fail_closed() {
        let make = |reviewed_id, reviewer_id, rationale, evidence_ids, conflict| {
            ReviewerFindingPayload::new(
                reviewed_id,
                reviewer_id,
                FindingKind::Inconclusive,
                FindingSeverity::Medium,
                rationale,
                evidence_ids,
                conflict,
                IndependenceResult::Unknown,
            )
        };
        assert_eq!(
            make(
                RecordId::from_bytes([0; ID_LEN]),
                identity(2),
                "R".into(),
                vec![],
                "NONE_DECLARED".into(),
            )
            .unwrap_err(),
            ReviewerFindingError::ZeroRecordId
        );
        assert_eq!(
            make(
                record_id(1),
                IdentityId::from_bytes([0; ID_LEN]),
                "R".into(),
                vec![],
                "NONE_DECLARED".into(),
            )
            .unwrap_err(),
            ReviewerFindingError::ZeroIdentity
        );
        assert_eq!(
            make(record_id(1), identity(2), "".into(), vec![], "C".into()).unwrap_err(),
            ReviewerFindingError::EmptyText
        );
        assert_eq!(
            make(record_id(1), identity(2), "R".into(), vec![], "".into()).unwrap_err(),
            ReviewerFindingError::EmptyText
        );
        assert_eq!(
            make(record_id(1), identity(2), "R\0".into(), vec![], "C".into(),).unwrap_err(),
            ReviewerFindingError::NulText
        );
        assert_eq!(
            make(
                record_id(1),
                identity(2),
                "R".into(),
                vec![RecordId::from_bytes([0; ID_LEN])],
                "C".into(),
            )
            .unwrap_err(),
            ReviewerFindingError::ZeroRecordId
        );
        assert_eq!(
            make(
                record_id(1),
                identity(2),
                "R".into(),
                vec![record_id(3), record_id(3)],
                "C".into(),
            )
            .unwrap_err(),
            ReviewerFindingError::DuplicateRecordId
        );
        assert!(make(
            record_id(1),
            identity(2),
            "r".repeat(MAX_TEXT_FIELD_LEN),
            (1..=MAX_LIST_ITEMS)
                .map(|value| record_id(value as u8))
                .collect(),
            "c".repeat(MAX_TEXT_FIELD_LEN),
        )
        .is_ok());
        assert_eq!(
            make(
                record_id(1),
                identity(2),
                "r".repeat(MAX_TEXT_FIELD_LEN + 1),
                vec![],
                "C".into(),
            )
            .unwrap_err(),
            ReviewerFindingError::TextTooLarge
        );
        assert_eq!(
            make(
                record_id(1),
                identity(2),
                "R".into(),
                (1..=MAX_LIST_ITEMS + 1)
                    .map(|value| record_id(value as u8))
                    .collect(),
                "C".into(),
            )
            .unwrap_err(),
            ReviewerFindingError::TooManyItems
        );
    }

    #[test]
    fn decoder_rejects_malformed_discriminants_lengths_and_utf8() {
        let minimal = ReviewerFindingPayload::new(
            record_id(1),
            identity(2),
            FindingKind::Support,
            FindingSeverity::Informational,
            "R".into(),
            vec![],
            "C".into(),
            IndependenceResult::Established,
        )
        .unwrap();
        let mut kind = minimal.encode();
        kind[2 * ID_LEN] = 0xff;
        assert_eq!(
            ReviewerFindingPayload::decode(&kind).unwrap_err(),
            ReviewerFindingError::UnknownFindingKind(0xff)
        );
        let mut severity = minimal.encode();
        severity[(2 * ID_LEN) + 1] = 0xff;
        assert_eq!(
            ReviewerFindingPayload::decode(&severity).unwrap_err(),
            ReviewerFindingError::UnknownSeverity(0xff)
        );
        let mut rationale_utf8 = minimal.encode();
        rationale_utf8[(2 * ID_LEN) + 2 + 4] = 0xff;
        assert_eq!(
            ReviewerFindingPayload::decode(&rationale_utf8).unwrap_err(),
            ReviewerFindingError::InvalidUtf8
        );
        let mut rationale_length = minimal.encode();
        rationale_length[(2 * ID_LEN) + 2..(2 * ID_LEN) + 6]
            .copy_from_slice(&((MAX_TEXT_FIELD_LEN + 1) as u32).to_be_bytes());
        assert_eq!(
            ReviewerFindingPayload::decode(&rationale_length).unwrap_err(),
            ReviewerFindingError::TextTooLarge
        );
        let mut evidence_count = minimal.encode();
        let count_offset = (2 * ID_LEN) + 2 + 4 + 1;
        evidence_count[count_offset..count_offset + 2]
            .copy_from_slice(&((MAX_LIST_ITEMS + 1) as u16).to_be_bytes());
        assert_eq!(
            ReviewerFindingPayload::decode(&evidence_count).unwrap_err(),
            ReviewerFindingError::TooManyItems
        );
        let mut conflict_utf8 = minimal.encode();
        let conflict_offset = count_offset + 2 + 4;
        conflict_utf8[conflict_offset] = 0xff;
        assert_eq!(
            ReviewerFindingPayload::decode(&conflict_utf8).unwrap_err(),
            ReviewerFindingError::InvalidUtf8
        );
        let mut independence = minimal.encode();
        *independence.last_mut().unwrap() = 0xff;
        assert_eq!(
            ReviewerFindingPayload::decode(&independence).unwrap_err(),
            ReviewerFindingError::UnknownIndependenceResult(0xff)
        );
    }

    #[test]
    fn every_truncation_trailing_and_oversize_fails_closed() {
        let encoded = finding().encode();
        for end in 0..encoded.len() {
            assert!(
                ReviewerFindingPayload::decode(&encoded[..end]).is_err(),
                "accepted {end}"
            );
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            ReviewerFindingPayload::decode(&trailing).unwrap_err(),
            ReviewerFindingError::TrailingBytes
        );
        assert_eq!(
            ReviewerFindingPayload::decode(&vec![0; MAX_EVIDENCE_PAYLOAD_LEN + 1]).unwrap_err(),
            ReviewerFindingError::PayloadTooLarge
        );
    }
}
