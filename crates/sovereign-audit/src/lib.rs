//! Canonical governed-evidence records for Sovereign OS.
//!
//! This crate currently implements only the local, authority-free common
//! envelope defined by `SPEC-EV-001`. Admission and capability decisions live
//! outside this crate.

use sovereign_registry::IdentityId;
use std::collections::HashSet;
use std::fmt;

pub const EVIDENCE_SCHEMA_VERSION: u16 = 1;
pub const MAX_EVIDENCE_PARENTS: usize = 64;
pub const MAX_EVIDENCE_PAYLOAD_LEN: usize = 1024 * 1024;

const EVIDENCE_DOMAIN_SEPARATOR: &[u8] = b"SOVEREIGN_EVIDENCE_V1";
const ID_LEN: usize = 32;
const FIXED_ENVELOPE_LEN: usize = 2 + 1 + (3 * ID_LEN) + 2 + 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RecordKind {
    Objective = 0x01,
    Claim = 0x02,
    Source = 0x03,
    Method = 0x04,
    Uncertainty = 0x05,
    FailedAttempt = 0x06,
    ReviewerFinding = 0x07,
    Dispute = 0x08,
    Disposition = 0x09,
}

impl RecordKind {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self, EvidenceError> {
        match value {
            0x01 => Ok(Self::Objective),
            0x02 => Ok(Self::Claim),
            0x03 => Ok(Self::Source),
            0x04 => Ok(Self::Method),
            0x05 => Ok(Self::Uncertainty),
            0x06 => Ok(Self::FailedAttempt),
            0x07 => Ok(Self::ReviewerFinding),
            0x08 => Ok(Self::Dispute),
            0x09 => Ok(Self::Disposition),
            _ => Err(EvidenceError::UnknownRecordKind(value)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordId([u8; ID_LEN]);

impl RecordId {
    pub const fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|byte| *byte == 0)
    }
}

impl fmt::Display for RecordId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    kind: RecordKind,
    issuer_id: IdentityId,
    subject_id: IdentityId,
    policy_id: IdentityId,
    parent_ids: Vec<RecordId>,
    payload: Vec<u8>,
    id: RecordId,
}

impl EvidenceRecord {
    pub fn new(
        kind: RecordKind,
        issuer_id: IdentityId,
        subject_id: IdentityId,
        policy_id: IdentityId,
        parent_ids: Vec<RecordId>,
        payload: Vec<u8>,
    ) -> Result<Self, EvidenceError> {
        validate_parts(issuer_id, subject_id, policy_id, &parent_ids, &payload)?;
        let encoded = encode_parts(
            kind,
            issuer_id,
            subject_id,
            policy_id,
            &parent_ids,
            &payload,
        );
        let id = derive_record_id(&encoded);

        Ok(Self {
            kind,
            issuer_id,
            subject_id,
            policy_id,
            parent_ids,
            payload,
            id,
        })
    }

    pub const fn kind(&self) -> RecordKind {
        self.kind
    }

    pub const fn issuer_id(&self) -> IdentityId {
        self.issuer_id
    }

    pub const fn subject_id(&self) -> IdentityId {
        self.subject_id
    }

    pub const fn policy_id(&self) -> IdentityId {
        self.policy_id
    }

