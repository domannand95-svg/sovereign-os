use serde::{Deserialize, Serialize};
use std::fmt;

const ACTION_MAGIC: &[u8; 6] = b"SOVACT";
const ACTION_VERSION: u8 = 1;
const MAX_ACTION_KIND_BYTES: usize = 64;
const MAX_TARGET_BYTES: usize = 1_024;
const MAX_PAYLOAD_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryParseError {
    EmptyInput,
    Truncated,
    InvalidMagic,
    UnsupportedVersion(u8),
    InvalidLength,
    TrailingBytes,
    InvalidUtf8,
    InvalidIdentifier,
    InvalidActionKind,
    InvalidTarget,
    InvalidDigest,
    EmptyPayload,
    PayloadTooLarge,
    DigestMismatch,
    NonCanonicalEncoding,
}

impl fmt::Display for BoundaryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for BoundaryParseError {}

fn valid_reference(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ExecutionId(String);

impl ExecutionId {
    pub fn parse(value: impl Into<String>) -> Result<Self, BoundaryParseError> {
        let value = value.into();
        if !valid_reference(&value, 128) {
            return Err(BoundaryParseError::InvalidIdentifier);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ExecutionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DigestRef([u8; 32]);

impl DigestRef {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn parse_hex(value: &str) -> Result<Self, BoundaryParseError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(BoundaryParseError::InvalidDigest);
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let pair = std::str::from_utf8(pair).map_err(|_| BoundaryParseError::InvalidDigest)?;
            bytes[index] =
                u8::from_str_radix(pair, 16).map_err(|_| BoundaryParseError::InvalidDigest)?;
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthorizationReceiptRef(DigestRef);

impl AuthorizationReceiptRef {
    pub fn parse_hex(value: &str) -> Result<Self, BoundaryParseError> {
        DigestRef::parse_hex(value).map(Self)
    }

    pub fn digest(&self) -> DigestRef {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalAction(Box<[u8]>);

impl CanonicalAction {
    pub fn encode(
        action_kind: &str,
        target: &str,
        payload_digest: DigestRef,
        payload: &[u8],
    ) -> Result<Self, BoundaryParseError> {
        validate_parts(action_kind, target, payload_digest, payload)?;
        Ok(Self(
            encode_parts(action_kind, target, payload_digest, payload).into_boxed_slice(),
        ))
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, BoundaryParseError> {
        if bytes.is_empty() {
            return Err(BoundaryParseError::EmptyInput);
        }
        let mut cursor = Cursor::new(bytes);
        if cursor.take(ACTION_MAGIC.len())? != ACTION_MAGIC {
            return Err(BoundaryParseError::InvalidMagic);
        }
        let version = cursor.byte()?;
        if version != ACTION_VERSION {
            return Err(BoundaryParseError::UnsupportedVersion(version));
        }
        let action_kind = cursor.length_prefixed_u16(MAX_ACTION_KIND_BYTES)?;
        let target = cursor.length_prefixed_u16(MAX_TARGET_BYTES)?;
        let digest_bytes: [u8; 32] = cursor
            .take(32)?
            .try_into()
            .map_err(|_| BoundaryParseError::Truncated)?;
        let payload_len = cursor.u32()? as usize;
        if payload_len > MAX_PAYLOAD_BYTES {
            return Err(BoundaryParseError::PayloadTooLarge);
        }
        let payload = cursor.take(payload_len)?;
        if !cursor.is_finished() {
            return Err(BoundaryParseError::TrailingBytes);
        }
        let action_kind =
            std::str::from_utf8(action_kind).map_err(|_| BoundaryParseError::InvalidUtf8)?;
        let target = std::str::from_utf8(target).map_err(|_| BoundaryParseError::InvalidUtf8)?;
        let digest = DigestRef::from_bytes(digest_bytes);
        validate_parts(action_kind, target, digest, payload)?;
        let encoded = encode_parts(action_kind, target, digest, payload);
        if encoded.as_slice() != bytes {
            return Err(BoundaryParseError::NonCanonicalEncoding);
        }
        Ok(Self(encoded.into_boxed_slice()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CanonicalAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Self::parse(&bytes).map_err(serde::de::Error::custom)
    }
}

fn validate_parts(
    action_kind: &str,
    target: &str,
    payload_digest: DigestRef,
    payload: &[u8],
) -> Result<(), BoundaryParseError> {
    if !valid_reference(action_kind, MAX_ACTION_KIND_BYTES) {
        return Err(BoundaryParseError::InvalidActionKind);
    }
    if target.is_empty() || target.len() > MAX_TARGET_BYTES || target.chars().any(char::is_control)
    {
        return Err(BoundaryParseError::InvalidTarget);
    }
    if payload.is_empty() {
        return Err(BoundaryParseError::EmptyPayload);
    }
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(BoundaryParseError::PayloadTooLarge);
    }
    use sha2::Digest;
    let actual: [u8; 32] = sha2::Sha256::digest(payload).into();
    if actual != *payload_digest.as_bytes() {
        return Err(BoundaryParseError::DigestMismatch);
    }
    Ok(())
}

fn encode_parts(
    action_kind: &str,
    target: &str,
    payload_digest: DigestRef,
    payload: &[u8],
) -> Vec<u8> {
    let mut output = Vec::with_capacity(
        ACTION_MAGIC.len() + 1 + 2 + action_kind.len() + 2 + target.len() + 32 + 4 + payload.len(),
    );
    output.extend_from_slice(ACTION_MAGIC);
    output.push(ACTION_VERSION);
    output.extend_from_slice(&(action_kind.len() as u16).to_be_bytes());
    output.extend_from_slice(action_kind.as_bytes());
    output.extend_from_slice(&(target.len() as u16).to_be_bytes());
    output.extend_from_slice(target.as_bytes());
    output.extend_from_slice(payload_digest.as_bytes());
    output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    output.extend_from_slice(payload);
    output
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], BoundaryParseError> {
        let end = self
            .position
            .checked_add(len)
            .ok_or(BoundaryParseError::InvalidLength)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(BoundaryParseError::Truncated)?;
        self.position = end;
        Ok(value)
    }

    fn byte(&mut self) -> Result<u8, BoundaryParseError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, BoundaryParseError> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| BoundaryParseError::Truncated)?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, BoundaryParseError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| BoundaryParseError::Truncated)?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn length_prefixed_u16(&mut self, max: usize) -> Result<&'a [u8], BoundaryParseError> {
        let len = self.u16()? as usize;
        if len == 0 || len > max {
            return Err(BoundaryParseError::InvalidLength);
        }
        self.take(len)
    }

    fn is_finished(&self) -> bool {
        self.position == self.bytes.len()
    }
}
