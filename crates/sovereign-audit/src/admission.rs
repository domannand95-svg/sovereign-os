use crate::{EvidencePayloadError, EvidenceRecord, RecordId, RecordKind};
use std::fmt;

/// Storage-neutral reference to one authoritative A04 admission state.
///
/// Implementations must bind every lookup performed during one admission
/// evaluation to the same explicitly supplied state reference. The evaluator
/// does not select ambient, latest, default, inherited, or wall-clock state.
pub trait EvidenceAdmissionStateRef: Eq {}

/// Fail-closed errors exposed by an authoritative A04 admission adapter.
///
/// Concrete storage, registry, identity, or policy failures must be mapped into
/// these storage-neutral outcomes before crossing the admission boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceAdmissionAuthorityError {
    StateUnavailable,
    RequiredRelationalInformationUnavailable,
}

/// Authoritative independence result.
///
/// This is deliberately distinct from the descriptive
/// `ReviewerFindingPayload::independence_result` field. Payload assertions do
/// not establish reviewer independence at admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoritativeIndependence {
    Established,
    NotEstablished,
    Conflicted,
    Unknown,
}

/// Storage-neutral authority boundary used by A04 cross-record admission.
///
/// The first implementation increment allocates only exact governed-record
/// resolution. Later owner-approved increments may extend the authority surface
/// for independence, criteria, dispute, cycle, and canonical-identity facts.
///
/// This interface grants no capability, execution, publication, promotion, or
/// mutation authority.
pub trait EvidenceAdmissionAuthority {
    type StateRef: EvidenceAdmissionStateRef;

    /// Confirm that the supplied state reference identifies available
    /// authoritative state before relational evaluation begins.
    fn validate_state_ref(
        &self,
        state_ref: &Self::StateRef,
    ) -> Result<(), EvidenceAdmissionAuthorityError>;

    /// Resolve one already-admitted A04 record by its exact `RecordId`.
    fn resolve_record(
        &self,
        record_id: &RecordId,
        state_ref: &Self::StateRef,
    ) -> Result<Option<EvidenceRecord>, EvidenceAdmissionAuthorityError>;
}

/// Stable A04 cross-record admission failures.
///
/// Several variants are frozen here before their corresponding relational rules
/// are implemented. Unknown or unavailable authoritative information must never
/// map to admission success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceAdmissionError {
    AbsentReferencedRecord,
    WrongReferencedRecordKind {
        expected: RecordKind,
        actual: RecordKind,
    },
    ResolvedRecordMismatch {
        requested: RecordId,
        actual: RecordId,
    },
    MalformedCandidatePayload,
    SelfReview,
    SelfDisposition,
    IndependenceNotEstablished,
    AuthoritativeIndependenceUnavailable,
    InvalidResolvedDisputeRelationship,
    RequiredEvidenceOmitted,
    RelationshipCycle,
    PrematureA05Reference,
    NonCanonicalExternalIdentity,
    RetroactiveMutationAttempt,
    AuthoritativeStateUnavailable,
    RequiredRelationalInformationUnavailable,
}

impl fmt::Display for EvidenceAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EvidenceAdmissionError {}

impl From<EvidenceAdmissionAuthorityError> for EvidenceAdmissionError {
    fn from(error: EvidenceAdmissionAuthorityError) -> Self {
        match error {
            EvidenceAdmissionAuthorityError::StateUnavailable => {
                Self::AuthoritativeStateUnavailable
            }
            EvidenceAdmissionAuthorityError::RequiredRelationalInformationUnavailable => {
                Self::RequiredRelationalInformationUnavailable
            }
        }
    }
}

/// Successful result of pure A04 cross-record evaluation.
///
/// Admissibility is epistemic only. It does not grant capability, execution,
/// publication, promotion, or repository authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceAdmissionResult {
    Admissible,
}

/// Evaluate the currently allocated A04 cross-record admission rules.
///
/// This function is read-only. It receives the immutable candidate, an
/// explicitly supplied authority, and an explicitly supplied authoritative
/// state reference. It has no interface through which authoritative state can
/// be mutated.
///
/// The first increment evaluates:
///
/// - authoritative-state availability;
/// - exact resolution of every envelope parent;
/// - exact resolution of Reviewer Finding reviewed/evidence records;
/// - self-review rejection;
/// - exact resolution of Disposition decided/evidence/dispute/supersession
///   references; and
/// - self-disposition rejection.
///
/// Remaining SPEC-EV-001 Section 15 rules are implemented only in later
/// owner-approved increments.
pub fn evaluate_admission<A: EvidenceAdmissionAuthority>(
    candidate: &EvidenceRecord,
    authority: &A,
    state_ref: &A::StateRef,
) -> Result<EvidenceAdmissionResult, EvidenceAdmissionError> {
    authority.validate_state_ref(state_ref)?;

    for parent_id in candidate.parent_ids() {
        resolve_exact(authority, state_ref, parent_id)?;
    }

    match candidate.kind() {
        RecordKind::ReviewerFinding => {
            evaluate_reviewer_finding(candidate, authority, state_ref)?;
        }
        RecordKind::Disposition => {
            evaluate_disposition(candidate, authority, state_ref)?;
        }
        _ => {}
    }

    Ok(EvidenceAdmissionResult::Admissible)
}