    pub fn parent_ids(&self) -> &[RecordId] {
        &self.parent_ids
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub const fn id(&self) -> RecordId {
        self.id
    }

    pub fn encode(&self) -> Vec<u8> {
        encode_parts(
            self.kind,
            self.issuer_id,
            self.subject_id,
            self.policy_id,
            &self.parent_ids,
            &self.payload,
        )
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, EvidenceError> {
        if bytes.len() < FIXED_ENVELOPE_LEN {
            return Err(EvidenceError::TruncatedEncoding);
        }

        let version = u16::from_be_bytes([bytes[0], bytes[1]]);
        if version != EVIDENCE_SCHEMA_VERSION {
            return Err(EvidenceError::UnsupportedVersion(version));
        }
        let kind = RecordKind::from_u8(bytes[2])?;
        let issuer_id = read_identity(bytes, 3);
        let subject_id = read_identity(bytes, 3 + ID_LEN);
        let policy_id = read_identity(bytes, 3 + (2 * ID_LEN));

        let parent_count_offset = 3 + (3 * ID_LEN);
        let parent_count =
            u16::from_be_bytes([bytes[parent_count_offset], bytes[parent_count_offset + 1]])
                as usize;
        if parent_count > MAX_EVIDENCE_PARENTS {
            return Err(EvidenceError::TooManyParents);
        }

        let parents_len = parent_count
            .checked_mul(ID_LEN)
            .ok_or(EvidenceError::TooManyParents)?;
        let payload_len_offset = parent_count_offset
            .checked_add(2)
            .and_then(|offset| offset.checked_add(parents_len))
            .ok_or(EvidenceError::TruncatedEncoding)?;
        let payload_header_end = payload_len_offset
            .checked_add(4)
            .ok_or(EvidenceError::TruncatedEncoding)?;
        if bytes.len() < payload_header_end {
            return Err(EvidenceError::TruncatedEncoding);
        }

        let payload_len = u32::from_be_bytes([
            bytes[payload_len_offset],
            bytes[payload_len_offset + 1],
            bytes[payload_len_offset + 2],
            bytes[payload_len_offset + 3],
        ]) as usize;
        if payload_len > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(EvidenceError::PayloadTooLarge);
        }
        let expected_len = payload_header_end
            .checked_add(payload_len)
            .ok_or(EvidenceError::PayloadTooLarge)?;
        if bytes.len() < expected_len {
            return Err(EvidenceError::TruncatedEncoding);
        }
        if bytes.len() > expected_len {
            return Err(EvidenceError::TrailingBytes);
        }

        let mut parent_ids = Vec::with_capacity(parent_count);
        let mut cursor = parent_count_offset + 2;
        for _ in 0..parent_count {
            let mut id = [0_u8; ID_LEN];
            id.copy_from_slice(&bytes[cursor..cursor + ID_LEN]);
            parent_ids.push(RecordId::from_bytes(id));
            cursor += ID_LEN;
        }

        Self::new(
            kind,
            issuer_id,
            subject_id,
            policy_id,
            parent_ids,
            bytes[payload_header_end..expected_len].to_vec(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceError {
    UnsupportedVersion(u16),
    UnknownRecordKind(u8),
    ZeroIdentity,
    ZeroParent,
    DuplicateParent,
    TooManyParents,
    EmptyPayload,
    PayloadTooLarge,
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EvidenceError {}

fn validate_parts(
    issuer_id: IdentityId,
    subject_id: IdentityId,
    policy_id: IdentityId,
    parent_ids: &[RecordId],
    payload: &[u8],
) -> Result<(), EvidenceError> {
    if issuer_id.is_zero() || subject_id.is_zero() || policy_id.is_zero() {
        return Err(EvidenceError::ZeroIdentity);
    }
    if parent_ids.len() > MAX_EVIDENCE_PARENTS {
        return Err(EvidenceError::TooManyParents);
    }
    let mut seen = HashSet::with_capacity(parent_ids.len());
    for parent in parent_ids {
        if parent.is_zero() {
            return Err(EvidenceError::ZeroParent);
        }
        if !seen.insert(*parent) {
            return Err(EvidenceError::DuplicateParent);
        }
    }
    if payload.is_empty() {
        return Err(EvidenceError::EmptyPayload);
    }
    if payload.len() > MAX_EVIDENCE_PAYLOAD_LEN {
        return Err(EvidenceError::PayloadTooLarge);
    }
    Ok(())
}

fn read_identity(bytes: &[u8], offset: usize) -> IdentityId {
    let mut id = [0_u8; ID_LEN];
    id.copy_from_slice(&bytes[offset..offset + ID_LEN]);
    IdentityId::from_bytes(id)
}

fn encode_parts(
    kind: RecordKind,
    issuer_id: IdentityId,
    subject_id: IdentityId,
    policy_id: IdentityId,
    parent_ids: &[RecordId],
    payload: &[u8],
) -> Vec<u8> {
    let mut encoded =
        Vec::with_capacity(FIXED_ENVELOPE_LEN + parent_ids.len() * ID_LEN + payload.len());
    encoded.extend_from_slice(&EVIDENCE_SCHEMA_VERSION.to_be_bytes());
    encoded.push(kind.as_u8());
    encoded.extend_from_slice(issuer_id.as_bytes());
    encoded.extend_from_slice(subject_id.as_bytes());
    encoded.extend_from_slice(policy_id.as_bytes());
    encoded.extend_from_slice(&(parent_ids.len() as u16).to_be_bytes());
    for parent in parent_ids {
        encoded.extend_from_slice(parent.as_bytes());
    }
    encoded.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    encoded.extend_from_slice(payload);
    encoded
}

fn derive_record_id(encoded: &[u8]) -> RecordId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(EVIDENCE_DOMAIN_SEPARATOR);
    hasher.update(encoded);
    RecordId::from_bytes(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(value: u8) -> IdentityId {
        IdentityId::from_bytes([value; ID_LEN])
    }

    fn record(kind: RecordKind) -> EvidenceRecord {
        EvidenceRecord::new(
            kind,
            identity(1),
            identity(2),
            identity(3),
            vec![RecordId::from_bytes([4; ID_LEN])],
            b"canonical payload".to_vec(),
        )
        .unwrap()
    }

    #[test]
    fn every_kind_round_trips_canonically() {
        for kind in [
            RecordKind::Objective,
            RecordKind::Claim,
            RecordKind::Source,
            RecordKind::Method,
            RecordKind::Uncertainty,
            RecordKind::FailedAttempt,
            RecordKind::ReviewerFinding,
            RecordKind::Dispute,
            RecordKind::Disposition,
        ] {
            let original = record(kind);
            let encoded = original.encode();
            let decoded = EvidenceRecord::decode(&encoded).unwrap();
            assert_eq!(decoded, original);
            assert_eq!(decoded.encode(), encoded);
        }
    }

    #[test]
    fn parent_order_changes_record_id() {
        let first = RecordId::from_bytes([4; ID_LEN]);
        let second = RecordId::from_bytes([5; ID_LEN]);
        let make = |parents| {
            EvidenceRecord::new(
                RecordKind::Claim,
                identity(1),
                identity(2),
                identity(3),
                parents,
                vec![1],
            )
            .unwrap()
        };
        assert_ne!(
            make(vec![first, second]).id(),
            make(vec![second, first]).id()
        );
    }

    #[test]
    fn duplicate_parent_is_rejected() {
        let parent = RecordId::from_bytes([4; ID_LEN]);
        assert_eq!(
            EvidenceRecord::new(
                RecordKind::Claim,
                identity(1),
                identity(2),
                identity(3),
                vec![parent, parent],
                vec![1]
            )
            .unwrap_err(),
            EvidenceError::DuplicateParent
        );
    }

    #[test]
    fn zero_identity_is_rejected() {
        assert_eq!(
            EvidenceRecord::new(
                RecordKind::Claim,
                identity(0),
                identity(2),
                identity(3),
                vec![],
                vec![1]
            )
            .unwrap_err(),
            EvidenceError::ZeroIdentity
        );
    }

    #[test]
    fn zero_parent_is_rejected() {
        assert_eq!(
            EvidenceRecord::new(
                RecordKind::Claim,
                identity(1),
                identity(2),
                identity(3),
                vec![RecordId::from_bytes([0; ID_LEN])],
                vec![1]
            )
            .unwrap_err(),
            EvidenceError::ZeroParent
        );
    }

    #[test]
    fn empty_and_oversize_payloads_are_rejected() {
        assert_eq!(
            EvidenceRecord::new(
                RecordKind::Claim,
                identity(1),
                identity(2),
                identity(3),
                vec![],
                vec![]
            )
            .unwrap_err(),
            EvidenceError::EmptyPayload
        );
        assert_eq!(
            EvidenceRecord::new(
                RecordKind::Claim,
                identity(1),
                identity(2),
                identity(3),
                vec![],
                vec![0; MAX_EVIDENCE_PAYLOAD_LEN + 1]
            )
            .unwrap_err(),
            EvidenceError::PayloadTooLarge
        );
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let mut encoded = record(RecordKind::Objective).encode();
        encoded[..2].copy_from_slice(&2_u16.to_be_bytes());
        assert_eq!(
            EvidenceRecord::decode(&encoded).unwrap_err(),
            EvidenceError::UnsupportedVersion(2)
        );
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let mut encoded = record(RecordKind::Objective).encode();
        encoded[2] = 0xff;
        assert_eq!(
            EvidenceRecord::decode(&encoded).unwrap_err(),
            EvidenceError::UnknownRecordKind(0xff)
        );
    }

    #[test]
    fn truncation_at_every_byte_is_rejected() {
        let encoded = record(RecordKind::Source).encode();
        for end in 0..encoded.len() {
            assert_eq!(
                EvidenceRecord::decode(&encoded[..end]).unwrap_err(),
                EvidenceError::TruncatedEncoding
            );
        }
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let mut encoded = record(RecordKind::Method).encode();
        encoded.push(0);
        assert_eq!(
            EvidenceRecord::decode(&encoded).unwrap_err(),
            EvidenceError::TrailingBytes
        );
    }

    #[test]
    fn declared_parent_overflow_is_rejected_before_allocation() {
        let mut encoded = record(RecordKind::Claim).encode();
        let offset = 3 + (3 * ID_LEN);
        encoded[offset..offset + 2].copy_from_slice(&u16::MAX.to_be_bytes());
        assert_eq!(
            EvidenceRecord::decode(&encoded).unwrap_err(),
            EvidenceError::TooManyParents
        );
    }

    #[test]
    fn declared_payload_overflow_is_rejected_before_allocation() {
        let mut encoded = record(RecordKind::Claim).encode();
        let offset = 3 + (3 * ID_LEN) + 2 + ID_LEN;
        encoded[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            EvidenceRecord::decode(&encoded).unwrap_err(),
            EvidenceError::PayloadTooLarge
        );
    }
}
