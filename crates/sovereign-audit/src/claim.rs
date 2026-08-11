use crate::{RecordId, MAX_EVIDENCE_PAYLOAD_LEN, MAX_LIST_ITEMS, MAX_TEXT_FIELD_LEN};
use std::collections::HashSet;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ClaimKind {
    Observation = 0x00,
    Inference = 0x01,
    Prediction = 0x02,
    Recommendation = 0x03,
    Normative = 0x04,
}

impl ClaimKind {
    fn decode(value: u8) -> Result<Self, ClaimError> {
        match value {
            0x00 => Ok(Self::Observation),
            0x01 => Ok(Self::Inference),
            0x02 => Ok(Self::Prediction),
            0x03 => Ok(Self::Recommendation),
            0x04 => Ok(Self::Normative),
            _ => Err(ClaimError::UnknownClaimKind(value)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Substantiation {
    Unsubstantiated = 0x00,
    EvidenceCited = 0x01,
}

impl Substantiation {
    fn decode(value: u8) -> Result<Self, ClaimError> {
        match value {
            0x00 => Ok(Self::Unsubstantiated),
            0x01 => Ok(Self::EvidenceCited),
            _ => Err(ClaimError::UnknownSubstantiation(value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimPayload {
    objective_id: RecordId,
    statement: String,
    claim_kind: ClaimKind,
    substantiation: Substantiation,
    supporting_evidence_ids: Vec<RecordId>,
    counter_evidence_ids: Vec<RecordId>,
    uncertainty_ids: Vec<RecordId>,
}

impl ClaimPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        objective_id: RecordId,
        statement: String,
        claim_kind: ClaimKind,
        substantiation: Substantiation,
        supporting_evidence_ids: Vec<RecordId>,
        counter_evidence_ids: Vec<RecordId>,
        uncertainty_ids: Vec<RecordId>,
    ) -> Result<Self, ClaimError> {
        if objective_id.is_zero() {
            return Err(ClaimError::ZeroIdentifier);
        }
        validate_text(&statement)?;
        validate_ids(&supporting_evidence_ids)?;
        validate_ids(&counter_evidence_ids)?;
        validate_ids(&uncertainty_ids)?;
        match (substantiation, supporting_evidence_ids.is_empty()) {
            (Substantiation::Unsubstantiated, false) | (Substantiation::EvidenceCited, true) => {
                return Err(ClaimError::IncoherentSubstantiation)
            }
            _ => {}
        }
        let payload = Self {
            objective_id,
            statement,
            claim_kind,
            substantiation,
            supporting_evidence_ids,
            counter_evidence_ids,
            uncertainty_ids,
        };
        if payload.encoded_len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(ClaimError::PayloadTooLarge);
        }
        Ok(payload)
    }

    pub const fn objective_id(&self) -> RecordId {
        self.objective_id
    }
    pub fn statement(&self) -> &str {
        &self.statement
    }
    pub const fn claim_kind(&self) -> ClaimKind {
        self.claim_kind
    }
    pub const fn substantiation(&self) -> Substantiation {
        self.substantiation
    }
    pub fn supporting_evidence_ids(&self) -> &[RecordId] {
        &self.supporting_evidence_ids
    }
    pub fn counter_evidence_ids(&self) -> &[RecordId] {
        &self.counter_evidence_ids
    }
    pub fn uncertainty_ids(&self) -> &[RecordId] {
        &self.uncertainty_ids
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(self.encoded_len());
        encoded.extend_from_slice(self.objective_id.as_bytes());
        encode_text(&mut encoded, &self.statement);
        encoded.push(self.claim_kind as u8);
        encoded.push(self.substantiation as u8);
        encode_ids(&mut encoded, &self.supporting_evidence_ids);
        encode_ids(&mut encoded, &self.counter_evidence_ids);
        encode_ids(&mut encoded, &self.uncertainty_ids);
        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ClaimError> {
        if bytes.len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(ClaimError::PayloadTooLarge);
        }
        let mut decoder = Decoder::new(bytes);
        let objective_id = decoder.read_id()?;
        let statement = decoder.read_text()?;
        let claim_kind = ClaimKind::decode(decoder.read_u8()?)?;
        let substantiation = Substantiation::decode(decoder.read_u8()?)?;
        let supporting = decoder.read_ids()?;
        let counter = decoder.read_ids()?;
        let uncertainty = decoder.read_ids()?;
        if !decoder.finished() {
            return Err(ClaimError::TrailingBytes);
        }
        Self::new(
            objective_id,
            statement,
            claim_kind,
            substantiation,
            supporting,
            counter,
            uncertainty,
        )
    }

    fn encoded_len(&self) -> usize {
        32 + 4
            + self.statement.len()
            + 2
            + 6
            + 32 * (self.supporting_evidence_ids.len()
                + self.counter_evidence_ids.len()
                + self.uncertainty_ids.len())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaimError {
    EmptyText,
    TextTooLarge,
    NulText,
    InvalidUtf8,
    ZeroIdentifier,
    DuplicateIdentifier,
    TooManyItems,
    UnknownClaimKind(u8),
    UnknownSubstantiation(u8),
    IncoherentSubstantiation,
    PayloadTooLarge,
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for ClaimError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for ClaimError {}

fn validate_text(value: &str) -> Result<(), ClaimError> {
    if value.is_empty() {
        return Err(ClaimError::EmptyText);
    }
    if value.len() > MAX_TEXT_FIELD_LEN {
        return Err(ClaimError::TextTooLarge);
    }
    if value.as_bytes().contains(&0) {
        return Err(ClaimError::NulText);
    }
    Ok(())
}

fn validate_ids(values: &[RecordId]) -> Result<(), ClaimError> {
    if values.len() > MAX_LIST_ITEMS {
        return Err(ClaimError::TooManyItems);
    }
    let mut seen = HashSet::with_capacity(values.len());
    for value in values {
        if value.is_zero() {
            return Err(ClaimError::ZeroIdentifier);
        }
        if !seen.insert(*value) {
            return Err(ClaimError::DuplicateIdentifier);
        }
    }
    Ok(())
}

fn encode_text(encoded: &mut Vec<u8>, value: &str) {
    encoded.extend_from_slice(&(value.len() as u32).to_be_bytes());
    encoded.extend_from_slice(value.as_bytes());
}

fn encode_ids(encoded: &mut Vec<u8>, values: &[RecordId]) {
    encoded.extend_from_slice(&(values.len() as u16).to_be_bytes());
    for value in values {
        encoded.extend_from_slice(value.as_bytes());
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }
    fn finished(&self) -> bool {
        self.cursor == self.bytes.len()
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], ClaimError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(ClaimError::TruncatedEncoding)?;
        if end > self.bytes.len() {
            return Err(ClaimError::TruncatedEncoding);
        }
        let value = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(value)
    }
    fn read_u8(&mut self) -> Result<u8, ClaimError> {
        Ok(self.take(1)?[0])
    }
    fn read_u16(&mut self) -> Result<u16, ClaimError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
    fn read_u32(&mut self) -> Result<u32, ClaimError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
    fn read_id(&mut self) -> Result<RecordId, ClaimError> {
        let bytes = self.take(32)?;
        let mut id = [0; 32];
        id.copy_from_slice(bytes);
        Ok(RecordId::from_bytes(id))
    }
    fn read_text(&mut self) -> Result<String, ClaimError> {
        let length = self.read_u32()? as usize;
        if length > MAX_TEXT_FIELD_LEN {
            return Err(ClaimError::TextTooLarge);
        }
        let bytes = self.take(length)?;
        let value = std::str::from_utf8(bytes).map_err(|_| ClaimError::InvalidUtf8)?;
        validate_text(value)?;
        Ok(value.to_owned())
    }
    fn read_ids(&mut self) -> Result<Vec<RecordId>, ClaimError> {
        let count = self.read_u16()? as usize;
        if count > MAX_LIST_ITEMS {
            return Err(ClaimError::TooManyItems);
        }
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.read_id()?);
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(value: u8) -> RecordId {
        RecordId::from_bytes([value; 32])
    }
    fn claim() -> ClaimPayload {
        ClaimPayload::new(
            id(1),
            "A bounded proposition".into(),
            ClaimKind::Inference,
            Substantiation::EvidenceCited,
            vec![id(2), id(3)],
            vec![id(4)],
            vec![id(5)],
        )
        .unwrap()
    }

    #[test]
    fn round_trips_canonically_and_preserves_order() {
        let original = claim();
        let decoded = ClaimPayload::decode(&original.encode()).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(decoded.encode(), original.encode());
        assert_eq!(decoded.supporting_evidence_ids(), &[id(2), id(3)]);
    }

    #[test]
    fn fixed_vector_is_stable() {
        let payload = ClaimPayload::new(
            id(1),
            "C".into(),
            ClaimKind::Observation,
            Substantiation::Unsubstantiated,
            vec![],
            vec![],
            vec![],
        )
        .unwrap();
        let mut expected = vec![1; 32];
        expected.extend_from_slice(&[0, 0, 0, 1, b'C', 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(payload.encode(), expected);
        assert_eq!(ClaimPayload::decode(&expected).unwrap(), payload);
    }

    #[test]
    fn substantiation_must_match_supporting_evidence() {
        let make = |state, support| {
            ClaimPayload::new(
                id(1),
                "C".into(),
                ClaimKind::Observation,
                state,
                support,
                vec![],
                vec![],
            )
        };
        assert_eq!(
            make(Substantiation::Unsubstantiated, vec![id(2)]).unwrap_err(),
            ClaimError::IncoherentSubstantiation
        );
        assert_eq!(
            make(Substantiation::EvidenceCited, vec![]).unwrap_err(),
            ClaimError::IncoherentSubstantiation
        );
        assert!(make(Substantiation::Unsubstantiated, vec![]).is_ok());
        assert!(make(Substantiation::EvidenceCited, vec![id(2)]).is_ok());
    }

    #[test]
    fn identifiers_are_nonzero_unique_and_bounded() {
        let make = |objective, support| {
            ClaimPayload::new(
                objective,
                "C".into(),
                ClaimKind::Observation,
                Substantiation::EvidenceCited,
                support,
                vec![],
                vec![],
            )
        };
        assert_eq!(
            make(id(0), vec![id(2)]).unwrap_err(),
            ClaimError::ZeroIdentifier
        );
        assert_eq!(
            make(id(1), vec![id(2), id(2)]).unwrap_err(),
            ClaimError::DuplicateIdentifier
        );
        assert_eq!(
            make(id(1), vec![id(2); MAX_LIST_ITEMS + 1]).unwrap_err(),
            ClaimError::TooManyItems
        );
    }

    #[test]
    fn text_and_discriminators_fail_closed() {
        assert_eq!(
            ClaimPayload::new(
                id(1),
                "".into(),
                ClaimKind::Observation,
                Substantiation::Unsubstantiated,
                vec![],
                vec![],
                vec![]
            )
            .unwrap_err(),
            ClaimError::EmptyText
        );
        let mut unknown_kind = claim().encode();
        unknown_kind[32 + 4 + claim().statement.len()] = 0xff;
        assert_eq!(
            ClaimPayload::decode(&unknown_kind).unwrap_err(),
            ClaimError::UnknownClaimKind(0xff)
        );
        let mut unknown_state = claim().encode();
        unknown_state[32 + 4 + claim().statement.len() + 1] = 0xff;
        assert_eq!(
            ClaimPayload::decode(&unknown_state).unwrap_err(),
            ClaimError::UnknownSubstantiation(0xff)
        );
    }

    #[test]
    fn malformed_lengths_utf8_truncation_and_trailing_bytes_fail_closed() {
        let mut oversized = claim().encode();
        oversized[32..36].copy_from_slice(&((MAX_TEXT_FIELD_LEN + 1) as u32).to_be_bytes());
        assert_eq!(
            ClaimPayload::decode(&oversized).unwrap_err(),
            ClaimError::TextTooLarge
        );
        let mut invalid = vec![1; 32];
        invalid.extend_from_slice(&[0, 0, 0, 1, 0xff]);
        assert_eq!(
            ClaimPayload::decode(&invalid).unwrap_err(),
            ClaimError::InvalidUtf8
        );
        let encoded = claim().encode();
        for end in 0..encoded.len() {
            assert!(ClaimPayload::decode(&encoded[..end]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            ClaimPayload::decode(&trailing).unwrap_err(),
            ClaimError::TrailingBytes
        );
    }
}