fn evaluate_reviewer_finding<A: EvidenceAdmissionAuthority>(
    candidate: &EvidenceRecord,
    authority: &A,
    state_ref: &A::StateRef,
) -> Result<(), EvidenceAdmissionError> {
    let payload = candidate
        .decode_reviewer_finding_payload()
        .map_err(map_payload_error)?;

    let reviewed = resolve_exact(authority, state_ref, &payload.reviewed_id())?;

    for evidence_id in payload.evidence_ids() {
        resolve_exact(authority, state_ref, evidence_id)?;
    }

    if payload.reviewer_id() == reviewed.subject_id() {
        return Err(EvidenceAdmissionError::SelfReview);
    }

    Ok(())
}

fn evaluate_disposition<A: EvidenceAdmissionAuthority>(
    candidate: &EvidenceRecord,
    authority: &A,
    state_ref: &A::StateRef,
) -> Result<(), EvidenceAdmissionError> {
    let payload = candidate
        .decode_disposition_payload()
        .map_err(map_payload_error)?;

    let decided = resolve_exact(authority, state_ref, &payload.decided_id())?;

    for evidence_id in payload.evidence_ids() {
        resolve_exact(authority, state_ref, evidence_id)?;
    }

    for dispute_id in payload.unresolved_dispute_ids() {
        resolve_exact(authority, state_ref, dispute_id)?;
    }

    if let Some(supersedes_id) = payload.supersedes_id() {
        resolve_exact(authority, state_ref, &supersedes_id)?;
    }

    if payload.decision_authority_id() == decided.subject_id() {
        return Err(EvidenceAdmissionError::SelfDisposition);
    }

    Ok(())
}

fn resolve_exact<A: EvidenceAdmissionAuthority>(
    authority: &A,
    state_ref: &A::StateRef,
    requested_id: &RecordId,
) -> Result<EvidenceRecord, EvidenceAdmissionError> {
    let resolved = authority
        .resolve_record(requested_id, state_ref)?
        .ok_or(EvidenceAdmissionError::AbsentReferencedRecord)?;

    if resolved.id() != *requested_id {
        return Err(EvidenceAdmissionError::ResolvedRecordMismatch {
            requested: *requested_id,
            actual: resolved.id(),
        });
    }

    Ok(resolved)
}

