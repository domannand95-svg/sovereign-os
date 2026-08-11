use crate::MAX_EVIDENCE_PAYLOAD_LEN;
use std::fmt;

pub const MAX_TEXT_FIELD_LEN: usize = 64 * 1024;
pub const MAX_LIST_ITEMS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectivePayload {
    statement: String,
    scope: String,
    success_criteria: Vec<String>,
    constraints: Vec<String>,
    expires_at: Option<u64>,
}

impl ObjectivePayload {
    pub fn new(
        statement: String,
        scope: String,
        success_criteria: Vec<String>,
        constraints: Vec<String>,
        expires_at: Option<u64>,
    ) -> Result<Self, ObjectiveError> {
        validate_text(&statement)?;
        validate_text(&scope)?;
        validate_list(&success_criteria, true)?;
        validate_list(&constraints, false)?;

        let payload = Self {
            statement,
            scope,
            success_criteria,
            constraints,
            expires_at,
        };
        if payload.encoded_len()? > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(ObjectiveError::PayloadTooLarge);
        }
        Ok(payload)
    }

    pub fn statement(&self) -> &str {
        &self.statement
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn success_criteria(&self) -> &[String] {
        &self.success_criteria
    }

    pub fn constraints(&self) -> &[String] {
        &self.constraints
    }

    pub const fn expires_at(&self) -> Option<u64> {
        self.expires_at
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(
            self.encoded_len()
                .expect("validated Objective payload length cannot overflow"),
        );
        encode_text(&mut encoded, &self.statement);
        encode_text(&mut encoded, &self.scope);
        encode_text_list(&mut encoded, &self.success_criteria);
        encode_text_list(&mut encoded, &self.constraints);
        match self.expires_at {
            None => encoded.push(0),
            Some(timestamp) => {
                encoded.push(1);
                encoded.extend_from_slice(&timestamp.to_be_bytes());
            }
        }
        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ObjectiveError> {
        if bytes.len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(ObjectiveError::PayloadTooLarge);
        }

        let mut decoder = Decoder::new(bytes);
        let statement = decoder.read_text()?;
        let scope = decoder.read_text()?;
        let success_criteria = decoder.read_text_list(true)?;
        let constraints = decoder.read_text_list(false)?;
        let expires_at = match decoder.read_u8()? {
            0 => None,
            1 => Some(decoder.read_u64()?),
            marker => return Err(ObjectiveError::InvalidOptionalMarker(marker)),
        };
        if !decoder.is_finished() {
            return Err(ObjectiveError::TrailingBytes);
        }

        Self::new(statement, scope, success_criteria, constraints, expires_at)
    }

    fn encoded_len(&self) -> Result<usize, ObjectiveError> {
        let mut length = 0_usize;
        length = checked_add(length, 4 + self.statement.len())?;
        length = checked_add(length, 4 + self.scope.len())?;
        length = checked_add(length, 2)?;
        for item in &self.success_criteria {
            length = checked_add(length, 4 + item.len())?;
        }
        length = checked_add(length, 2)?;
        for item in &self.constraints {
            length = checked_add(length, 4 + item.len())?;
        }
        checked_add(length, if self.expires_at.is_some() { 9 } else { 1 })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectiveError {
    EmptyText,
    TextTooLarge,
    NulText,
    InvalidUtf8,
    EmptySuccessCriteria,
    TooManyItems,
    PayloadTooLarge,
    InvalidOptionalMarker(u8),
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for ObjectiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ObjectiveError {}

fn checked_add(left: usize, right: usize) -> Result<usize, ObjectiveError> {
    left.checked_add(right)
        .ok_or(ObjectiveError::PayloadTooLarge)
}

fn validate_text(value: &str) -> Result<(), ObjectiveError> {
    if value.is_empty() {
        return Err(ObjectiveError::EmptyText);
    }
    if value.len() > MAX_TEXT_FIELD_LEN {
        return Err(ObjectiveError::TextTooLarge);
    }
    if value.as_bytes().contains(&0) {
        return Err(ObjectiveError::NulText);
    }
    Ok(())
}

fn validate_list(values: &[String], required: bool) -> Result<(), ObjectiveError> {
    if required && values.is_empty() {
        return Err(ObjectiveError::EmptySuccessCriteria);
    }
    if values.len() > MAX_LIST_ITEMS {
        return Err(ObjectiveError::TooManyItems);
    }
    for value in values {
        validate_text(value)?;
    }
    Ok(())
}

fn encode_text(encoded: &mut Vec<u8>, value: &str) {
    encoded.extend_from_slice(&(value.len() as u32).to_be_bytes());
    encoded.extend_from_slice(value.as_bytes());
}

fn encode_text_list(encoded: &mut Vec<u8>, values: &[String]) {
    encoded.extend_from_slice(&(values.len() as u16).to_be_bytes());
    for value in values {
        encode_text(encoded, value);
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

    fn is_finished(&self) -> bool {
        self.cursor == self.bytes.len()
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ObjectiveError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(ObjectiveError::TruncatedEncoding)?;
        if end > self.bytes.len() {
            return Err(ObjectiveError::TruncatedEncoding);
        }
        let value = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, ObjectiveError> {
        Ok(self.take(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, ObjectiveError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, ObjectiveError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, ObjectiveError> {
        let bytes = self.take(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_text(&mut self) -> Result<String, ObjectiveError> {
        let length = self.read_u32()? as usize;
        if length > MAX_TEXT_FIELD_LEN {
            return Err(ObjectiveError::TextTooLarge);
        }
        let bytes = self.take(length)?;
        let value = std::str::from_utf8(bytes).map_err(|_| ObjectiveError::InvalidUtf8)?;
        validate_text(value)?;
        Ok(value.to_owned())
    }

    fn read_text_list(&mut self, required: bool) -> Result<Vec<String>, ObjectiveError> {
        let count = self.read_u16()? as usize;
        if count > MAX_LIST_ITEMS {
            return Err(ObjectiveError::TooManyItems);
        }
        if required && count == 0 {
            return Err(ObjectiveError::EmptySuccessCriteria);
        }
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.read_text()?);
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn objective() -> ObjectivePayload {
        ObjectivePayload::new(
            "Establish a bounded result".to_owned(),
            "Includes deterministic encoding; excludes admission".to_owned(),
            vec!["Round trips byte-identically".to_owned()],
            vec!["No runtime authority".to_owned()],
            Some(1_786_406_400),
        )
        .unwrap()
    }

    #[test]
    fn objective_round_trips_canonically() {
        let original = objective();
        let encoded = original.encode();
        let decoded = ObjectivePayload::decode(&encoded).unwrap();

        assert_eq!(decoded, original);
        assert_eq!(decoded.encode(), encoded);
        assert_eq!(decoded.statement(), "Establish a bounded result");
        assert!(decoded.scope().contains("excludes admission"));
        assert_eq!(decoded.success_criteria().len(), 1);
        assert_eq!(decoded.constraints().len(), 1);
        assert_eq!(decoded.expires_at(), Some(1_786_406_400));
    }

    #[test]
    fn fixed_vector_is_stable_and_big_endian() {
        let payload = ObjectivePayload::new(
            "S".to_owned(),
            "P".to_owned(),
            vec!["C".to_owned()],
            vec!["K".to_owned()],
            Some(42),
        )
        .unwrap();
        let expected = [
            0, 0, 0, 1, b'S', 0, 0, 0, 1, b'P', 0, 1, 0, 0, 0, 1, b'C', 0, 1, 0, 0, 0, 1, b'K', 1,
            0, 0, 0, 0, 0, 0, 0, 42,
        ];

        assert_eq!(payload.encode(), expected);
        assert_eq!(ObjectivePayload::decode(&expected).unwrap(), payload);
    }

    #[test]
    fn required_empty_and_nul_text_are_rejected() {
        assert_eq!(
            ObjectivePayload::new(
                String::new(),
                "scope".to_owned(),
                vec!["criterion".to_owned()],
                vec![],
                None,
            )
            .unwrap_err(),
            ObjectiveError::EmptyText
        );
        assert_eq!(
            ObjectivePayload::new(
                "statement".to_owned(),
                "scope\0hidden".to_owned(),
                vec!["criterion".to_owned()],
                vec![],
                None,
            )
            .unwrap_err(),
            ObjectiveError::NulText
        );
    }

    #[test]
    fn text_and_list_limits_are_enforced() {
        assert_eq!(
            ObjectivePayload::new(
                "x".repeat(MAX_TEXT_FIELD_LEN + 1),
                "scope".to_owned(),
                vec!["criterion".to_owned()],
                vec![],
                None,
            )
            .unwrap_err(),
            ObjectiveError::TextTooLarge
        );
        assert_eq!(
            ObjectivePayload::new(
                "statement".to_owned(),
                "scope".to_owned(),
                vec!["criterion".to_owned(); MAX_LIST_ITEMS + 1],
                vec![],
                None,
            )
            .unwrap_err(),
            ObjectiveError::TooManyItems
        );
        assert_eq!(
            ObjectivePayload::new(
                "statement".to_owned(),
                "scope".to_owned(),
                vec![],
                vec![],
                None,
            )
            .unwrap_err(),
            ObjectiveError::EmptySuccessCriteria
        );
    }

    #[test]
    fn overall_payload_limit_is_enforced() {
        let large = "x".repeat(MAX_TEXT_FIELD_LEN);
        assert_eq!(
            ObjectivePayload::new(
                large.clone(),
                large.clone(),
                vec![large.clone(); 14],
                vec![],
                None,
            )
            .unwrap_err(),
            ObjectiveError::PayloadTooLarge
        );
    }

    #[test]
    fn decoder_rejects_invalid_utf8_and_declared_limits_before_allocation() {
        let invalid_utf8 = [0, 0, 0, 1, 0xff];
        assert_eq!(
            ObjectivePayload::decode(&invalid_utf8).unwrap_err(),
            ObjectiveError::InvalidUtf8
        );

        let declared_oversize = ((MAX_TEXT_FIELD_LEN + 1) as u32).to_be_bytes();
        assert_eq!(
            ObjectivePayload::decode(&declared_oversize).unwrap_err(),
            ObjectiveError::TextTooLarge
        );

        let mut too_many = objective().encode();
        let criteria_count_offset = 4 + objective().statement.len() + 4 + objective().scope.len();
        too_many[criteria_count_offset..criteria_count_offset + 2]
            .copy_from_slice(&((MAX_LIST_ITEMS + 1) as u16).to_be_bytes());
        assert_eq!(
            ObjectivePayload::decode(&too_many).unwrap_err(),
            ObjectiveError::TooManyItems
        );
    }

    #[test]
    fn optional_timestamp_marker_is_canonical() {
        let mut encoded = objective().encode();
        let marker = encoded.len() - 9;
        encoded[marker] = 2;
        assert_eq!(
            ObjectivePayload::decode(&encoded).unwrap_err(),
            ObjectiveError::InvalidOptionalMarker(2)
        );

        let absent = ObjectivePayload::new(
            "statement".to_owned(),
            "scope".to_owned(),
            vec!["criterion".to_owned()],
            vec![],
            None,
        )
        .unwrap();
        assert_eq!(absent.encode().last(), Some(&0));
        assert_eq!(ObjectivePayload::decode(&absent.encode()).unwrap(), absent);
    }

    #[test]
    fn every_truncation_and_trailing_byte_fails_closed() {
        let encoded = objective().encode();
        for end in 0..encoded.len() {
            assert!(ObjectivePayload::decode(&encoded[..end]).is_err());
        }

        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            ObjectivePayload::decode(&trailing).unwrap_err(),
            ObjectiveError::TrailingBytes
        );
    }

    #[test]
    fn maximum_list_count_is_accepted() {
        let payload = ObjectivePayload::new(
            "statement".to_owned(),
            "scope".to_owned(),
            vec!["criterion".to_owned(); MAX_LIST_ITEMS],
            vec!["constraint".to_owned(); MAX_LIST_ITEMS],
            Some(u64::MAX),
        )
        .unwrap();

        assert_eq!(
            ObjectivePayload::decode(&payload.encode()).unwrap(),
            payload
        );
    }
}
