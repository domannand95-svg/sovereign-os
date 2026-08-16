use crate::{RecordId, MAX_EVIDENCE_PAYLOAD_LEN, MAX_LIST_ITEMS, MAX_TEXT_FIELD_LEN};
use sovereign_registry::IdentityId;
use std::collections::HashSet;
use std::fmt;

const ID_LEN: usize = 32;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DispositionDecision {
    AcceptForReview = 0x00,
    Reject = 0x01,
    Defer = 0x02,
    Quarantine = 0x03,
    Supersede = 0x04,
    Revoke = 0x05,
}

impl DispositionDecision {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self, DispositionError> {
        match value {
            0x00 => Ok(Self::AcceptForReview),
            0x01 => Ok(Self::Reject),
            0x02 => Ok(Self::Defer),
            0x03 => Ok(Self::Quarantine),
            0x04 => Ok(Self::Supersede),
            0x05 => Ok(Self::Revoke),
            unknown => Err(DispositionError::UnknownDecision(unknown)),
        }
    }

    const fn requires_supersedes_id(self) -> bool {
        matches!(self, Self::Supersede | Self::Revoke)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispositionPayload {
    decided_id: RecordId,
    decision: DispositionDecision,
    decision_authority_id: IdentityId,
    criteria_id: IdentityId,
    evidence_ids: Vec<RecordId>,
    unresolved_dispute_ids: Vec<RecordId>,
    reason: String,
    supersedes_id: Option<RecordId>,
}

impl DispositionPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        decided_id: RecordId,
        decision: DispositionDecision,
        decision_authority_id: IdentityId,
        criteria_id: IdentityId,
        evidence_ids: Vec<RecordId>,
        unresolved_dispute_ids: Vec<RecordId>,
        reason: String,
        supersedes_id: Option<RecordId>,
    ) -> Result<Self, DispositionError> {
        validate_record_id(decided_id)?;

        if decision_authority_id.is_zero() || criteria_id.is_zero() {
            return Err(DispositionError::ZeroIdentity);
        }

        validate_record_ids(&evidence_ids)?;
        validate_record_ids(&unresolved_dispute_ids)?;
        validate_reason(&reason)?;

        if let Some(id) = supersedes_id {
            validate_record_id(id)?;
        }

        match (decision.requires_supersedes_id(), supersedes_id) {
            (true, None) => return Err(DispositionError::SupersedesRequired),
            (false, Some(_)) => return Err(DispositionError::SupersedesForbidden),
            _ => {}
        }

        let payload = Self {
            decided_id,
            decision,
            decision_authority_id,
            criteria_id,
            evidence_ids,
            unresolved_dispute_ids,
            reason,
            supersedes_id,
        };

        if payload.encoded_len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(DispositionError::PayloadTooLarge);
        }

