use crate::{
    DigestAlgorithm, RecordId, MAX_EVIDENCE_PAYLOAD_LEN, MAX_LIST_ITEMS, MAX_TEXT_FIELD_LEN,
};
use sovereign_registry::IdentityId;
use std::collections::HashSet;
use std::fmt;

const ID_LEN: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodPayload {
    objective_id: RecordId,
    procedure: String,
    input_ids: Vec<RecordId>,
    tool_ids: Vec<IdentityId>,
    environment_id: IdentityId,
    parameters_digest_algorithm: DigestAlgorithm,
    parameters_digest: [u8; ID_LEN],
    budget_reference: Option<RecordId>,
}

impl MethodPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        objective_id: RecordId,
        procedure: String,
        input_ids: Vec<RecordId>,
        tool_ids: Vec<IdentityId>,
        environment_id: IdentityId,
        parameters_digest_algorithm: DigestAlgorithm,
        parameters_digest: [u8; ID_LEN],
        budget_reference: Option<RecordId>,
    ) -> Result<Self, MethodError> {
        validate_record_id(objective_id)?;
        validate_text(&procedure)?;
        validate_record_ids(&input_ids)?;
        validate_identity_ids(&tool_ids)?;
        if environment_id.is_zero() {
            return Err(MethodError::ZeroIdentity);
        }
        if budget_reference.is_some_and(|reference| reference.is_zero()) {
            return Err(MethodError::ZeroRecordId);
        }
        let payload = Self {
            objective_id,
            procedure,
            input_ids,
            tool_ids,
            environment_id,
            parameters_digest_algorithm,
            parameters_digest,
            budget_reference,
        };
        if payload.encoded_len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(MethodError::PayloadTooLarge);
        }
        Ok(payload)
    }

    pub const fn objective_id(&self) -> RecordId {
        self.objective_id
    }
    pub fn procedure(&self) -> &str {
        &self.procedure
    }
    pub fn input_ids(&self) -> &[RecordId] {
        &self.input_ids
    }
    pub fn tool_ids(&self) -> &[IdentityId] {
        &self.tool_ids
    }
    pub const fn environment_id(&self) -> IdentityId {
        self.environment_id
    }
    pub const fn parameters_digest_algorithm(&self) -> DigestAlgorithm {
        self.parameters_digest_algorithm
    }
    pub const fn parameters_digest(&self) -> &[u8; ID_LEN] {
        &self.parameters_digest
    }
    pub const fn budget_reference(&self) -> Option<RecordId> {
        self.budget_reference
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(self.encoded_len());
        encoded.extend_from_slice(self.objective_id.as_bytes());
        encode_text(&mut encoded, &self.procedure);
        encode_record_ids(&mut encoded, &self.input_ids);
        encode_identity_ids(&mut encoded, &self.tool_ids);
        encoded.extend_from_slice(self.environment_id.as_bytes());
        encoded.push(self.parameters_digest_algorithm as u8);
        encoded.extend_from_slice(&self.parameters_digest);
        match self.budget_reference {
            None => encoded.push(0),
            Some(reference) => {
                encoded.push(1);
                encoded.extend_from_slice(reference.as_bytes());
            }
        }
        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MethodError> {
        if bytes.len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(MethodError::PayloadTooLarge);
        }
        let mut decoder = Decoder::new(bytes);
        let objective_id = decoder.read_record_id()?;
        let procedure = decoder.read_text()?;
        let input_ids = decoder.read_record_ids()?;
        let tool_ids = decoder.read_identity_ids()?;
        let environment_id = decoder.read_identity_id()?;
        let parameters_digest_algorithm = match decoder.read_u8()? {
            0x01 => DigestAlgorithm::Sha256,
            0x02 => DigestAlgorithm::Blake3,
            value => return Err(MethodError::UnknownDigestAlgorithm(value)),
        };
        let parameters_digest = decoder.read_array()?;
        let budget_reference = match decoder.read_u8()? {
            0 => None,
            1 => Some(decoder.read_record_id()?),
            marker => return Err(MethodError::InvalidOptionalMarker(marker)),
        };
        if !decoder.is_finished() {
            return Err(MethodError::TrailingBytes);
        }
        Self::new(
            objective_id,
            procedure,
            input_ids,
            tool_ids,
            environment_id,
            parameters_digest_algorithm,
            parameters_digest,
            budget_reference,
        )
    }

    fn encoded_len(&self) -> usize {
        ID_LEN
            + 4
            + self.procedure.len()
            + 2
            + (self.input_ids.len() * ID_LEN)
            + 2
            + (self.tool_ids.len() * ID_LEN)
            + ID_LEN
            + 1
            + ID_LEN
            + 1
            + self.budget_reference.map_or(0, |_| ID_LEN)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MethodError {
    EmptyText,
    NulText,
    TextTooLarge,
    InvalidUtf8,
    TooManyItems,
    ZeroRecordId,
    DuplicateRecordId,
    ZeroIdentity,
    DuplicateIdentity,
    UnknownDigestAlgorithm(u8),
    InvalidOptionalMarker(u8),
    PayloadTooLarge,
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for MethodError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for MethodError {}

fn validate_text(value: &str) -> Result<(), MethodError> {
    if value.is_empty() {
        return Err(MethodError::EmptyText);
    }
    if value.len() > MAX_TEXT_FIELD_LEN {
        return Err(MethodError::TextTooLarge);
    }
    if value.contains('\0') {
        return Err(MethodError::NulText);
    }
    Ok(())
}

fn validate_record_id(id: RecordId) -> Result<(), MethodError> {
    if id.is_zero() {
        Err(MethodError::ZeroRecordId)
    } else {
        Ok(())
    }
}

fn validate_record_ids(ids: &[RecordId]) -> Result<(), MethodError> {
    if ids.len() > MAX_LIST_ITEMS {
        return Err(MethodError::TooManyItems);
    }
    let mut seen = HashSet::with_capacity(ids.len());
    for id in ids {
        validate_record_id(*id)?;
        if !seen.insert(*id) {
            return Err(MethodError::DuplicateRecordId);
        }
    }
    Ok(())
}

fn validate_identity_ids(ids: &[IdentityId]) -> Result<(), MethodError> {
    if ids.len() > MAX_LIST_ITEMS {
        return Err(MethodError::TooManyItems);
    }
    let mut seen = HashSet::with_capacity(ids.len());
    for id in ids {
        if id.is_zero() {
            return Err(MethodError::ZeroIdentity);
        }
        if !seen.insert(*id) {
            return Err(MethodError::DuplicateIdentity);
        }
    }
    Ok(())
}

fn encode_text(encoded: &mut Vec<u8>, value: &str) {
    encoded.extend_from_slice(&(value.len() as u32).to_be_bytes());
    encoded.extend_from_slice(value.as_bytes());
}

fn encode_record_ids(encoded: &mut Vec<u8>, ids: &[RecordId]) {
    encoded.extend_from_slice(&(ids.len() as u16).to_be_bytes());
    for id in ids {
        encoded.extend_from_slice(id.as_bytes());
    }
}

fn encode_identity_ids(encoded: &mut Vec<u8>, ids: &[IdentityId]) {
    encoded.extend_from_slice(&(ids.len() as u16).to_be_bytes());
    for id in ids {
        encoded.extend_from_slice(id.as_bytes());
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

    fn read_u8(&mut self) -> Result<u8, MethodError> {
        let value = *self
            .bytes
            .get(self.cursor)
            .ok_or(MethodError::TruncatedEncoding)?;
        self.cursor += 1;
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16, MethodError> {
        Ok(u16::from_be_bytes(self.read_array()?))
    }
    fn read_u32(&mut self) -> Result<u32, MethodError> {
        Ok(u32::from_be_bytes(self.read_array()?))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], MethodError> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(MethodError::TruncatedEncoding)?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(MethodError::TruncatedEncoding)?;
        let mut value = [0_u8; N];
        value.copy_from_slice(bytes);
        self.cursor = end;
        Ok(value)
    }

    fn read_text(&mut self) -> Result<String, MethodError> {
        let length = self.read_u32()? as usize;
        if length > MAX_TEXT_FIELD_LEN {
            return Err(MethodError::TextTooLarge);
        }
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(MethodError::TruncatedEncoding)?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(MethodError::TruncatedEncoding)?;
        let value = std::str::from_utf8(bytes)
            .map_err(|_| MethodError::InvalidUtf8)?
            .to_owned();
        self.cursor = end;
        Ok(value)
    }

    fn read_record_id(&mut self) -> Result<RecordId, MethodError> {
        Ok(RecordId::from_bytes(self.read_array()?))
    }
    fn read_identity_id(&mut self) -> Result<IdentityId, MethodError> {
        Ok(IdentityId::from_bytes(self.read_array()?))
    }

    fn read_record_ids(&mut self) -> Result<Vec<RecordId>, MethodError> {
        let count = self.read_u16()? as usize;
        if count > MAX_LIST_ITEMS {
            return Err(MethodError::TooManyItems);
        }
        if self.bytes.len().saturating_sub(self.cursor) < count * ID_LEN {
            return Err(MethodError::TruncatedEncoding);
        }
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.read_record_id()?);
        }
        Ok(values)
    }

    fn read_identity_ids(&mut self) -> Result<Vec<IdentityId>, MethodError> {
        let count = self.read_u16()? as usize;
        if count > MAX_LIST_ITEMS {
            return Err(MethodError::TooManyItems);
        }
        if self.bytes.len().saturating_sub(self.cursor) < count * ID_LEN {
            return Err(MethodError::TruncatedEncoding);
        }
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.read_identity_id()?);
        }
        Ok(values)
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

    fn method() -> MethodPayload {
        MethodPayload::new(
            record_id(1),
            "P".into(),
            vec![record_id(2), record_id(3)],
            vec![identity(4), identity(5)],
            identity(6),
            DigestAlgorithm::Blake3,
            [7; ID_LEN],
            Some(record_id(8)),
        )
        .unwrap()
    }

    #[test]
    fn method_round_trips_and_preserves_order() {
        let payload = method();
        assert_eq!(MethodPayload::decode(&payload.encode()).unwrap(), payload);
        assert_eq!(payload.input_ids(), &[record_id(2), record_id(3)]);
        assert_eq!(payload.tool_ids(), &[identity(4), identity(5)]);
    }

    #[test]
    fn minimal_fixed_vector_is_stable_and_big_endian() {
        let payload = MethodPayload::new(
            record_id(1),
            "P".into(),
            vec![],
            vec![],
            identity(2),
            DigestAlgorithm::Sha256,
            [3; ID_LEN],
            None,
        )
        .unwrap();
        let mut expected = vec![1; ID_LEN];
        expected.extend_from_slice(&[0, 0, 0, 1, b'P', 0, 0, 0, 0]);
        expected.extend_from_slice(&[2; ID_LEN]);
        expected.push(1);
        expected.extend_from_slice(&[3; ID_LEN]);
        expected.push(0);
        assert_eq!(payload.encode(), expected);
        assert_eq!(MethodPayload::decode(&expected).unwrap(), payload);
    }

    #[test]
    fn governed_method_record_identity_is_stable() {
        let record = EvidenceRecord::new_method(
            identity(9),
            identity(10),
            identity(11),
            vec![record_id(12)],
            method(),
        )
        .unwrap();
        assert_eq!(
            record.id().to_string(),
            "8aaa8fc818dd0dbbae00b092cfe5f3707549e56ef664a163ab001b8ac68ad31e"
        );
        let decoded = EvidenceRecord::decode(&record.encode()).unwrap();
        assert_eq!(decoded.id(), record.id());
        assert_eq!(decoded.decode_method_payload().unwrap(), method());
    }

    #[test]
    fn required_values_and_uniqueness_fail_closed() {
        let make =
            |objective_id, procedure, input_ids, tool_ids, environment_id, budget_reference| {
                MethodPayload::new(
                    objective_id,
                    procedure,
                    input_ids,
                    tool_ids,
                    environment_id,
                    DigestAlgorithm::Sha256,
                    [1; ID_LEN],
                    budget_reference,
                )
            };
        assert_eq!(
            make(
                RecordId::from_bytes([0; ID_LEN]),
                "P".into(),
                vec![],
                vec![],
                identity(1),
                None
            )
            .unwrap_err(),
            MethodError::ZeroRecordId
        );
        assert_eq!(
            make(record_id(1), "".into(), vec![], vec![], identity(1), None).unwrap_err(),
            MethodError::EmptyText
        );
        assert_eq!(
            make(
                record_id(1),
                "P\0x".into(),
                vec![],
                vec![],
                identity(1),
                None
            )
            .unwrap_err(),
            MethodError::NulText
        );
        assert_eq!(
            make(
                record_id(1),
                "P".into(),
                vec![record_id(2), record_id(2)],
                vec![],
                identity(1),
                None
            )
            .unwrap_err(),
            MethodError::DuplicateRecordId
        );
        assert_eq!(
            make(
                record_id(1),
                "P".into(),
                vec![],
                vec![identity(2), identity(2)],
                identity(1),
                None
            )
            .unwrap_err(),
            MethodError::DuplicateIdentity
        );
        assert_eq!(
            make(
                record_id(1),
                "P".into(),
                vec![],
                vec![],
                IdentityId::from_bytes([0; ID_LEN]),
                None
            )
            .unwrap_err(),
            MethodError::ZeroIdentity
        );
        assert_eq!(
            make(
                record_id(1),
                "P".into(),
                vec![],
                vec![],
                identity(1),
                Some(RecordId::from_bytes([0; ID_LEN]))
            )
            .unwrap_err(),
            MethodError::ZeroRecordId
        );
    }

    #[test]
    fn text_and_list_boundaries_are_enforced() {
        assert!(MethodPayload::new(
            record_id(1),
            "x".repeat(MAX_TEXT_FIELD_LEN),
            (1..=MAX_LIST_ITEMS)
                .map(|value| record_id(value as u8))
                .collect(),
            (1..=MAX_LIST_ITEMS)
                .map(|value| identity(value as u8))
                .collect(),
            identity(65),
            DigestAlgorithm::Sha256,
            [1; ID_LEN],
            None,
        )
        .is_ok());
        assert_eq!(
            MethodPayload::new(
                record_id(1),
                "x".repeat(MAX_TEXT_FIELD_LEN + 1),
                vec![],
                vec![],
                identity(1),
                DigestAlgorithm::Sha256,
                [1; ID_LEN],
                None
            )
            .unwrap_err(),
            MethodError::TextTooLarge
        );
        assert_eq!(
            MethodPayload::new(
                record_id(1),
                "P".into(),
                (1..=(MAX_LIST_ITEMS + 1))
                    .map(|value| record_id(value as u8))
                    .collect(),
                vec![],
                identity(1),
                DigestAlgorithm::Sha256,
                [1; ID_LEN],
                None
            )
            .unwrap_err(),
            MethodError::TooManyItems
        );
    }

    #[test]
    fn decoder_rejects_invalid_utf8_counts_algorithm_and_marker() {
        let mut invalid_utf8 = method().encode();
        invalid_utf8[ID_LEN + 4] = 0xff;
        assert_eq!(
            MethodPayload::decode(&invalid_utf8).unwrap_err(),
            MethodError::InvalidUtf8
        );
        let mut excessive_count = method().encode();
        let input_count = ID_LEN + 4 + method().procedure.len();
        excessive_count[input_count..input_count + 2]
            .copy_from_slice(&((MAX_LIST_ITEMS + 1) as u16).to_be_bytes());
        assert_eq!(
            MethodPayload::decode(&excessive_count).unwrap_err(),
            MethodError::TooManyItems
        );
        let minimal = MethodPayload::new(
            record_id(1),
            "P".into(),
            vec![],
            vec![],
            identity(2),
            DigestAlgorithm::Sha256,
            [3; ID_LEN],
            None,
        )
        .unwrap();
        let algorithm_index = ID_LEN + 4 + 1 + 2 + 2 + ID_LEN;
        let mut unknown_algorithm = minimal.encode();
        unknown_algorithm[algorithm_index] = 0xff;
        assert_eq!(
            MethodPayload::decode(&unknown_algorithm).unwrap_err(),
            MethodError::UnknownDigestAlgorithm(0xff)
        );
        let mut marker = minimal.encode();
        *marker.last_mut().unwrap() = 2;
        assert_eq!(
            MethodPayload::decode(&marker).unwrap_err(),
            MethodError::InvalidOptionalMarker(2)
        );
    }

    #[test]
    fn every_truncation_and_trailing_byte_fails_closed() {
        let encoded = method().encode();
        for end in 0..encoded.len() {
            assert!(MethodPayload::decode(&encoded[..end]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            MethodPayload::decode(&trailing).unwrap_err(),
            MethodError::TrailingBytes
        );
    }

    #[test]
    fn oversized_payload_is_rejected_before_decoding_fields() {
        assert_eq!(
            MethodPayload::decode(&vec![0; MAX_EVIDENCE_PAYLOAD_LEN + 1]).unwrap_err(),
            MethodError::PayloadTooLarge
        );
    }
}
