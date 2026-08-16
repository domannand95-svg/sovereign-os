use crate::{RecordId, MAX_EVIDENCE_PAYLOAD_LEN, MAX_TEXT_FIELD_LEN};
use std::cmp::Ordering;
use std::fmt;

const ID_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum UncertaintyKind {
    Measurement = 0x00,
    Model = 0x01,
    Sampling = 0x02,
    Source = 0x03,
    Interpretation = 0x04,
    Unknown = 0x05,
}

impl UncertaintyKind {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self, UncertaintyError> {
        match value {
            0x00 => Ok(Self::Measurement),
            0x01 => Ok(Self::Model),
            0x02 => Ok(Self::Sampling),
            0x03 => Ok(Self::Source),
            0x04 => Ok(Self::Interpretation),
            0x05 => Ok(Self::Unknown),
            _ => Err(UncertaintyError::UnknownUncertaintyKind(value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncertaintyPayload {
    about_id: RecordId,
    uncertainty_kind: UncertaintyKind,
    statement: String,
    lower_bound: Option<String>,
    upper_bound: Option<String>,
    confidence_basis: String,
}

impl UncertaintyPayload {
    pub fn new(
        about_id: RecordId,
        uncertainty_kind: UncertaintyKind,
        statement: String,
        lower_bound: Option<String>,
        upper_bound: Option<String>,
        confidence_basis: String,
    ) -> Result<Self, UncertaintyError> {
        if about_id.is_zero() {
            return Err(UncertaintyError::ZeroRecordId);
        }
        validate_text(&statement)?;
        validate_text(&confidence_basis)?;
        match (&lower_bound, &upper_bound) {
            (None, None) => {}
            (Some(lower), Some(upper)) => {
                validate_decimal(lower)?;
                validate_decimal(upper)?;
                if compare_decimals(lower, upper) == Ordering::Greater {
                    return Err(UncertaintyError::ReversedBounds);
                }
            }
            _ => return Err(UncertaintyError::IncompleteBounds),
        }

        let payload = Self {
            about_id,
            uncertainty_kind,
            statement,
            lower_bound,
            upper_bound,
            confidence_basis,
        };
        if payload.encoded_len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(UncertaintyError::PayloadTooLarge);
        }
        Ok(payload)
    }

    pub const fn about_id(&self) -> RecordId {
        self.about_id
    }
    pub const fn uncertainty_kind(&self) -> UncertaintyKind {
        self.uncertainty_kind
    }
    pub fn statement(&self) -> &str {
        &self.statement
    }
    pub fn lower_bound(&self) -> Option<&str> {
        self.lower_bound.as_deref()
    }
    pub fn upper_bound(&self) -> Option<&str> {
        self.upper_bound.as_deref()
    }
    pub fn confidence_basis(&self) -> &str {
        &self.confidence_basis
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(self.encoded_len());
        encoded.extend_from_slice(self.about_id.as_bytes());
        encoded.push(self.uncertainty_kind.as_u8());
        encode_text(&mut encoded, &self.statement);
        match (&self.lower_bound, &self.upper_bound) {
            (None, None) => encoded.push(0),
            (Some(lower), Some(upper)) => {
                encoded.push(1);
                encode_text(&mut encoded, lower);
                encode_text(&mut encoded, upper);
            }
            _ => unreachable!("validated bounds are always an atomic pair"),
        }
        encode_text(&mut encoded, &self.confidence_basis);
        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, UncertaintyError> {
        if bytes.len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(UncertaintyError::PayloadTooLarge);
        }
        let mut decoder = Decoder::new(bytes);
        let about_id = decoder.read_record_id()?;
        let uncertainty_kind = UncertaintyKind::from_u8(decoder.read_u8()?)?;
        let statement = decoder.read_text()?;
        let (lower_bound, upper_bound) = match decoder.read_u8()? {
            0 => (None, None),
            1 => (Some(decoder.read_text()?), Some(decoder.read_text()?)),
            marker => return Err(UncertaintyError::InvalidOptionalMarker(marker)),
        };
        let confidence_basis = decoder.read_text()?;
        if !decoder.is_finished() {
            return Err(UncertaintyError::TrailingBytes);
        }
        Self::new(
            about_id,
            uncertainty_kind,
            statement,
            lower_bound,
            upper_bound,
            confidence_basis,
        )
    }

    fn encoded_len(&self) -> usize {
        ID_LEN
            + 1
            + 4
            + self.statement.len()
            + 1
            + self.lower_bound.as_ref().map_or(0, |value| 4 + value.len())
            + self.upper_bound.as_ref().map_or(0, |value| 4 + value.len())
            + 4
            + self.confidence_basis.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UncertaintyError {
    EmptyText,
    NulText,
    TextTooLarge,
    InvalidUtf8,
    ZeroRecordId,
    UnknownUncertaintyKind(u8),
    InvalidOptionalMarker(u8),
    IncompleteBounds,
    InvalidDecimal,
    ReversedBounds,
    PayloadTooLarge,
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for UncertaintyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for UncertaintyError {}

fn validate_text(value: &str) -> Result<(), UncertaintyError> {
    if value.is_empty() {
        return Err(UncertaintyError::EmptyText);
    }
    if value.len() > MAX_TEXT_FIELD_LEN {
        return Err(UncertaintyError::TextTooLarge);
    }
    if value.contains('\0') {
        return Err(UncertaintyError::NulText);
    }
    Ok(())
}

fn validate_decimal(value: &str) -> Result<(), UncertaintyError> {
    if value.is_empty() || value.len() > MAX_TEXT_FIELD_LEN || !value.is_ascii() {
        return Err(UncertaintyError::InvalidDecimal);
    }
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    if unsigned.is_empty() || value.starts_with('+') {
        return Err(UncertaintyError::InvalidDecimal);
    }
    let mut components = unsigned.split('.');
    let integer = components.next().unwrap_or_default();
    let fraction = components.next();
    if components.next().is_some()
        || integer.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || (integer.len() > 1 && integer.starts_with('0'))
    {
        return Err(UncertaintyError::InvalidDecimal);
    }
    if let Some(fraction) = fraction {
        if fraction.is_empty()
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
            || fraction.ends_with('0')
        {
            return Err(UncertaintyError::InvalidDecimal);
        }
    }
    if value.starts_with('-') && integer == "0" && fraction.is_none() {
        return Err(UncertaintyError::InvalidDecimal);
    }
    Ok(())
}

fn compare_decimals(left: &str, right: &str) -> Ordering {
    let left_negative = left.starts_with('-');
    let right_negative = right.starts_with('-');
    match (left_negative, right_negative) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }

    let absolute_order = compare_absolute(
        left.strip_prefix('-').unwrap_or(left),
        right.strip_prefix('-').unwrap_or(right),
    );
    if left_negative {
        absolute_order.reverse()
    } else {
        absolute_order
    }
}

fn compare_absolute(left: &str, right: &str) -> Ordering {
    let (left_integer, left_fraction) = left.split_once('.').unwrap_or((left, ""));
    let (right_integer, right_fraction) = right.split_once('.').unwrap_or((right, ""));
    left_integer
        .len()
        .cmp(&right_integer.len())
        .then_with(|| left_integer.cmp(right_integer))
        .then_with(|| {
            let length = left_fraction.len().max(right_fraction.len());
            left_fraction
                .bytes()
                .chain(std::iter::repeat(b'0'))
                .take(length)
                .cmp(
                    right_fraction
                        .bytes()
                        .chain(std::iter::repeat(b'0'))
                        .take(length),
                )
        })
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

    fn read_u8(&mut self) -> Result<u8, UncertaintyError> {
        let value = *self
            .bytes
            .get(self.cursor)
            .ok_or(UncertaintyError::TruncatedEncoding)?;
        self.cursor += 1;
        Ok(value)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], UncertaintyError> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(UncertaintyError::TruncatedEncoding)?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(UncertaintyError::TruncatedEncoding)?;
        let mut value = [0_u8; N];
        value.copy_from_slice(bytes);
        self.cursor = end;
        Ok(value)
    }

    fn read_text(&mut self) -> Result<String, UncertaintyError> {
        let length = u32::from_be_bytes(self.read_array()?) as usize;
        if length > MAX_TEXT_FIELD_LEN {
            return Err(UncertaintyError::TextTooLarge);
        }
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(UncertaintyError::TruncatedEncoding)?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(UncertaintyError::TruncatedEncoding)?;
        let value = std::str::from_utf8(bytes)
            .map_err(|_| UncertaintyError::InvalidUtf8)?
            .to_owned();
        self.cursor = end;
        Ok(value)
    }

    fn read_record_id(&mut self) -> Result<RecordId, UncertaintyError> {
        Ok(RecordId::from_bytes(self.read_array()?))
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

    fn record_id(seed: u8) -> RecordId {
        RecordId::from_bytes([seed; ID_LEN])
    }
    fn identity(seed: u8) -> IdentityId {
        IdentityId::from_bytes([seed; ID_LEN])
    }
    fn uncertainty() -> UncertaintyPayload {
        UncertaintyPayload::new(
            record_id(1),
            UncertaintyKind::Model,
            "Model interval".into(),
            Some("-0.125".into()),
            Some("10.25".into()),
            "Declared calibration study".into(),
        )
        .unwrap()
    }

    #[test]
    fn round_trip_preserves_bounds_and_every_kind() {
        let payload = uncertainty();
        assert_eq!(
            UncertaintyPayload::decode(&payload.encode()).unwrap(),
            payload
        );
        assert_eq!(payload.lower_bound(), Some("-0.125"));
        assert_eq!(payload.upper_bound(), Some("10.25"));
        for kind in [
            UncertaintyKind::Measurement,
            UncertaintyKind::Model,
            UncertaintyKind::Sampling,
            UncertaintyKind::Source,
            UncertaintyKind::Interpretation,
            UncertaintyKind::Unknown,
        ] {
            let value =
                UncertaintyPayload::new(record_id(1), kind, "S".into(), None, None, "B".into())
                    .unwrap();
            assert_eq!(UncertaintyPayload::decode(&value.encode()).unwrap(), value);
        }
    }

    #[test]
    fn minimal_fixed_vector_is_stable_and_big_endian() {
        let payload = UncertaintyPayload::new(
            record_id(1),
            UncertaintyKind::Measurement,
            "U".into(),
            None,
            None,
            "B".into(),
        )
        .unwrap();
        let mut expected = vec![1; ID_LEN];
        expected.extend_from_slice(&[0, 0, 0, 0, 1, b'U', 0, 0, 0, 0, 1, b'B']);
        assert_eq!(payload.encode(), expected);
        assert_eq!(UncertaintyPayload::decode(&expected).unwrap(), payload);
    }

    #[test]
    fn governed_uncertainty_record_identity_is_stable() {
        let record = EvidenceRecord::new_uncertainty(
            identity(9),
            identity(10),
            identity(11),
            vec![record_id(12)],
            uncertainty(),
        )
        .unwrap();
        assert_eq!(
            record.id().to_string(),
            "d692732f05cf162c2984d79c30d6fef02332271024934d37f4a97be9e4542956"
        );
        let decoded = EvidenceRecord::decode(&record.encode()).unwrap();
        assert_eq!(decoded.id(), record.id());
        assert_eq!(decoded.decode_uncertainty_payload().unwrap(), uncertainty());
    }

    #[test]
    fn canonical_decimal_grammar_accepts_only_one_representation() {
        for value in ["0", "1", "-1", "10", "0.01", "-0.01", "10.25", "-10.25"] {
            assert_eq!(validate_decimal(value), Ok(()), "{value}");
        }
        for value in [
            "", "+1", "01", "-01", ".5", "-.5", "1.", "1.0", "1.20", "-0", "-0.0", "1e2", "NaN",
            "inf", "Infinity", " 1", "1 ", "1,5", "١",
        ] {
            assert_eq!(
                validate_decimal(value),
                Err(UncertaintyError::InvalidDecimal),
                "{value}"
            );
        }
    }

    #[test]
    fn exact_decimal_ordering_handles_sign_scale_and_equal_values() {
        for (lower, upper) in [
            ("0", "0"),
            ("0", "0.01"),
            ("0.01", "0.1"),
            ("0.9", "1"),
            ("2", "10"),
            ("-10", "-2"),
            ("-1", "-0.01"),
            ("-0.01", "0"),
        ] {
            assert_ne!(
                compare_decimals(lower, upper),
                Ordering::Greater,
                "{lower} {upper}"
            );
        }
        for (lower, upper) in [("1", "0.9"), ("10", "2"), ("-2", "-10"), ("0", "-0.01")] {
            assert_eq!(
                UncertaintyPayload::new(
                    record_id(1),
                    UncertaintyKind::Measurement,
                    "S".into(),
                    Some(lower.into()),
                    Some(upper.into()),
                    "B".into(),
                )
                .unwrap_err(),
                UncertaintyError::ReversedBounds
            );
        }
    }

    #[test]
    fn required_fields_and_atomic_bounds_fail_closed() {
        let make = |about_id, statement, lower, upper, basis| {
            UncertaintyPayload::new(
                about_id,
                UncertaintyKind::Measurement,
                statement,
                lower,
                upper,
                basis,
            )
        };
        assert_eq!(
            make(
                RecordId::from_bytes([0; ID_LEN]),
                "S".into(),
                None,
                None,
                "B".into()
            )
            .unwrap_err(),
            UncertaintyError::ZeroRecordId
        );
        assert_eq!(
            make(record_id(1), "".into(), None, None, "B".into()).unwrap_err(),
            UncertaintyError::EmptyText
        );
        assert_eq!(
            make(record_id(1), "S".into(), None, None, "".into()).unwrap_err(),
            UncertaintyError::EmptyText
        );
        assert_eq!(
            make(record_id(1), "S\0".into(), None, None, "B".into()).unwrap_err(),
            UncertaintyError::NulText
        );
        assert_eq!(
            make(record_id(1), "S".into(), Some("0".into()), None, "B".into()).unwrap_err(),
            UncertaintyError::IncompleteBounds
        );
        assert_eq!(
            make(record_id(1), "S".into(), None, Some("1".into()), "B".into()).unwrap_err(),
            UncertaintyError::IncompleteBounds
        );
    }

    #[test]
    fn text_boundaries_are_enforced() {
        assert!(UncertaintyPayload::new(
            record_id(1),
            UncertaintyKind::Unknown,
            "x".repeat(MAX_TEXT_FIELD_LEN),
            Some("0".into()),
            Some("1".into()),
            "y".repeat(MAX_TEXT_FIELD_LEN),
        )
        .is_ok());
        assert_eq!(
            UncertaintyPayload::new(
                record_id(1),
                UncertaintyKind::Unknown,
                "x".repeat(MAX_TEXT_FIELD_LEN + 1),
                None,
                None,
                "B".into(),
            )
            .unwrap_err(),
            UncertaintyError::TextTooLarge
        );
        assert_eq!(
            UncertaintyPayload::new(
                record_id(1),
                UncertaintyKind::Unknown,
                "S".into(),
                Some("1".repeat(MAX_TEXT_FIELD_LEN + 1)),
                Some("2".into()),
                "B".into(),
            )
            .unwrap_err(),
            UncertaintyError::InvalidDecimal
        );
    }

    #[test]
    fn decoder_rejects_malformed_input_and_unknown_discriminants() {
        let minimal = UncertaintyPayload::new(
            record_id(1),
            UncertaintyKind::Measurement,
            "U".into(),
            None,
            None,
            "B".into(),
        )
        .unwrap();
        let mut invalid_utf8 = minimal.encode();
        invalid_utf8[ID_LEN + 1 + 4] = 0xff;
        assert_eq!(
            UncertaintyPayload::decode(&invalid_utf8).unwrap_err(),
            UncertaintyError::InvalidUtf8
        );
        let mut unknown_kind = minimal.encode();
        unknown_kind[ID_LEN] = 0xff;
        assert_eq!(
            UncertaintyPayload::decode(&unknown_kind).unwrap_err(),
            UncertaintyError::UnknownUncertaintyKind(0xff)
        );
        let mut invalid_marker = minimal.encode();
        invalid_marker[ID_LEN + 1 + 4 + 1] = 2;
        assert_eq!(
            UncertaintyPayload::decode(&invalid_marker).unwrap_err(),
            UncertaintyError::InvalidOptionalMarker(2)
        );
        let mut excessive_length = minimal.encode();
        excessive_length[ID_LEN + 1..ID_LEN + 5]
            .copy_from_slice(&((MAX_TEXT_FIELD_LEN + 1) as u32).to_be_bytes());
        assert_eq!(
            UncertaintyPayload::decode(&excessive_length).unwrap_err(),
            UncertaintyError::TextTooLarge
        );
    }

    #[test]
    fn every_truncation_and_trailing_byte_fails_closed() {
        let encoded = uncertainty().encode();
        for end in 0..encoded.len() {
            assert!(
                UncertaintyPayload::decode(&encoded[..end]).is_err(),
                "accepted {end}"
            );
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            UncertaintyPayload::decode(&trailing).unwrap_err(),
            UncertaintyError::TrailingBytes
        );
    }

    #[test]
    fn oversized_payload_is_rejected_before_decoding_fields() {
        assert_eq!(
            UncertaintyPayload::decode(&vec![0; MAX_EVIDENCE_PAYLOAD_LEN + 1]).unwrap_err(),
            UncertaintyError::PayloadTooLarge
        );
    }
}