fn map_payload_error(_: EvidencePayloadError) -> EvidenceAdmissionError {
    EvidenceAdmissionError::MalformedCandidatePayload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DispositionDecision, DispositionPayload, FindingKind, FindingSeverity, IndependenceResult,
        ReviewerFindingPayload,
    };
    use sovereign_registry::{IdentityId, IdentityKind, IdentityRecord};
    use std::collections::BTreeMap;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestStateRef(u8);

    impl EvidenceAdmissionStateRef for TestStateRef {}

    struct TestAuthority {
        expected_state: TestStateRef,
        records: BTreeMap<RecordId, EvidenceRecord>,
        substitute: Option<EvidenceRecord>,
    }

    impl TestAuthority {
        fn new(expected_state: TestStateRef) -> Self {
            Self {
                expected_state,
                records: BTreeMap::new(),
                substitute: None,
            }
        }

        fn insert(&mut self, record: EvidenceRecord) {
            self.records.insert(record.id(), record);
        }

        fn substitute_with(&mut self, record: EvidenceRecord) {
            self.substitute = Some(record);
        }
    }

    impl EvidenceAdmissionAuthority for TestAuthority {
        type StateRef = TestStateRef;

        fn validate_state_ref(
            &self,
            state_ref: &Self::StateRef,
        ) -> Result<(), EvidenceAdmissionAuthorityError> {
            if state_ref == &self.expected_state {
                Ok(())
            } else {
                Err(EvidenceAdmissionAuthorityError::StateUnavailable)
            }
        }

        fn resolve_record(
            &self,
            record_id: &RecordId,
            state_ref: &Self::StateRef,
        ) -> Result<Option<EvidenceRecord>, EvidenceAdmissionAuthorityError> {
            self.validate_state_ref(state_ref)?;

            if let Some(substitute) = &self.substitute {
                return Ok(Some(substitute.clone()));
            }

            Ok(self.records.get(record_id).cloned())
        }
    }

    fn identity(seed: u8) -> IdentityId {
        IdentityRecord::new(IdentityKind::Agent, vec![seed])
            .expect("test identity must be valid")
            .id()
    }

    fn generic_record(
        kind: RecordKind,
        seed: u8,
        subject_id: IdentityId,
        parent_ids: Vec<RecordId>,
    ) -> EvidenceRecord {
        EvidenceRecord::new(
            kind,
            identity(seed),
            subject_id,
            identity(seed.wrapping_add(1)),
            parent_ids,
            vec![seed],
        )
        .expect("test record must be valid")
    }

    #[test]
    fn resolved_parent_allows_admission() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let parent = generic_record(RecordKind::Objective, 10, identity(40), vec![]);
        authority.insert(parent.clone());

        let candidate = generic_record(RecordKind::Claim, 11, identity(41), vec![parent.id()]);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn absent_parent_fails_closed() {
        let state = TestStateRef(1);
        let authority = TestAuthority::new(state.clone());

        let missing = RecordId::from_bytes([0xAA; 32]);
        let candidate = generic_record(RecordKind::Claim, 12, identity(42), vec![missing]);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::AbsentReferencedRecord)
        );
    }

    #[test]
    fn unavailable_authoritative_state_fails_closed() {
        let authority = TestAuthority::new(TestStateRef(1));
        let supplied_state = TestStateRef(2);

        let candidate = generic_record(RecordKind::Objective, 13, identity(43), vec![]);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &supplied_state),
            Err(EvidenceAdmissionError::AuthoritativeStateUnavailable)
        );
    }

    #[test]
    fn resolver_must_return_the_exact_requested_record() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let expected = generic_record(RecordKind::Objective, 14, identity(44), vec![]);
        let substitute = generic_record(RecordKind::Objective, 15, identity(45), vec![]);

        authority.insert(expected.clone());
        authority.substitute_with(substitute.clone());

        let candidate = generic_record(RecordKind::Claim, 16, identity(46), vec![expected.id()]);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::ResolvedRecordMismatch {
                requested: expected.id(),
                actual: substitute.id(),
            })
        );
    }

    #[test]
    fn reviewer_cannot_review_its_own_subject() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let reviewer = identity(50);
        let reviewed = generic_record(RecordKind::Claim, 17, reviewer, vec![]);
        authority.insert(reviewed.clone());

        let payload = ReviewerFindingPayload::new(
            reviewed.id(),
            reviewer,
            FindingKind::Support,
            FindingSeverity::Informational,
            "review".to_owned(),
            vec![],
            "none".to_owned(),
            IndependenceResult::Established,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_reviewer_finding(
            reviewer,
            reviewed.subject_id(),
            identity(51),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::SelfReview)
        );
    }

    #[test]
    fn disposition_authority_cannot_dispose_its_own_subject() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decision_authority = identity(60);
        let decided = generic_record(RecordKind::Claim, 18, decision_authority, vec![]);
        authority.insert(decided.clone());

        let payload = DispositionPayload::new(
            decided.id(),
            DispositionDecision::AcceptForReview,
            decision_authority,
            identity(61),
            vec![],
            vec![],
            "decision".to_owned(),
            None,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_disposition(
            decision_authority,
            decided.subject_id(),
            identity(62),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::SelfDisposition)
        );
    }

    #[test]
    fn reviewer_evidence_references_must_exist() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let reviewer = identity(70);
        let reviewed = generic_record(RecordKind::Claim, 19, identity(71), vec![]);
        authority.insert(reviewed.clone());

        let missing_evidence = RecordId::from_bytes([0xBB; 32]);

        let payload = ReviewerFindingPayload::new(
            reviewed.id(),
            reviewer,
            FindingKind::Support,
            FindingSeverity::Low,
            "review".to_owned(),
            vec![missing_evidence],
            "none".to_owned(),
            IndependenceResult::Unknown,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_reviewer_finding(
            reviewer,
            reviewed.subject_id(),
            identity(72),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::AbsentReferencedRecord)
        );
    }

    #[test]
    fn malformed_typed_candidate_payload_fails_closed() {
        let state = TestStateRef(1);
        let authority = TestAuthority::new(state.clone());

        let candidate = EvidenceRecord::new(
            RecordKind::ReviewerFinding,
            identity(80),
            identity(81),
            identity(82),
            vec![],
            vec![0x01],
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::MalformedCandidatePayload)
        );
    }

    #[test]
    fn evaluation_preserves_candidate_bytes_and_replays_deterministically() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let reviewer = identity(90);
        let reviewed = generic_record(RecordKind::Claim, 20, identity(91), vec![]);
        authority.insert(reviewed.clone());

        let payload = ReviewerFindingPayload::new(
            reviewed.id(),
            reviewer,
            FindingKind::Inconclusive,
            FindingSeverity::Informational,
            "review".to_owned(),
            vec![],
            "none".to_owned(),
            IndependenceResult::Unknown,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_reviewer_finding(
            reviewer,
            reviewed.subject_id(),
            identity(92),
            vec![],
            payload,
        )
        .unwrap();

        let before = candidate.encode();

        let first = evaluate_admission(&candidate, &authority, &state);
        let second = evaluate_admission(&candidate, &authority, &state);

        assert_eq!(first, Ok(EvidenceAdmissionResult::Admissible));
        assert_eq!(first, second);
        assert_eq!(candidate.encode(), before);
    }
}