        Ok(payload)
    }

    pub const fn decided_id(&self) -> RecordId {
        self.decided_id
    }

    pub const fn decision(&self) -> DispositionDecision {
        self.decision
    }

    pub const fn decision_authority_id(&self) -> IdentityId {
        self.decision_authority_id
    }

    pub const fn criteria_id(&self) -> IdentityId {
        self.criteria_id
    }

    pub fn evidence_ids(&self) -> &[RecordId] {
        &self.evidence_ids
    }

    pub fn unresolved_dispute_ids(&self) -> &[RecordId] {
        &self.unresolved_dispute_ids
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub const fn supersedes_id(&self) -> Option<RecordId> {
        self.supersedes_id
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(self.encoded_len());

        encoded.extend_from_slice(self.decided_id.as_bytes());
        encoded.push(self.decision.as_u8());
        encoded.extend_from_slice(self.decision_authority_id.as_bytes());
        encoded.extend_from_slice(self.criteria_id.as_bytes());

        encode_record_ids(&mut encoded, &self.evidence_ids);
        encode_record_ids(&mut encoded, &self.unresolved_dispute_ids);

        encoded.extend_from_slice(&(self.reason.len() as u32).to_be_bytes());
        encoded.extend_from_slice(self.reason.as_bytes());

        match self.supersedes_id {
            None => encoded.push(0),
            Some(id) => {
                encoded.push(1);
                encoded.extend_from_slice(id.as_bytes());
            }
        }

        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DispositionError> {
        if bytes.len() > MAX_EVIDENCE_PAYLOAD_LEN {
            return Err(DispositionError::PayloadTooLarge);
        }

        let mut decoder = Decoder::new(bytes);

        let decided_id = decoder.read_record_id()?;
        let decision = DispositionDecision::from_u8(decoder.read_u8()?)?;
        let decision_authority_id = decoder.read_identity_id()?;
        let criteria_id = decoder.read_identity_id()?;
        let evidence_ids = decoder.read_record_ids()?;
        let unresolved_dispute_ids = decoder.read_record_ids()?;
        let reason = decoder.read_text()?;

        let supersedes_id = match decoder.read_u8()? {
            0 => None,
            1 => Some(decoder.read_record_id()?),
            marker => return Err(DispositionError::InvalidOptionalMarker(marker)),
        };

        if !decoder.is_finished() {
            return Err(DispositionError::TrailingBytes);
        }

        Self::new(
            decided_id,
            decision,
            decision_authority_id,
            criteria_id,
            evidence_ids,
            unresolved_dispute_ids,
            reason,
            supersedes_id,
        )
    }

    fn encoded_len(&self) -> usize {
        ID_LEN
            + 1
            + ID_LEN
            + ID_LEN
            + 2
            + self.evidence_ids.len() * ID_LEN
            + 2
            + self.unresolved_dispute_ids.len() * ID_LEN
            + 4
            + self.reason.len()
            + 1
            + self.supersedes_id.map_or(0, |_| ID_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispositionError {
    ZeroRecordId,
    ZeroIdentity,
    DuplicateRecordId,
    TooManyItems,
    EmptyReason,
    ReasonContainsNul,
    ReasonTooLong,
    InvalidUtf8,
    UnknownDecision(u8),
    InvalidOptionalMarker(u8),
    SupersedesRequired,
    SupersedesForbidden,
    PayloadTooLarge,
    TruncatedEncoding,
    TrailingBytes,
}

impl fmt::Display for DispositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DispositionError {}

fn validate_record_id(id: RecordId) -> Result<(), DispositionError> {
    if id.is_zero() {
        return Err(DispositionError::ZeroRecordId);
    }

    Ok(())
}

fn validate_record_ids(ids: &[RecordId]) -> Result<(), DispositionError> {
    if ids.len() > MAX_LIST_ITEMS {
        return Err(DispositionError::TooManyItems);
    }

    let mut seen = HashSet::with_capacity(ids.len());

    for id in ids {
        validate_record_id(*id)?;

        if !seen.insert(*id) {
            return Err(DispositionError::DuplicateRecordId);
        }
    }

    Ok(())
}

fn validate_reason(reason: &str) -> Result<(), DispositionError> {
    if reason.is_empty() {
        return Err(DispositionError::EmptyReason);
    }

    if reason.as_bytes().contains(&0) {
        return Err(DispositionError::ReasonContainsNul);
    }

    if reason.len() > MAX_TEXT_FIELD_LEN {
        return Err(DispositionError::ReasonTooLong);
    }

    Ok(())
}

fn encode_record_ids(encoded: &mut Vec<u8>, ids: &[RecordId]) {
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

    fn read_u8(&mut self) -> Result<u8, DispositionError> {
        let value = *self
            .bytes
            .get(self.cursor)
            .ok_or(DispositionError::TruncatedEncoding)?;

        self.cursor += 1;
        Ok(value)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], DispositionError> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(DispositionError::TruncatedEncoding)?;

        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(DispositionError::TruncatedEncoding)?;

        let mut value = [0_u8; N];
        value.copy_from_slice(bytes);
        self.cursor = end;

        Ok(value)
    }

    fn read_record_id(&mut self) -> Result<RecordId, DispositionError> {
        Ok(RecordId::from_bytes(self.read_array()?))
    }

    fn read_identity_id(&mut self) -> Result<IdentityId, DispositionError> {
        Ok(IdentityId::from_bytes(self.read_array()?))
    }

    fn read_record_ids(&mut self) -> Result<Vec<RecordId>, DispositionError> {
        let count = u16::from_be_bytes(self.read_array()?) as usize;

        if count > MAX_LIST_ITEMS {
            return Err(DispositionError::TooManyItems);
        }

        let required = count
            .checked_mul(ID_LEN)
            .ok_or(DispositionError::TruncatedEncoding)?;

        let end = self
            .cursor
            .checked_add(required)
            .ok_or(DispositionError::TruncatedEncoding)?;

        if end > self.bytes.len() {
            return Err(DispositionError::TruncatedEncoding);
        }

        let mut ids = Vec::with_capacity(count);

        for _ in 0..count {
            ids.push(self.read_record_id()?);
        }

        Ok(ids)
    }

    fn read_text(&mut self) -> Result<String, DispositionError> {
        let length = u32::from_be_bytes(self.read_array()?) as usize;

        if length > MAX_TEXT_FIELD_LEN {
            return Err(DispositionError::ReasonTooLong);
        }

        let end = self
            .cursor
            .checked_add(length)
            .ok_or(DispositionError::TruncatedEncoding)?;

        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(DispositionError::TruncatedEncoding)?;

        let text = std::str::from_utf8(bytes)
            .map_err(|_| DispositionError::InvalidUtf8)?
            .to_owned();

        self.cursor = end;
        Ok(text)
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

    fn payload(decision: DispositionDecision) -> DispositionPayload {
        let supersedes_id = if decision.requires_supersedes_id() {
            Some(record_id(8))
        } else {
            None
        };

        DispositionPayload::new(
            record_id(1),
            decision,
            identity(2),
            identity(3),
            vec![record_id(4), record_id(5)],
            vec![record_id(6), record_id(7)],
            "reason".to_owned(),
            supersedes_id,
        )
        .unwrap()
    }

    #[test]
    fn decision_discriminators_are_frozen() {
        assert_eq!(DispositionDecision::AcceptForReview.as_u8(), 0x00);
        assert_eq!(DispositionDecision::Reject.as_u8(), 0x01);
        assert_eq!(DispositionDecision::Defer.as_u8(), 0x02);
        assert_eq!(DispositionDecision::Quarantine.as_u8(), 0x03);
        assert_eq!(DispositionDecision::Supersede.as_u8(), 0x04);
        assert_eq!(DispositionDecision::Revoke.as_u8(), 0x05);
    }

    #[test]
    fn round_trip_preserves_every_field_and_decision() {
        for decision in [
            DispositionDecision::AcceptForReview,
            DispositionDecision::Reject,
            DispositionDecision::Defer,
            DispositionDecision::Quarantine,
            DispositionDecision::Supersede,
            DispositionDecision::Revoke,
        ] {
            let original = payload(decision);
            let encoded = original.encode();
            let decoded = DispositionPayload::decode(&encoded).unwrap();

            assert_eq!(decoded, original);
            assert_eq!(decoded.encode(), encoded);
        }
    }

    #[test]
    fn minimal_fixed_vector_is_stable_and_big_endian() {
        let value = DispositionPayload::new(
            record_id(1),
            DispositionDecision::AcceptForReview,
            identity(2),
            identity(3),
            vec![],
            vec![],
            "R".to_owned(),
            None,
        )
        .unwrap();

        let mut expected = vec![1; ID_LEN];
        expected.push(0x00);
        expected.extend_from_slice(&[2; ID_LEN]);
        expected.extend_from_slice(&[3; ID_LEN]);
        expected.extend_from_slice(&0_u16.to_be_bytes());
        expected.extend_from_slice(&0_u16.to_be_bytes());
        expected.extend_from_slice(&1_u32.to_be_bytes());
        expected.push(b'R');
        expected.push(0);

        assert_eq!(expected.len(), 107);
        assert_eq!(value.encode(), expected);
        assert_eq!(DispositionPayload::decode(&expected).unwrap(), value);
    }

    #[test]
    fn required_identifiers_fail_closed() {
        assert_eq!(
            DispositionPayload::new(
                RecordId::from_bytes([0; ID_LEN]),
                DispositionDecision::Reject,
                identity(2),
                identity(3),
                vec![],
                vec![],
                "reason".to_owned(),
                None,
            )
            .unwrap_err(),
            DispositionError::ZeroRecordId
        );

        assert_eq!(
            DispositionPayload::new(
                record_id(1),
                DispositionDecision::Reject,
                IdentityId::from_bytes([0; ID_LEN]),
                identity(3),
                vec![],
                vec![],
                "reason".to_owned(),
                None,
            )
            .unwrap_err(),
            DispositionError::ZeroIdentity
        );

        assert_eq!(
            DispositionPayload::new(
                record_id(1),
                DispositionDecision::Reject,
                identity(2),
                IdentityId::from_bytes([0; ID_LEN]),
                vec![],
                vec![],
                "reason".to_owned(),
                None,
            )
            .unwrap_err(),
            DispositionError::ZeroIdentity
        );
    }

    #[test]
    fn evidence_and_dispute_lists_are_unique_nonzero_and_bounded() {
        for (evidence_ids, unresolved_dispute_ids) in [
            (vec![RecordId::from_bytes([0; ID_LEN])], vec![]),
            (vec![], vec![RecordId::from_bytes([0; ID_LEN])]),
        ] {
            assert_eq!(
                DispositionPayload::new(
                    record_id(1),
                    DispositionDecision::Reject,
                    identity(2),
                    identity(3),
                    evidence_ids,
                    unresolved_dispute_ids,
                    "reason".to_owned(),
                    None,
                )
                .unwrap_err(),
                DispositionError::ZeroRecordId
            );
        }

        for (evidence_ids, unresolved_dispute_ids) in [
            (vec![record_id(4), record_id(4)], vec![]),
            (vec![], vec![record_id(5), record_id(5)]),
        ] {
            assert_eq!(
                DispositionPayload::new(
                    record_id(1),
                    DispositionDecision::Reject,
                    identity(2),
                    identity(3),
                    evidence_ids,
                    unresolved_dispute_ids,
                    "reason".to_owned(),
                    None,
                )
                .unwrap_err(),
                DispositionError::DuplicateRecordId
            );
        }

        let max_ids: Vec<_> = (1..=MAX_LIST_ITEMS)
            .map(|value| record_id(value as u8))
            .collect();

        assert!(DispositionPayload::new(
            record_id(100),
            DispositionDecision::Reject,
            identity(101),
            identity(102),
            max_ids.clone(),
            max_ids,
            "reason".to_owned(),
            None,
        )
        .is_ok());

        let too_many: Vec<_> = (1..=(MAX_LIST_ITEMS + 1))
            .map(|value| record_id(value as u8))
            .collect();

        assert_eq!(
            DispositionPayload::new(
                record_id(100),
                DispositionDecision::Reject,
                identity(101),
                identity(102),
                too_many,
                vec![],
                "reason".to_owned(),
                None,
            )
            .unwrap_err(),
            DispositionError::TooManyItems
        );
    }

    #[test]
    fn declared_list_order_is_preserved() {
        let first = DispositionPayload::new(
            record_id(1),
            DispositionDecision::Defer,
            identity(2),
            identity(3),
            vec![record_id(4), record_id(5)],
            vec![record_id(6), record_id(7)],
            "reason".to_owned(),
            None,
        )
        .unwrap();

        let reordered = DispositionPayload::new(
            record_id(1),
            DispositionDecision::Defer,
            identity(2),
            identity(3),
            vec![record_id(5), record_id(4)],
            vec![record_id(7), record_id(6)],
            "reason".to_owned(),
            None,
        )
        .unwrap();

        assert_ne!(first.encode(), reordered.encode());
        assert_eq!(first.evidence_ids(), &[record_id(4), record_id(5)]);
        assert_eq!(
            first.unresolved_dispute_ids(),
            &[record_id(6), record_id(7)]
        );
    }

    #[test]
    fn reason_constraints_fail_closed() {
        assert_eq!(
            DispositionPayload::new(
                record_id(1),
                DispositionDecision::Reject,
                identity(2),
                identity(3),
                vec![],
                vec![],
                String::new(),
                None,
            )
            .unwrap_err(),
            DispositionError::EmptyReason
        );

        assert_eq!(
            DispositionPayload::new(
                record_id(1),
                DispositionDecision::Reject,
                identity(2),
                identity(3),
                vec![],
                vec![],
                "bad\0reason".to_owned(),
                None,
            )
            .unwrap_err(),
            DispositionError::ReasonContainsNul
        );

        assert_eq!(
            DispositionPayload::new(
                record_id(1),
                DispositionDecision::Reject,
                identity(2),
                identity(3),
                vec![],
                vec![],
                "x".repeat(MAX_TEXT_FIELD_LEN + 1),
                None,
            )
            .unwrap_err(),
            DispositionError::ReasonTooLong
        );

        assert!(DispositionPayload::new(
            record_id(1),
            DispositionDecision::Reject,
            identity(2),
            identity(3),
            vec![],
            vec![],
            "x".repeat(MAX_TEXT_FIELD_LEN),
            None,
        )
        .is_ok());
    }

    #[test]
    fn supersedes_coherence_fails_closed() {
        for decision in [DispositionDecision::Supersede, DispositionDecision::Revoke] {
            assert_eq!(
                DispositionPayload::new(
                    record_id(1),
                    decision,
                    identity(2),
                    identity(3),
                    vec![],
                    vec![],
                    "reason".to_owned(),
                    None,
                )
                .unwrap_err(),
                DispositionError::SupersedesRequired
            );

            assert_eq!(
                DispositionPayload::new(
                    record_id(1),
                    decision,
                    identity(2),
                    identity(3),
                    vec![],
                    vec![],
                    "reason".to_owned(),
                    Some(RecordId::from_bytes([0; ID_LEN])),
                )
                .unwrap_err(),
                DispositionError::ZeroRecordId
            );
        }

        for decision in [
            DispositionDecision::AcceptForReview,
            DispositionDecision::Reject,
            DispositionDecision::Defer,
            DispositionDecision::Quarantine,
        ] {
            assert_eq!(
                DispositionPayload::new(
                    record_id(1),
                    decision,
                    identity(2),
                    identity(3),
                    vec![],
                    vec![],
                    "reason".to_owned(),
                    Some(record_id(8)),
                )
                .unwrap_err(),
                DispositionError::SupersedesForbidden
            );
        }
    }

    #[test]
    fn malformed_decision_and_optional_marker_fail_closed() {
        let minimal = DispositionPayload::new(
            record_id(1),
            DispositionDecision::AcceptForReview,
            identity(2),
            identity(3),
            vec![],
            vec![],
            "R".to_owned(),
            None,
        )
        .unwrap();

        let mut bad_decision = minimal.encode();
        bad_decision[ID_LEN] = 0xff;
        assert_eq!(
            DispositionPayload::decode(&bad_decision).unwrap_err(),
            DispositionError::UnknownDecision(0xff)
        );

        let mut bad_marker = minimal.encode();
        let marker_offset = bad_marker.len() - 1;
        bad_marker[marker_offset] = 0xff;
        assert_eq!(
            DispositionPayload::decode(&bad_marker).unwrap_err(),
            DispositionError::InvalidOptionalMarker(0xff)
        );
    }

    #[test]
    fn malformed_counts_are_rejected_before_unbounded_allocation() {
        let minimal = DispositionPayload::new(
            record_id(1),
            DispositionDecision::Reject,
            identity(2),
            identity(3),
            vec![],
            vec![],
            "R".to_owned(),
            None,
        )
        .unwrap();

        let mut excessive_evidence = minimal.encode();
        let evidence_count_offset = ID_LEN + 1 + ID_LEN + ID_LEN;
        excessive_evidence[evidence_count_offset..evidence_count_offset + 2]
            .copy_from_slice(&((MAX_LIST_ITEMS + 1) as u16).to_be_bytes());

        assert_eq!(
            DispositionPayload::decode(&excessive_evidence).unwrap_err(),
            DispositionError::TooManyItems
        );
    }

    #[test]
    fn invalid_utf8_fails_closed() {
        let value = DispositionPayload::new(
            record_id(1),
            DispositionDecision::Reject,
            identity(2),
            identity(3),
            vec![],
            vec![],
            "R".to_owned(),
            None,
        )
        .unwrap();

        let mut encoded = value.encode();

        let reason_offset = ID_LEN + 1 + ID_LEN + ID_LEN + 2 + 2 + 4;

        encoded[reason_offset] = 0xff;

        assert_eq!(
            DispositionPayload::decode(&encoded).unwrap_err(),
            DispositionError::InvalidUtf8
        );
    }

    #[test]
    fn every_truncation_trailing_and_oversize_fails_closed() {
        let encoded = payload(DispositionDecision::Supersede).encode();

        for length in 0..encoded.len() {
            assert!(
                DispositionPayload::decode(&encoded[..length]).is_err(),
                "truncation unexpectedly accepted at {length}"
            );
        }

        let mut trailing = encoded.clone();
        trailing.push(0);
        assert_eq!(
            DispositionPayload::decode(&trailing).unwrap_err(),
            DispositionError::TrailingBytes
        );

        let oversized = vec![0; MAX_EVIDENCE_PAYLOAD_LEN + 1];
        assert_eq!(
            DispositionPayload::decode(&oversized).unwrap_err(),
            DispositionError::PayloadTooLarge
        );
    }

    #[test]
    fn every_committed_payload_field_changes_encoding() {
        let baseline = payload(DispositionDecision::Supersede);

        let variations = [
            DispositionPayload::new(
                record_id(9),
                baseline.decision(),
                baseline.decision_authority_id(),
                baseline.criteria_id(),
                baseline.evidence_ids().to_vec(),
                baseline.unresolved_dispute_ids().to_vec(),
                baseline.reason().to_owned(),
                baseline.supersedes_id(),
            )
            .unwrap(),
            DispositionPayload::new(
                baseline.decided_id(),
                DispositionDecision::Revoke,
                baseline.decision_authority_id(),
                baseline.criteria_id(),
                baseline.evidence_ids().to_vec(),
                baseline.unresolved_dispute_ids().to_vec(),
                baseline.reason().to_owned(),
                baseline.supersedes_id(),
            )
            .unwrap(),
            DispositionPayload::new(
                baseline.decided_id(),
                baseline.decision(),
                identity(9),
                baseline.criteria_id(),
                baseline.evidence_ids().to_vec(),
                baseline.unresolved_dispute_ids().to_vec(),
                baseline.reason().to_owned(),
                baseline.supersedes_id(),
            )
            .unwrap(),
            DispositionPayload::new(
                baseline.decided_id(),
                baseline.decision(),
                baseline.decision_authority_id(),
                identity(9),
                baseline.evidence_ids().to_vec(),
                baseline.unresolved_dispute_ids().to_vec(),
                baseline.reason().to_owned(),
                baseline.supersedes_id(),
            )
            .unwrap(),
            DispositionPayload::new(
                baseline.decided_id(),
                baseline.decision(),
                baseline.decision_authority_id(),
                baseline.criteria_id(),
                vec![record_id(5), record_id(4)],
                baseline.unresolved_dispute_ids().to_vec(),
                baseline.reason().to_owned(),
                baseline.supersedes_id(),
            )
            .unwrap(),
            DispositionPayload::new(
                baseline.decided_id(),
                baseline.decision(),
                baseline.decision_authority_id(),
                baseline.criteria_id(),
                baseline.evidence_ids().to_vec(),
                vec![record_id(7), record_id(6)],
                baseline.reason().to_owned(),
                baseline.supersedes_id(),
            )
            .unwrap(),
            DispositionPayload::new(
                baseline.decided_id(),
                baseline.decision(),
                baseline.decision_authority_id(),
                baseline.criteria_id(),
                baseline.evidence_ids().to_vec(),
                baseline.unresolved_dispute_ids().to_vec(),
                "different reason".to_owned(),
                baseline.supersedes_id(),
            )
            .unwrap(),
            DispositionPayload::new(
                baseline.decided_id(),
                baseline.decision(),
                baseline.decision_authority_id(),
                baseline.criteria_id(),
                baseline.evidence_ids().to_vec(),
                baseline.unresolved_dispute_ids().to_vec(),
                baseline.reason().to_owned(),
                Some(record_id(9)),
            )
            .unwrap(),
        ];

        let baseline_bytes = baseline.encode();

        for variation in variations {
            assert_ne!(variation.encode(), baseline_bytes);
        }
    }

    #[test]
    fn governed_disposition_fixture_id_is_frozen() {
        let payload = DispositionPayload::new(
            record_id(1),
            DispositionDecision::AcceptForReview,
            identity(2),
            identity(3),
            vec![],
            vec![],
            "R".to_owned(),
            None,
        )
        .unwrap();

        let record = EvidenceRecord::new_disposition(
            identity(9),
            identity(10),
            identity(11),
            vec![record_id(12)],
            payload,
        )
        .unwrap();

        assert_eq!(
            record.id().as_bytes(),
            &[
                0xf0, 0x51, 0xf3, 0x9b, 0xc5, 0xc3, 0x30, 0x12, 0x23, 0x22, 0x7f, 0x60, 0xe3, 0x2e,
                0x00, 0x0f, 0xbe, 0xc0, 0x38, 0xf9, 0x84, 0x52, 0xa9, 0xf4, 0xa2, 0x09, 0xcb, 0x9e,
                0xc1, 0xda, 0xec, 0x67,
            ]
        );
    }

    #[test]
    fn governed_record_identity_changes_with_payload() {
        fn governed(payload: DispositionPayload) -> EvidenceRecord {
            EvidenceRecord::new(
                RecordKind::Disposition,
                identity(20),
                identity(21),
                identity(22),
                vec![record_id(23)],
                payload.encode(),
            )
            .unwrap()
        }

        let baseline = governed(payload(DispositionDecision::Supersede));
        let changed = governed(
            DispositionPayload::new(
                record_id(1),
                DispositionDecision::Revoke,
                identity(2),
                identity(3),
                vec![record_id(4), record_id(5)],
                vec![record_id(6), record_id(7)],
                "reason".to_owned(),
                Some(record_id(8)),
            )
            .unwrap(),
        );

        assert_ne!(baseline.id(), changed.id());
    }
}
