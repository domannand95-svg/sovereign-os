use crate::{MAX_EVIDENCE_PAYLOAD_LEN, MAX_TEXT_FIELD_LEN};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DigestAlgorithm {
    Sha256 = 0x01,
    Blake3 = 0x02,
}

impl DigestAlgorithm {
    fn decode(value: u8) -> Result<Self, SourceError> {
        match value {
            0x01 => Ok(Self::Sha256),
            0x02 => Ok(Self::Blake3),
            _ => Err(SourceError::UnknownDigestAlgorithm(value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePayload {
    locator: String,
    digest_algorithm: DigestAlgorithm,
    content_digest: [u8; 32],
    retrieved_at: u64,
    media_type: String,
    publication_date_text: Option<String>,
}

impl SourcePayload {
    pub fn new(
        locator: String,
        digest_algorithm: DigestAlgorithm,
        content_digest: [u8; 32],
        retrieved_at: u64,
        media_type: String,
        publication_date_text: Option<String>,
    ) -> Result<Self, SourceError> {
        validate_text(&locator)?;
        validate_media_type(&media_type)?;
        if let Some(value) = publication_date_text.as_deref() {
            validate_text(value)?;
        }

        let payload = Self {
            locator,
            digest_algorithm,
            content_digest,
            retrieved_at,
            media_type,
            publication_date_text,
        };
        if payload.encoded_len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(SourceError::PayloadTooLarge);
        }
        Ok(payload)
    }

    pub fn locator(&self) -> &str {
        &self.locator
    }

    pub const fn digest_algorithm(&self) -> DigestAlgorithm {
        self.digest_algorithm
    }

    pub const fn content_digest(&self) -> &[u8; 32] {
        &self.content_digest
    }

    pub const fn retrieved_at(&self) -> u64 {
        self.retrieved_at
    }

    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    pub fn publication_date_text(&self) -> Option<&str> {
        self.publication_date_text.as_deref()
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(self.encoded_len());
        encode_text(&mut encoded, &self.locator);
        encoded.push(self.digest_algorithm as u8);
        encoded.extend_from_slice(&self.content_digest);
        encoded.extend_from_slice(&self.retrieved_at.to_be_bytes());
        encode_text(&mut encoded, &self.media_type);
        match &self.publication_date_text {
            None => encoded.push(0),
            Some(value) => {
                encoded.push(1);
                encode_text(&mut encoded, value);
            }
        }
        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(SourceError::PayloadTooLarge);
        }
        let mut decoder = Decoder::new(bytes);
        let locator = decoder.read_text()?;
        let digest_algorithm = DigestAlgorithm::decode(decoder.read_u8()?)?;
        let content_digest = decoder.read_digest()?;
        let retrieved_at = decoder.read_u64()?;
        let media_type = decoder.read_text()?;
        let publication_date_text = match decoder.read_u8()? {
            0 => None,
            1 => Some(decoder.read_text()?),
            marker => return Err(SourceError::InvalidOptionalMarker(marker)),
        };
        if !decoder.finished() {
            return Err(SourceError::TrailingBytes);
        }
        Self::new(
            locator,
            digest_algorithm,
            content_digest,
            retrieved_at,
            media_type,
            publication_date_text,
        )
    }

    fn encoded_len(&self) -> usize {
        4 + self.locator.len()
            + 1
            + 32
            + 8
            + 4
            + self.media_type.len()
            + 1
            + self
                .publication_date_text
                .as_ref()
                .map_or(0, |value| 4 + value.len())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceError {
    EmptyText,
    TextTooLarge,
    NulText,
    InvalidUtf8,
    UnknownDigestAlgorithm(u8),
    InvalidMediaType,
    InvalidOptionalMarker(u8),
    PayloadTooLarge,
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SourceError {}

fn validate_text(value: &str) -> Result<(), SourceError> {
    if value.is_empty() {
        return Err(SourceError::EmptyText);
    }
    if value.len() > MAX_TEXT_FIELD_LEN {
        return Err(SourceError::TextTooLarge);
    }
    if value.as_bytes().contains(&0) {
        return Err(SourceError::NulText);
    }
    Ok(())
}

fn validate_media_type(value: &str) -> Result<(), SourceError> {
    validate_text(value)?;
    if !value.is_ascii() || value.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err(SourceError::InvalidMediaType);
    }
    let mut parts = value.split('/');
    let Some(media_type) = parts.next() else {
        return Err(SourceError::InvalidMediaType);
    };
    let Some(subtype) = parts.next() else {
        return Err(SourceError::InvalidMediaType);
    };
    if parts.next().is_some()
        || media_type.is_empty()
        || subtype.is_empty()
        || !media_type.bytes().all(is_token_byte)
        || !subtype.bytes().all(is_token_byte)
    {
        return Err(SourceError::InvalidMediaType);
    }
    Ok(())
}

fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase()
        || byte.is_ascii_digit()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
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

    fn finished(&self) -> bool {
        self.cursor == self.bytes.len()
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], SourceError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(SourceError::TruncatedEncoding)?;
        if end > self.bytes.len() {
            return Err(SourceError::TruncatedEncoding);
        }
        let value = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, SourceError> {
        Ok(self.take(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, SourceError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, SourceError> {
        let bytes = self.take(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_digest(&mut self) -> Result<[u8; 32], SourceError> {
        let bytes = self.take(32)?;
        let mut digest = [0; 32];
        digest.copy_from_slice(bytes);
        Ok(digest)
    }

    fn read_text(&mut self) -> Result<String, SourceError> {
        let length = self.read_u32()? as usize;
        if length > MAX_TEXT_FIELD_LEN {
            return Err(SourceError::TextTooLarge);
        }
        let bytes = self.take(length)?;
        let value = std::str::from_utf8(bytes).map_err(|_| SourceError::InvalidUtf8)?;
        validate_text(value)?;
        Ok(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> SourcePayload {
        SourcePayload::new(
            "HTTPS://Example.invalid/paper/".to_owned(),
            DigestAlgorithm::Blake3,
            [0x42; 32],
            1_786_406_400,
            "application/pdf".to_owned(),
            Some("First published 12 March 1890".to_owned()),
        )
        .unwrap()
    }

    #[test]
    fn source_round_trips_and_preserves_exact_locator() {
        let original = source();
        let decoded = SourcePayload::decode(&original.encode()).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(decoded.encode(), original.encode());
        assert_eq!(decoded.locator(), "HTTPS://Example.invalid/paper/");
        assert_eq!(decoded.digest_algorithm(), DigestAlgorithm::Blake3);
        assert_eq!(decoded.content_digest(), &[0x42; 32]);
        assert_eq!(decoded.retrieved_at(), 1_786_406_400);
        assert_eq!(decoded.media_type(), "application/pdf");
        assert_eq!(
            decoded.publication_date_text(),
            Some("First published 12 March 1890")
        );
    }

    #[test]
    fn fixed_vector_is_stable_and_big_endian() {
        let payload = SourcePayload::new(
            "L".to_owned(),
            DigestAlgorithm::Sha256,
            [0x02; 32],
            42,
            "text/plain".to_owned(),
            None,
        )
        .unwrap();
        let mut expected = vec![0, 0, 0, 1, b'L', 1];
        expected.extend_from_slice(&[0x02; 32]);
        expected.extend_from_slice(&42_u64.to_be_bytes());
        expected.extend_from_slice(&[0, 0, 0, 10]);
        expected.extend_from_slice(b"text/plain");
        expected.push(0);
        assert_eq!(payload.encode(), expected);
        assert_eq!(SourcePayload::decode(&expected).unwrap(), payload);
    }

    #[test]
    fn algorithms_are_explicit_and_unknown_values_fail_closed() {
        let sha = SourcePayload::new(
            "L".into(),
            DigestAlgorithm::Sha256,
            [1; 32],
            0,
            "text/plain".into(),
            None,
        )
        .unwrap();
        let blake = SourcePayload::new(
            "L".into(),
            DigestAlgorithm::Blake3,
            [1; 32],
            0,
            "text/plain".into(),
            None,
        )
        .unwrap();
        assert_ne!(sha.encode(), blake.encode());
        let mut unknown = sha.encode();
        unknown[5] = 3;
        assert_eq!(
            SourcePayload::decode(&unknown).unwrap_err(),
            SourceError::UnknownDigestAlgorithm(3)
        );
    }

    #[test]
    fn media_type_is_strict_lowercase_bare_ascii() {
        for invalid in [
            "Text/plain",
            "text",
            "/plain",
            "text/",
            "text/plain/extra",
            "text/plain; charset=utf-8",
            "text /plain",
            "tęxt/plain",
        ] {
            assert_eq!(
                SourcePayload::new(
                    "L".into(),
                    DigestAlgorithm::Sha256,
                    [1; 32],
                    0,
                    invalid.into(),
                    None,
                )
                .unwrap_err(),
                SourceError::InvalidMediaType,
                "{invalid}"
            );
        }
        for valid in [
            "text/plain",
            "application/vnd.example+json",
            "image/svg+xml",
        ] {
            assert!(
                SourcePayload::new(
                    "L".into(),
                    DigestAlgorithm::Sha256,
                    [1; 32],
                    0,
                    valid.into(),
                    None,
                )
                .is_ok(),
                "{valid}"
            );
        }
    }

    #[test]
    fn required_and_optional_text_is_strict() {
        assert_eq!(
            SourcePayload::new(
                "".into(),
                DigestAlgorithm::Sha256,
                [1; 32],
                0,
                "text/plain".into(),
                None,
            )
            .unwrap_err(),
            SourceError::EmptyText
        );
        assert_eq!(
            SourcePayload::new(
                "L\0hidden".into(),
                DigestAlgorithm::Sha256,
                [1; 32],
                0,
                "text/plain".into(),
                None,
            )
            .unwrap_err(),
            SourceError::NulText
        );
        assert_eq!(
            SourcePayload::new(
                "L".into(),
                DigestAlgorithm::Sha256,
                [1; 32],
                0,
                "text/plain".into(),
                Some("".into()),
            )
            .unwrap_err(),
            SourceError::EmptyText
        );
        assert!(SourcePayload::new(
            "L".into(),
            DigestAlgorithm::Sha256,
            [1; 32],
            u64::MAX,
            "text/plain".into(),
            Some("1890".into()),
        )
        .is_ok());
    }

    #[test]
    fn decoder_rejects_declared_limits_invalid_utf8_and_markers() {
        let oversized = ((MAX_TEXT_FIELD_LEN + 1) as u32).to_be_bytes();
        assert_eq!(
            SourcePayload::decode(&oversized).unwrap_err(),
            SourceError::TextTooLarge
        );
        let invalid_utf8 = [0, 0, 0, 1, 0xff];
        assert_eq!(
            SourcePayload::decode(&invalid_utf8).unwrap_err(),
            SourceError::InvalidUtf8
        );
        let mut marker = source().encode();
        let marker_index =
            marker.len() - (1 + 4 + source().publication_date_text.as_ref().unwrap().len());
        marker[marker_index] = 2;
        assert_eq!(
            SourcePayload::decode(&marker).unwrap_err(),
            SourceError::InvalidOptionalMarker(2)
        );
    }

    #[test]
    fn every_truncation_and_trailing_byte_fails_closed() {
        let encoded = source().encode();
        for end in 0..encoded.len() {
            assert!(SourcePayload::decode(&encoded[..end]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            SourcePayload::decode(&trailing).unwrap_err(),
            SourceError::TrailingBytes
        );
    }
}
