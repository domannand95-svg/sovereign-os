use crate::{EvidencePayloadError, EvidenceRecord, RecordId, RecordKind};
use sovereign_registry::IdentityId;
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
    IndependenceUnavailable,
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

/// Whether the exact governing evidence policy requires reviewer independence.
///
/// This is authoritative admission state. It is not inferred from
/// `ReviewerFindingPayload::independence_result`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewerIndependenceRequirement {
    Required,
    NotRequired,
}

/// Exact evidence that authoritative policy and criteria require a Disposition
/// to preserve for one decided record.
///
/// The authority determines which records are required. The evaluator only
/// checks whether the immutable candidate includes those exact identifiers; it
/// does not infer negativity, minority status, failure relevance, or dispute
/// relevance from payload contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispositionEvidenceRequirements {
    required_failed_attempt_ids: Vec<RecordId>,
    required_negative_finding_ids: Vec<RecordId>,
    required_unresolved_dispute_ids: Vec<RecordId>,
}

impl DispositionEvidenceRequirements {
    pub fn new(
        required_failed_attempt_ids: Vec<RecordId>,
        required_negative_finding_ids: Vec<RecordId>,
        required_unresolved_dispute_ids: Vec<RecordId>,
    ) -> Self {
        Self {
            required_failed_attempt_ids,
            required_negative_finding_ids,
            required_unresolved_dispute_ids,
        }
    }

    pub fn required_failed_attempt_ids(&self) -> &[RecordId] {
        &self.required_failed_attempt_ids
    }

    pub fn required_negative_finding_ids(&self) -> &[RecordId] {
        &self.required_negative_finding_ids
    }

    pub fn required_unresolved_dispute_ids(&self) -> &[RecordId] {
        &self.required_unresolved_dispute_ids
    }
}

/// Authoritative validity of a resolved Dispute's resolution relationship.
///
/// `Valid` means the supplied authoritative state establishes that the exact
/// resolution record is a valid later Disposition for the exact Dispute.
/// The evaluator does not infer this relationship from payload fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoritativeDisputeResolution {
    Valid,
    Invalid,
}

/// Exact A04 relationship class subject to the normative cycle prohibition.
///
/// This is deliberately narrower than a generic graph-edge taxonomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceRelationshipKind {
    Retry,
    Supersession,
    Resolution,
}

/// Authoritative result for one proposed A04 relationship edge.
///
/// The authority determines this result from the explicitly supplied state.
/// The evaluator does not traverse, infer, or allocate a graph representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoritativeRelationshipCycle {
    Acyclic,
    Cycle,
}

/// Authoritative activation state for one exact future A05 reference.
///
/// Active means the governing A05 schema or reference class for the exact
/// supplied reference is active in the explicitly supplied authoritative state.
/// This is admission metadata only and grants no resource-consumption,
/// capability, execution, tool, publication, promotion, or mutation authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoritativeA05ReferenceActivation {
    Active,
    Inactive,
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

    /// Resolve whether the candidate's exact governing evidence policy
    /// requires reviewer independence.
    ///
    /// Implementations must not substitute ambient, current, latest,
    /// inherited, or default policy state.
    fn reviewer_independence_requirement(
        &self,
        policy_id: &IdentityId,
        state_ref: &Self::StateRef,
    ) -> Result<ReviewerIndependenceRequirement, EvidenceAdmissionAuthorityError>;

    /// Resolve authoritative reviewer-to-subject independence.
    ///
    /// An adapter may account for common control, model lineage, shared hidden
    /// context or data, delegated authority, and material conflicts. This
    /// interface deliberately does not allocate a production controller store.
    fn reviewer_independence(
        &self,
        reviewer_id: &IdentityId,
        reviewed_subject_id: &IdentityId,
        state_ref: &Self::StateRef,
    ) -> Result<AuthoritativeIndependence, EvidenceAdmissionAuthorityError>;

    /// Resolve the complete evidence set required for one Disposition.
    ///
    /// Requirements are bound to the candidate's exact governing policy,
    /// payload criteria, decided record, and explicitly supplied state.
    /// Implementations must not substitute ambient, current, latest, inherited,
    /// or default requirements.
    fn disposition_evidence_requirements(
        &self,
        policy_id: &IdentityId,
        criteria_id: &IdentityId,
        decided_id: &RecordId,
        state_ref: &Self::StateRef,
    ) -> Result<DispositionEvidenceRequirements, EvidenceAdmissionAuthorityError>;

    /// Resolve whether an exact Dispute-to-Disposition resolution relationship
    /// is valid in the explicitly supplied authoritative state.
    ///
    /// This includes the normative requirement that the resolution be a later
    /// valid Disposition. The evaluator does not invent a `decided_id`
    /// relationship or use ambient ordering.
    fn dispute_resolution_relationship(
        &self,
        dispute_id: &RecordId,
        disputed_id: &RecordId,
        resolution_id: &RecordId,
        state_ref: &Self::StateRef,
    ) -> Result<AuthoritativeDisputeResolution, EvidenceAdmissionAuthorityError>;

    /// Resolve whether one exact proposed A04 relationship participates in a
    /// prohibited cycle in the explicitly supplied authoritative state.
    ///
    /// `candidate_id` is the immutable candidate record being evaluated and
    /// `referenced_id` is the exact relationship target carried by its payload.
    /// Implementations must not substitute ambient/current graph state or infer
    /// a different relationship class.
    fn relationship_cycle(
        &self,
        relationship_kind: EvidenceRelationshipKind,
        candidate_id: &RecordId,
        referenced_id: &RecordId,
        state_ref: &Self::StateRef,
    ) -> Result<AuthoritativeRelationshipCycle, EvidenceAdmissionAuthorityError>;

    /// Resolve whether the governing A05 schema or reference class for one
    /// exact future A05 reference is active in the supplied authoritative state.
    ///
    /// Activation must not be inferred from payload presence, record existence,
    /// Capability V1 state, resource availability, or ambient/latest state.
    fn a05_reference_activation(
        &self,
        reference_id: &RecordId,
        state_ref: &Self::StateRef,
    ) -> Result<AuthoritativeA05ReferenceActivation, EvidenceAdmissionAuthorityError>;
}
/// Normative kind requirement for a typed A04 record reference.
///
/// This type represents only kind constraints explicitly frozen by
/// `SPEC-EV-001`. References described merely as governed records or evidence
/// remain existence-only until a narrower normative kind is specified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKindRequirement {
    Objective,
    Uncertainty,
    Method,
    FailedAttempt,
    ClaimOrReviewerFinding,
    Dispute,
    Disposition,
}

impl RecordKindRequirement {
    fn accepts(self, actual: RecordKind) -> bool {
        match self {
            Self::Objective => actual == RecordKind::Objective,
            Self::Uncertainty => actual == RecordKind::Uncertainty,
            Self::Method => actual == RecordKind::Method,
            Self::FailedAttempt => actual == RecordKind::FailedAttempt,
            Self::ClaimOrReviewerFinding => {
                matches!(actual, RecordKind::Claim | RecordKind::ReviewerFinding)
            }
            Self::Dispute => actual == RecordKind::Dispute,
            Self::Disposition => actual == RecordKind::Disposition,
        }
    }
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
        required: RecordKindRequirement,
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
            EvidenceAdmissionAuthorityError::IndependenceUnavailable => {
                Self::AuthoritativeIndependenceUnavailable
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
/// This increment evaluates:
///
/// - authoritative-state availability;
/// - exact resolution of every envelope parent;
/// - exact existence of governed-record and evidence references;
/// - the explicit record-kind relationships frozen by `SPEC-EV-001`;
/// - self-review rejection; and
/// - self-disposition rejection.
///
/// `MethodPayload::budget_reference` is intentionally excluded here because it
/// is governed by the separate premature-A05-reference rule. No specific kind
/// is imposed on references whose normative text says only governed record,
/// evidence record, source/dataset/artifact, or supersession target.
///
/// Remaining Section 15 rules are implemented only in later owner-approved
/// increments.
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
        RecordKind::Claim => evaluate_claim(candidate, authority, state_ref)?,
        RecordKind::Method => evaluate_method(candidate, authority, state_ref)?,
        RecordKind::Uncertainty => evaluate_uncertainty(candidate, authority, state_ref)?,
        RecordKind::FailedAttempt => evaluate_failed_attempt(candidate, authority, state_ref)?,
        RecordKind::ReviewerFinding => {
            evaluate_reviewer_finding(candidate, authority, state_ref)?;
        }
        RecordKind::Dispute => evaluate_dispute(candidate, authority, state_ref)?,
        RecordKind::Disposition => evaluate_disposition(candidate, authority, state_ref)?,
        RecordKind::Objective | RecordKind::Source => {}
    }

    Ok(EvidenceAdmissionResult::Admissible)
}

fn evaluate_claim<A: EvidenceAdmissionAuthority>(
    candidate: &EvidenceRecord,
    authority: &A,
    state_ref: &A::StateRef,
) -> Result<(), EvidenceAdmissionError> {
    let payload = candidate
        .decode_claim_payload()
        .map_err(map_payload_error)?;

    resolve_required_kind(
        authority,
        state_ref,
        &payload.objective_id(),
        RecordKindRequirement::Objective,
    )?;

    for evidence_id in payload.supporting_evidence_ids() {
        resolve_exact(authority, state_ref, evidence_id)?;
    }

    for evidence_id in payload.counter_evidence_ids() {
        resolve_exact(authority, state_ref, evidence_id)?;
    }

    for uncertainty_id in payload.uncertainty_ids() {
        resolve_required_kind(
            authority,
            state_ref,
            uncertainty_id,
            RecordKindRequirement::Uncertainty,
        )?;
    }

    Ok(())
}

fn evaluate_method<A: EvidenceAdmissionAuthority>(
    candidate: &EvidenceRecord,
    authority: &A,
    state_ref: &A::StateRef,
) -> Result<(), EvidenceAdmissionError> {
    let payload = candidate
        .decode_method_payload()
        .map_err(map_payload_error)?;

    resolve_required_kind(
        authority,
        state_ref,
        &payload.objective_id(),
        RecordKindRequirement::Objective,
    )?;

    for input_id in payload.input_ids() {
        resolve_exact(authority, state_ref, input_id)?;
    }

    if let Some(budget_reference) = payload.budget_reference() {
        match authority.a05_reference_activation(&budget_reference, state_ref)? {
            AuthoritativeA05ReferenceActivation::Active => {}
            AuthoritativeA05ReferenceActivation::Inactive => {
                return Err(EvidenceAdmissionError::PrematureA05Reference);
            }
        }
    }

    Ok(())
}

fn evaluate_uncertainty<A: EvidenceAdmissionAuthority>(
    candidate: &EvidenceRecord,
    authority: &A,
    state_ref: &A::StateRef,
) -> Result<(), EvidenceAdmissionError> {
    let payload = candidate
        .decode_uncertainty_payload()
        .map_err(map_payload_error)?;

    resolve_exact(authority, state_ref, &payload.about_id())?;
    Ok(())
}

fn evaluate_failed_attempt<A: EvidenceAdmissionAuthority>(
    candidate: &EvidenceRecord,
    authority: &A,
    state_ref: &A::StateRef,
) -> Result<(), EvidenceAdmissionError> {
    let payload = candidate
        .decode_failed_attempt_payload()
        .map_err(map_payload_error)?;

    resolve_required_kind(
        authority,
        state_ref,
        &payload.objective_id(),
        RecordKindRequirement::Objective,
    )?;

    resolve_required_kind(
        authority,
        state_ref,
        &payload.method_id(),
        RecordKindRequirement::Method,
    )?;

    for evidence_id in payload.evidence_ids() {
        resolve_exact(authority, state_ref, evidence_id)?;
    }

    if let Some(retry_of) = payload.retry_of() {
        resolve_required_kind(
            authority,
            state_ref,
            &retry_of,
            RecordKindRequirement::FailedAttempt,
        )?;

        ensure_acyclic_relationship(
            authority,
            state_ref,
            EvidenceRelationshipKind::Retry,
            &candidate.id(),
            &retry_of,
        )?;
    }

    Ok(())
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

    let policy_id = candidate.policy_id();
    let requirement = authority.reviewer_independence_requirement(&policy_id, state_ref)?;

    if requirement == ReviewerIndependenceRequirement::NotRequired {
        return Ok(());
    }

    match authority.reviewer_independence(
        &payload.reviewer_id(),
        &reviewed.subject_id(),
        state_ref,
    )? {
        AuthoritativeIndependence::Established => Ok(()),
        AuthoritativeIndependence::NotEstablished | AuthoritativeIndependence::Conflicted => {
            Err(EvidenceAdmissionError::IndependenceNotEstablished)
        }
        AuthoritativeIndependence::Unknown => {
            Err(EvidenceAdmissionError::AuthoritativeIndependenceUnavailable)
        }
    }
}

fn evaluate_dispute<A: EvidenceAdmissionAuthority>(
    candidate: &EvidenceRecord,
    authority: &A,
    state_ref: &A::StateRef,
) -> Result<(), EvidenceAdmissionError> {
    let payload = candidate
        .decode_dispute_payload()
        .map_err(map_payload_error)?;

    resolve_exact(authority, state_ref, &payload.disputed_id())?;

    for position_id in payload.position_ids() {
        resolve_required_kind(
            authority,
            state_ref,
            position_id,
            RecordKindRequirement::ClaimOrReviewerFinding,
        )?;
    }

    if let Some(resolution_id) = payload.resolution_id() {
        resolve_required_kind(
            authority,
            state_ref,
            &resolution_id,
            RecordKindRequirement::Disposition,
        )?;

        let dispute_id = candidate.id();
        let disputed_id = payload.disputed_id();

        match authority.dispute_resolution_relationship(
            &dispute_id,
            &disputed_id,
            &resolution_id,
            state_ref,
        )? {
            AuthoritativeDisputeResolution::Valid => {}
            AuthoritativeDisputeResolution::Invalid => {
                return Err(EvidenceAdmissionError::InvalidResolvedDisputeRelationship);
            }
        }

        ensure_acyclic_relationship(
            authority,
            state_ref,
            EvidenceRelationshipKind::Resolution,
            &dispute_id,
            &resolution_id,
        )?;
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
        resolve_required_kind(
            authority,
            state_ref,
            dispute_id,
            RecordKindRequirement::Dispute,
        )?;
    }

    if let Some(supersedes_id) = payload.supersedes_id() {
        resolve_exact(authority, state_ref, &supersedes_id)?;

        ensure_acyclic_relationship(
            authority,
            state_ref,
            EvidenceRelationshipKind::Supersession,
            &candidate.id(),
            &supersedes_id,
        )?;
    }

    if payload.decision_authority_id() == decided.subject_id() {
        return Err(EvidenceAdmissionError::SelfDisposition);
    }

    let policy_id = candidate.policy_id();
    let criteria_id = payload.criteria_id();
    let decided_id = payload.decided_id();

    let requirements = authority.disposition_evidence_requirements(
        &policy_id,
        &criteria_id,
        &decided_id,
        state_ref,
    )?;

    let evidence_ids = payload.evidence_ids();

    if requirements
        .required_failed_attempt_ids()
        .iter()
        .chain(requirements.required_negative_finding_ids().iter())
        .any(|required_id| !evidence_ids.contains(required_id))
    {
        return Err(EvidenceAdmissionError::RequiredEvidenceOmitted);
    }

    if requirements
        .required_unresolved_dispute_ids()
        .iter()
        .any(|required_id| !payload.unresolved_dispute_ids().contains(required_id))
    {
        return Err(EvidenceAdmissionError::RequiredEvidenceOmitted);
    }

    Ok(())
}

fn ensure_acyclic_relationship<A: EvidenceAdmissionAuthority>(
    authority: &A,
    state_ref: &A::StateRef,
    relationship_kind: EvidenceRelationshipKind,
    candidate_id: &RecordId,
    referenced_id: &RecordId,
) -> Result<(), EvidenceAdmissionError> {
    match authority.relationship_cycle(relationship_kind, candidate_id, referenced_id, state_ref)? {
        AuthoritativeRelationshipCycle::Acyclic => Ok(()),
        AuthoritativeRelationshipCycle::Cycle => Err(EvidenceAdmissionError::RelationshipCycle),
    }
}
fn resolve_required_kind<A: EvidenceAdmissionAuthority>(
    authority: &A,
    state_ref: &A::StateRef,
    requested_id: &RecordId,
    required: RecordKindRequirement,
) -> Result<EvidenceRecord, EvidenceAdmissionError> {
    let resolved = resolve_exact(authority, state_ref, requested_id)?;

    if !required.accepts(resolved.kind()) {
        return Err(EvidenceAdmissionError::WrongReferencedRecordKind {
            required,
            actual: resolved.kind(),
        });
    }

    Ok(resolved)
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
        ClaimKind, ClaimPayload, DispositionDecision, DispositionPayload, DisputePayload,
        DisputeStatus, FailedAttemptPayload, FailureKind, FindingKind, FindingSeverity,
        IndependenceResult, ReviewerFindingPayload, Substantiation,
    };
    use sovereign_registry::{IdentityKind, IdentityRecord};
    use std::collections::BTreeMap;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestStateRef(u8);

    impl EvidenceAdmissionStateRef for TestStateRef {}

    struct TestAuthority {
        expected_state: TestStateRef,
        records: BTreeMap<RecordId, EvidenceRecord>,
        substitute: Option<EvidenceRecord>,
        independence_requirement: ReviewerIndependenceRequirement,
        independence: AuthoritativeIndependence,
        requirement_error: Option<EvidenceAdmissionAuthorityError>,
        independence_error: Option<EvidenceAdmissionAuthorityError>,
        forbid_independence_lookup: bool,
        expected_policy_id: Option<IdentityId>,
        disposition_requirements: DispositionEvidenceRequirements,
        disposition_requirements_error: Option<EvidenceAdmissionAuthorityError>,
        expected_criteria_id: Option<IdentityId>,
        expected_decided_id: Option<RecordId>,
        dispute_resolution: AuthoritativeDisputeResolution,
        dispute_resolution_error: Option<EvidenceAdmissionAuthorityError>,
        forbid_dispute_resolution_lookup: bool,
        expected_dispute_id: Option<RecordId>,
        expected_disputed_id: Option<RecordId>,
        expected_resolution_id: Option<RecordId>,
        cycle_result: AuthoritativeRelationshipCycle,
        cycle_error: Option<EvidenceAdmissionAuthorityError>,
        forbid_cycle_lookup: bool,
        expected_cycle_kind: Option<EvidenceRelationshipKind>,
        expected_cycle_candidate_id: Option<RecordId>,
        expected_cycle_referenced_id: Option<RecordId>,
        a05_activation: AuthoritativeA05ReferenceActivation,
        a05_activation_error: Option<EvidenceAdmissionAuthorityError>,
        forbid_a05_activation_lookup: bool,
        expected_a05_reference_id: Option<RecordId>,
    }

    impl TestAuthority {
        fn new(expected_state: TestStateRef) -> Self {
            Self {
                expected_state,
                records: BTreeMap::new(),
                substitute: None,
                independence_requirement: ReviewerIndependenceRequirement::NotRequired,
                independence: AuthoritativeIndependence::Established,
                requirement_error: None,
                independence_error: None,
                forbid_independence_lookup: false,
                expected_policy_id: None,
                disposition_requirements: DispositionEvidenceRequirements::new(
                    vec![],
                    vec![],
                    vec![],
                ),
                disposition_requirements_error: None,
                expected_criteria_id: None,
                expected_decided_id: None,
                dispute_resolution: AuthoritativeDisputeResolution::Valid,
                dispute_resolution_error: None,
                forbid_dispute_resolution_lookup: false,
                expected_dispute_id: None,
                expected_disputed_id: None,
                expected_resolution_id: None,
                cycle_result: AuthoritativeRelationshipCycle::Acyclic,
                cycle_error: None,
                forbid_cycle_lookup: false,
                expected_cycle_kind: None,
                expected_cycle_candidate_id: None,
                expected_cycle_referenced_id: None,
                a05_activation: AuthoritativeA05ReferenceActivation::Inactive,
                a05_activation_error: None,
                forbid_a05_activation_lookup: false,
                expected_a05_reference_id: None,
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

        fn reviewer_independence_requirement(
            &self,
            policy_id: &IdentityId,
            state_ref: &Self::StateRef,
        ) -> Result<ReviewerIndependenceRequirement, EvidenceAdmissionAuthorityError> {
            self.validate_state_ref(state_ref)?;

            if let Some(expected_policy_id) = self.expected_policy_id {
                assert_eq!(
                    *policy_id, expected_policy_id,
                    "admission must query the candidate's exact explicit policy_id"
                );
            }

            if let Some(error) = self.requirement_error {
                return Err(error);
            }

            Ok(self.independence_requirement)
        }

        fn reviewer_independence(
            &self,
            _reviewer_id: &IdentityId,
            _reviewed_subject_id: &IdentityId,
            state_ref: &Self::StateRef,
        ) -> Result<AuthoritativeIndependence, EvidenceAdmissionAuthorityError> {
            self.validate_state_ref(state_ref)?;

            assert!(
                !self.forbid_independence_lookup,
                "independence lookup must not occur when policy does not require it"
            );

            if let Some(error) = self.independence_error {
                return Err(error);
            }

            Ok(self.independence)
        }

        fn disposition_evidence_requirements(
            &self,
            policy_id: &IdentityId,
            criteria_id: &IdentityId,
            decided_id: &RecordId,
            state_ref: &Self::StateRef,
        ) -> Result<DispositionEvidenceRequirements, EvidenceAdmissionAuthorityError> {
            self.validate_state_ref(state_ref)?;

            if let Some(expected_policy_id) = self.expected_policy_id {
                assert_eq!(
                    *policy_id, expected_policy_id,
                    "Disposition completeness must use the candidate's exact policy_id"
                );
            }

            if let Some(expected_criteria_id) = self.expected_criteria_id {
                assert_eq!(
                    *criteria_id, expected_criteria_id,
                    "Disposition completeness must use the payload's exact criteria_id"
                );
            }

            if let Some(expected_decided_id) = self.expected_decided_id {
                assert_eq!(
                    *decided_id, expected_decided_id,
                    "Disposition completeness must use the payload's exact decided_id"
                );
            }

            if let Some(error) = self.disposition_requirements_error {
                return Err(error);
            }

            Ok(self.disposition_requirements.clone())
        }

        fn dispute_resolution_relationship(
            &self,
            dispute_id: &RecordId,
            disputed_id: &RecordId,
            resolution_id: &RecordId,
            state_ref: &Self::StateRef,
        ) -> Result<AuthoritativeDisputeResolution, EvidenceAdmissionAuthorityError> {
            self.validate_state_ref(state_ref)?;

            assert!(
                !self.forbid_dispute_resolution_lookup,
                "dispute resolution relationship lookup must not occur"
            );

            if let Some(expected_dispute_id) = self.expected_dispute_id {
                assert_eq!(
                    *dispute_id, expected_dispute_id,
                    "resolution validation must use the exact candidate Dispute ID"
                );
            }

            if let Some(expected_disputed_id) = self.expected_disputed_id {
                assert_eq!(
                    *disputed_id, expected_disputed_id,
                    "resolution validation must use the exact disputed_id"
                );
            }

            if let Some(expected_resolution_id) = self.expected_resolution_id {
                assert_eq!(
                    *resolution_id, expected_resolution_id,
                    "resolution validation must use the exact resolution_id"
                );
            }

            if let Some(error) = self.dispute_resolution_error {
                return Err(error);
            }

            Ok(self.dispute_resolution)
        }

        fn relationship_cycle(
            &self,
            relationship_kind: EvidenceRelationshipKind,
            candidate_id: &RecordId,
            referenced_id: &RecordId,
            state_ref: &Self::StateRef,
        ) -> Result<AuthoritativeRelationshipCycle, EvidenceAdmissionAuthorityError> {
            self.validate_state_ref(state_ref)?;

            assert!(
                !self.forbid_cycle_lookup,
                "relationship cycle lookup must not occur when no governed relationship is present"
            );

            if let Some(expected_kind) = self.expected_cycle_kind {
                assert_eq!(
                    relationship_kind, expected_kind,
                    "cycle validation must use the exact normative relationship kind"
                );
            }

            if let Some(expected_candidate_id) = self.expected_cycle_candidate_id {
                assert_eq!(
                    *candidate_id, expected_candidate_id,
                    "cycle validation must use the exact immutable candidate ID"
                );
            }

            if let Some(expected_referenced_id) = self.expected_cycle_referenced_id {
                assert_eq!(
                    *referenced_id, expected_referenced_id,
                    "cycle validation must use the exact referenced relationship target"
                );
            }

            if let Some(error) = self.cycle_error {
                return Err(error);
            }

            Ok(self.cycle_result)
        }

        fn a05_reference_activation(
            &self,
            reference_id: &RecordId,
            state_ref: &Self::StateRef,
        ) -> Result<AuthoritativeA05ReferenceActivation, EvidenceAdmissionAuthorityError> {
            self.validate_state_ref(state_ref)?;

            assert!(
                !self.forbid_a05_activation_lookup,
                "A05 activation lookup must not occur without a budget_reference"
            );

            if let Some(expected) = self.expected_a05_reference_id {
                assert_eq!(
                    *reference_id, expected,
                    "A05 activation must use the exact Method budget_reference"
                );
            }

            if let Some(error) = self.a05_activation_error {
                return Err(error);
            }

            Ok(self.a05_activation)
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

        let candidate = generic_record(RecordKind::Objective, 11, identity(41), vec![parent.id()]);

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

    #[test]
    fn claim_objective_requires_objective_kind() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let wrong_objective = generic_record(RecordKind::Source, 21, identity(101), vec![]);
        authority.insert(wrong_objective.clone());

        let payload = ClaimPayload::new(
            wrong_objective.id(),
            "claim".to_owned(),
            ClaimKind::Observation,
            Substantiation::Unsubstantiated,
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let candidate =
            EvidenceRecord::new_claim(identity(102), identity(103), identity(104), vec![], payload)
                .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::WrongReferencedRecordKind {
                required: RecordKindRequirement::Objective,
                actual: RecordKind::Source,
            })
        );
    }

    #[test]
    fn claim_uncertainty_requires_uncertainty_kind() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let objective = generic_record(RecordKind::Objective, 22, identity(105), vec![]);
        let wrong_uncertainty = generic_record(RecordKind::Method, 23, identity(106), vec![]);

        authority.insert(objective.clone());
        authority.insert(wrong_uncertainty.clone());

        let payload = ClaimPayload::new(
            objective.id(),
            "claim".to_owned(),
            ClaimKind::Inference,
            Substantiation::Unsubstantiated,
            vec![],
            vec![],
            vec![wrong_uncertainty.id()],
        )
        .unwrap();

        let candidate =
            EvidenceRecord::new_claim(identity(107), identity(108), identity(109), vec![], payload)
                .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::WrongReferencedRecordKind {
                required: RecordKindRequirement::Uncertainty,
                actual: RecordKind::Method,
            })
        );
    }

    #[test]
    fn failed_attempt_retry_requires_failed_attempt_kind() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let objective = generic_record(RecordKind::Objective, 24, identity(110), vec![]);
        let method = generic_record(RecordKind::Method, 25, identity(111), vec![]);
        let wrong_retry = generic_record(RecordKind::Claim, 26, identity(112), vec![]);

        authority.insert(objective.clone());
        authority.insert(method.clone());
        authority.insert(wrong_retry.clone());

        let payload = FailedAttemptPayload::new(
            objective.id(),
            method.id(),
            FailureKind::MethodFailure,
            "failed".to_owned(),
            vec![],
            Some(wrong_retry.id()),
        )
        .unwrap();

        let candidate = EvidenceRecord::new_failed_attempt(
            identity(113),
            identity(114),
            identity(115),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::WrongReferencedRecordKind {
                required: RecordKindRequirement::FailedAttempt,
                actual: RecordKind::Claim,
            })
        );
    }

    #[test]
    fn dispute_positions_accept_claim_and_reviewer_finding() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let disputed = generic_record(RecordKind::Objective, 27, identity(116), vec![]);
        let claim_position = generic_record(RecordKind::Claim, 28, identity(117), vec![]);
        let finding_position =
            generic_record(RecordKind::ReviewerFinding, 29, identity(118), vec![]);

        authority.insert(disputed.clone());
        authority.insert(claim_position.clone());
        authority.insert(finding_position.clone());

        let payload = DisputePayload::new(
            disputed.id(),
            vec![claim_position.id(), finding_position.id()],
            identity(119),
            DisputeStatus::Open,
            None,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_dispute(
            identity(120),
            identity(121),
            identity(122),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn dispute_position_rejects_other_record_kinds() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let disputed = generic_record(RecordKind::Source, 30, identity(123), vec![]);
        let claim_position = generic_record(RecordKind::Claim, 31, identity(124), vec![]);
        let wrong_position = generic_record(RecordKind::Objective, 32, identity(125), vec![]);

        authority.insert(disputed.clone());
        authority.insert(claim_position.clone());
        authority.insert(wrong_position.clone());

        let payload = DisputePayload::new(
            disputed.id(),
            vec![claim_position.id(), wrong_position.id()],
            identity(126),
            DisputeStatus::Open,
            None,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_dispute(
            identity(127),
            identity(128),
            identity(129),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::WrongReferencedRecordKind {
                required: RecordKindRequirement::ClaimOrReviewerFinding,
                actual: RecordKind::Objective,
            })
        );
    }

    #[test]
    fn resolved_dispute_resolution_requires_disposition_kind() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.forbid_dispute_resolution_lookup = true;

        let disputed = generic_record(RecordKind::Objective, 33, identity(130), vec![]);
        let first_position = generic_record(RecordKind::Claim, 34, identity(131), vec![]);
        let second_position =
            generic_record(RecordKind::ReviewerFinding, 35, identity(132), vec![]);
        let wrong_resolution = generic_record(RecordKind::Claim, 36, identity(133), vec![]);

        authority.insert(disputed.clone());
        authority.insert(first_position.clone());
        authority.insert(second_position.clone());
        authority.insert(wrong_resolution.clone());

        let payload = DisputePayload::new(
            disputed.id(),
            vec![first_position.id(), second_position.id()],
            identity(134),
            DisputeStatus::Resolved,
            Some(wrong_resolution.id()),
        )
        .unwrap();

        let candidate = EvidenceRecord::new_dispute(
            identity(135),
            identity(136),
            identity(137),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::WrongReferencedRecordKind {
                required: RecordKindRequirement::Disposition,
                actual: RecordKind::Claim,
            })
        );
    }

    #[test]
    fn disposition_unresolved_reference_requires_dispute_kind() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decided = generic_record(RecordKind::Claim, 37, identity(138), vec![]);
        let wrong_dispute = generic_record(RecordKind::Claim, 38, identity(139), vec![]);

        authority.insert(decided.clone());
        authority.insert(wrong_dispute.clone());

        let decision_authority = identity(140);

        let payload = DispositionPayload::new(
            decided.id(),
            DispositionDecision::Defer,
            decision_authority,
            identity(141),
            vec![],
            vec![wrong_dispute.id()],
            "defer".to_owned(),
            None,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_disposition(
            identity(142),
            identity(143),
            identity(144),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::WrongReferencedRecordKind {
                required: RecordKindRequirement::Dispute,
                actual: RecordKind::Claim,
            })
        );
    }
    fn reviewer_finding_candidate(
        reviewed: &EvidenceRecord,
        reviewer: IdentityId,
        policy_id: IdentityId,
        payload_independence: IndependenceResult,
    ) -> EvidenceRecord {
        let payload = ReviewerFindingPayload::new(
            reviewed.id(),
            reviewer,
            FindingKind::Support,
            FindingSeverity::Informational,
            "review".to_owned(),
            vec![],
            "NONE_DECLARED".to_owned(),
            payload_independence,
        )
        .unwrap();

        EvidenceRecord::new_reviewer_finding(
            identity(200),
            reviewed.subject_id(),
            policy_id,
            vec![],
            payload,
        )
        .unwrap()
    }

    #[test]
    fn independence_is_not_queried_when_policy_does_not_require_it() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.forbid_independence_lookup = true;

        let reviewed = generic_record(RecordKind::Claim, 39, identity(150), vec![]);
        authority.insert(reviewed.clone());

        let policy_id = identity(151);
        authority.expected_policy_id = Some(policy_id);

        let candidate = reviewer_finding_candidate(
            &reviewed,
            identity(152),
            policy_id,
            IndependenceResult::Unknown,
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn authoritative_established_independence_passes_when_required() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.independence_requirement = ReviewerIndependenceRequirement::Required;
        authority.independence = AuthoritativeIndependence::Established;

        let reviewed = generic_record(RecordKind::Claim, 40, identity(153), vec![]);
        authority.insert(reviewed.clone());

        let policy_id = identity(154);
        authority.expected_policy_id = Some(policy_id);

        let candidate = reviewer_finding_candidate(
            &reviewed,
            identity(155),
            policy_id,
            IndependenceResult::Unknown,
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn payload_established_cannot_override_authoritative_not_established() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.independence_requirement = ReviewerIndependenceRequirement::Required;
        authority.independence = AuthoritativeIndependence::NotEstablished;

        let reviewed = generic_record(RecordKind::Claim, 41, identity(156), vec![]);
        authority.insert(reviewed.clone());

        let policy_id = identity(157);
        let candidate = reviewer_finding_candidate(
            &reviewed,
            identity(158),
            policy_id,
            IndependenceResult::Established,
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::IndependenceNotEstablished)
        );
    }

    #[test]
    fn authoritative_conflict_fails_closed_when_independence_is_required() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.independence_requirement = ReviewerIndependenceRequirement::Required;
        authority.independence = AuthoritativeIndependence::Conflicted;

        let reviewed = generic_record(RecordKind::Claim, 42, identity(159), vec![]);
        authority.insert(reviewed.clone());

        let candidate = reviewer_finding_candidate(
            &reviewed,
            identity(160),
            identity(161),
            IndependenceResult::Established,
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::IndependenceNotEstablished)
        );
    }

    #[test]
    fn authoritative_unknown_independence_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.independence_requirement = ReviewerIndependenceRequirement::Required;
        authority.independence = AuthoritativeIndependence::Unknown;

        let reviewed = generic_record(RecordKind::Claim, 43, identity(162), vec![]);
        authority.insert(reviewed.clone());

        let candidate = reviewer_finding_candidate(
            &reviewed,
            identity(163),
            identity(164),
            IndependenceResult::Established,
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::AuthoritativeIndependenceUnavailable)
        );
    }

    #[test]
    fn unavailable_authoritative_independence_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.independence_requirement = ReviewerIndependenceRequirement::Required;
        authority.independence_error =
            Some(EvidenceAdmissionAuthorityError::IndependenceUnavailable);

        let reviewed = generic_record(RecordKind::Claim, 44, identity(165), vec![]);
        authority.insert(reviewed.clone());

        let candidate = reviewer_finding_candidate(
            &reviewed,
            identity(166),
            identity(167),
            IndependenceResult::Established,
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::AuthoritativeIndependenceUnavailable)
        );
    }

    #[test]
    fn unavailable_policy_requirement_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.requirement_error =
            Some(EvidenceAdmissionAuthorityError::RequiredRelationalInformationUnavailable);

        let reviewed = generic_record(RecordKind::Claim, 45, identity(168), vec![]);
        authority.insert(reviewed.clone());

        let candidate = reviewer_finding_candidate(
            &reviewed,
            identity(169),
            identity(170),
            IndependenceResult::Established,
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RequiredRelationalInformationUnavailable)
        );
    }
    fn disposition_completeness_candidate(
        decided: &EvidenceRecord,
        decision_authority: IdentityId,
        policy_id: IdentityId,
        criteria_id: IdentityId,
        evidence_ids: Vec<RecordId>,
        unresolved_dispute_ids: Vec<RecordId>,
    ) -> EvidenceRecord {
        let payload = DispositionPayload::new(
            decided.id(),
            DispositionDecision::Defer,
            decision_authority,
            criteria_id,
            evidence_ids,
            unresolved_dispute_ids,
            "completeness decision".to_owned(),
            None,
        )
        .unwrap();

        EvidenceRecord::new_disposition(
            identity(201),
            decided.subject_id(),
            policy_id,
            vec![],
            payload,
        )
        .unwrap()
    }

    #[test]
    fn disposition_all_authoritative_requirements_present_is_admissible() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decided = generic_record(RecordKind::Claim, 46, identity(171), vec![]);
        let failed_attempt = generic_record(RecordKind::FailedAttempt, 47, identity(172), vec![]);
        let negative_finding =
            generic_record(RecordKind::ReviewerFinding, 48, identity(173), vec![]);
        let unresolved_dispute = generic_record(RecordKind::Dispute, 49, identity(174), vec![]);

        authority.insert(decided.clone());
        authority.insert(failed_attempt.clone());
        authority.insert(negative_finding.clone());
        authority.insert(unresolved_dispute.clone());

        authority.disposition_requirements = DispositionEvidenceRequirements::new(
            vec![failed_attempt.id()],
            vec![negative_finding.id()],
            vec![unresolved_dispute.id()],
        );

        let candidate = disposition_completeness_candidate(
            &decided,
            identity(175),
            identity(176),
            identity(177),
            vec![failed_attempt.id(), negative_finding.id()],
            vec![unresolved_dispute.id()],
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn disposition_omitted_required_failed_attempt_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decided = generic_record(RecordKind::Claim, 50, identity(178), vec![]);
        let failed_attempt = generic_record(RecordKind::FailedAttempt, 51, identity(179), vec![]);

        authority.insert(decided.clone());
        authority.insert(failed_attempt.clone());

        let policy_id = identity(180);
        let criteria_id = identity(181);

        authority.expected_policy_id = Some(policy_id);
        authority.expected_criteria_id = Some(criteria_id);
        authority.expected_decided_id = Some(decided.id());

        authority.disposition_requirements =
            DispositionEvidenceRequirements::new(vec![failed_attempt.id()], vec![], vec![]);

        let candidate = disposition_completeness_candidate(
            &decided,
            identity(182),
            policy_id,
            criteria_id,
            vec![],
            vec![],
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RequiredEvidenceOmitted)
        );
    }

    #[test]
    fn disposition_omitted_required_negative_finding_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decided = generic_record(RecordKind::Claim, 52, identity(183), vec![]);
        let negative_finding =
            generic_record(RecordKind::ReviewerFinding, 53, identity(184), vec![]);

        authority.insert(decided.clone());
        authority.insert(negative_finding.clone());

        authority.disposition_requirements =
            DispositionEvidenceRequirements::new(vec![], vec![negative_finding.id()], vec![]);

        let candidate = disposition_completeness_candidate(
            &decided,
            identity(185),
            identity(186),
            identity(187),
            vec![],
            vec![],
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RequiredEvidenceOmitted)
        );
    }

    #[test]
    fn disposition_omitted_required_unresolved_dispute_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decided = generic_record(RecordKind::Claim, 54, identity(188), vec![]);
        let unresolved_dispute = generic_record(RecordKind::Dispute, 55, identity(189), vec![]);

        authority.insert(decided.clone());
        authority.insert(unresolved_dispute.clone());

        authority.disposition_requirements =
            DispositionEvidenceRequirements::new(vec![], vec![], vec![unresolved_dispute.id()]);

        let candidate = disposition_completeness_candidate(
            &decided,
            identity(190),
            identity(191),
            identity(192),
            vec![],
            vec![],
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RequiredEvidenceOmitted)
        );
    }

    #[test]
    fn disposition_extra_evidence_is_permitted() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decided = generic_record(RecordKind::Claim, 56, identity(193), vec![]);
        let required = generic_record(RecordKind::FailedAttempt, 57, identity(194), vec![]);
        let extra = generic_record(RecordKind::Source, 58, identity(195), vec![]);

        authority.insert(decided.clone());
        authority.insert(required.clone());
        authority.insert(extra.clone());

        authority.disposition_requirements =
            DispositionEvidenceRequirements::new(vec![required.id()], vec![], vec![]);

        let candidate = disposition_completeness_candidate(
            &decided,
            identity(196),
            identity(197),
            identity(198),
            vec![required.id(), extra.id()],
            vec![],
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn disposition_requirement_lookup_failure_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decided = generic_record(RecordKind::Claim, 59, identity(199), vec![]);
        authority.insert(decided.clone());

        authority.disposition_requirements_error =
            Some(EvidenceAdmissionAuthorityError::RequiredRelationalInformationUnavailable);

        let candidate = disposition_completeness_candidate(
            &decided,
            identity(202),
            identity(203),
            identity(204),
            vec![],
            vec![],
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RequiredRelationalInformationUnavailable)
        );
    }

    #[test]
    fn disposition_empty_requirements_do_not_infer_negative_evidence() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decided = generic_record(RecordKind::Claim, 60, identity(205), vec![]);
        let unrelated_negative =
            generic_record(RecordKind::ReviewerFinding, 61, identity(206), vec![]);

        authority.insert(decided.clone());
        authority.insert(unrelated_negative);

        let candidate = disposition_completeness_candidate(
            &decided,
            identity(207),
            identity(208),
            identity(209),
            vec![],
            vec![],
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }
    fn dispute_resolution_fixture(
        authority: &mut TestAuthority,
        disputed_seed: u8,
    ) -> (
        EvidenceRecord,
        EvidenceRecord,
        EvidenceRecord,
        EvidenceRecord,
    ) {
        let disputed = generic_record(RecordKind::Objective, disputed_seed, identity(210), vec![]);
        let first_position = generic_record(
            RecordKind::Claim,
            disputed_seed.wrapping_add(1),
            identity(211),
            vec![],
        );
        let second_position = generic_record(
            RecordKind::ReviewerFinding,
            disputed_seed.wrapping_add(2),
            identity(212),
            vec![],
        );
        let resolution = generic_record(
            RecordKind::Disposition,
            disputed_seed.wrapping_add(3),
            identity(213),
            vec![],
        );

        authority.insert(disputed.clone());
        authority.insert(first_position.clone());
        authority.insert(second_position.clone());
        authority.insert(resolution.clone());

        (disputed, first_position, second_position, resolution)
    }

    fn resolved_dispute_candidate(
        disputed: &EvidenceRecord,
        first_position: &EvidenceRecord,
        second_position: &EvidenceRecord,
        resolution: &EvidenceRecord,
    ) -> EvidenceRecord {
        let payload = DisputePayload::new(
            disputed.id(),
            vec![first_position.id(), second_position.id()],
            identity(214),
            DisputeStatus::Resolved,
            Some(resolution.id()),
        )
        .unwrap();

        EvidenceRecord::new_dispute(
            identity(215),
            disputed.subject_id(),
            identity(216),
            vec![],
            payload,
        )
        .unwrap()
    }

    #[test]
    fn authoritative_valid_resolution_relationship_is_admissible() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let (disputed, first_position, second_position, resolution) =
            dispute_resolution_fixture(&mut authority, 62);

        let candidate =
            resolved_dispute_candidate(&disputed, &first_position, &second_position, &resolution);

        authority.expected_dispute_id = Some(candidate.id());
        authority.expected_disputed_id = Some(disputed.id());
        authority.expected_resolution_id = Some(resolution.id());
        authority.dispute_resolution = AuthoritativeDisputeResolution::Valid;

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn authoritative_invalid_resolution_relationship_is_rejected() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let (disputed, first_position, second_position, resolution) =
            dispute_resolution_fixture(&mut authority, 66);

        let candidate =
            resolved_dispute_candidate(&disputed, &first_position, &second_position, &resolution);

        authority.dispute_resolution = AuthoritativeDisputeResolution::Invalid;

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::InvalidResolvedDisputeRelationship)
        );
    }

    #[test]
    fn unavailable_resolution_relationship_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let (disputed, first_position, second_position, resolution) =
            dispute_resolution_fixture(&mut authority, 70);

        let candidate =
            resolved_dispute_candidate(&disputed, &first_position, &second_position, &resolution);

        authority.dispute_resolution_error =
            Some(EvidenceAdmissionAuthorityError::RequiredRelationalInformationUnavailable);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RequiredRelationalInformationUnavailable)
        );
    }

    #[test]
    fn unresolved_dispute_does_not_query_resolution_relationship() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.forbid_dispute_resolution_lookup = true;

        let disputed = generic_record(RecordKind::Objective, 74, identity(217), vec![]);
        let first_position = generic_record(RecordKind::Claim, 75, identity(218), vec![]);
        let second_position =
            generic_record(RecordKind::ReviewerFinding, 76, identity(219), vec![]);

        authority.insert(disputed.clone());
        authority.insert(first_position.clone());
        authority.insert(second_position.clone());

        let payload = DisputePayload::new(
            disputed.id(),
            vec![first_position.id(), second_position.id()],
            identity(220),
            DisputeStatus::Open,
            None,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_dispute(
            identity(221),
            disputed.subject_id(),
            identity(222),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn resolution_validity_is_not_inferred_from_disposition_decided_id() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let disputed = generic_record(RecordKind::Objective, 77, identity(223), vec![]);
        let unrelated_decided = generic_record(RecordKind::Claim, 78, identity(224), vec![]);
        let first_position = generic_record(RecordKind::Claim, 79, identity(225), vec![]);
        let second_position =
            generic_record(RecordKind::ReviewerFinding, 80, identity(226), vec![]);

        authority.insert(disputed.clone());
        authority.insert(unrelated_decided.clone());
        authority.insert(first_position.clone());
        authority.insert(second_position.clone());

        let resolution_payload = DispositionPayload::new(
            unrelated_decided.id(),
            DispositionDecision::Defer,
            identity(227),
            identity(228),
            vec![],
            vec![],
            "authoritative resolution relation".to_owned(),
            None,
        )
        .unwrap();

        let resolution = EvidenceRecord::new_disposition(
            identity(229),
            unrelated_decided.subject_id(),
            identity(230),
            vec![],
            resolution_payload,
        )
        .unwrap();

        authority.insert(resolution.clone());
        authority.dispute_resolution = AuthoritativeDisputeResolution::Valid;

        let candidate =
            resolved_dispute_candidate(&disputed, &first_position, &second_position, &resolution);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }
    #[test]
    fn retry_relationship_cycle_is_rejected() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let objective = generic_record(RecordKind::Objective, 220, identity(220), vec![]);
        let method = generic_record(RecordKind::Method, 221, identity(221), vec![]);
        let prior_attempt = generic_record(RecordKind::FailedAttempt, 222, identity(222), vec![]);

        authority.insert(objective.clone());
        authority.insert(method.clone());
        authority.insert(prior_attempt.clone());

        let payload = FailedAttemptPayload::new(
            objective.id(),
            method.id(),
            FailureKind::MethodFailure,
            "retry cycle".to_owned(),
            vec![],
            Some(prior_attempt.id()),
        )
        .unwrap();

        let candidate = EvidenceRecord::new_failed_attempt(
            identity(223),
            identity(224),
            identity(225),
            vec![],
            payload,
        )
        .unwrap();

        authority.cycle_result = AuthoritativeRelationshipCycle::Cycle;
        authority.expected_cycle_kind = Some(EvidenceRelationshipKind::Retry);
        authority.expected_cycle_candidate_id = Some(candidate.id());
        authority.expected_cycle_referenced_id = Some(prior_attempt.id());

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RelationshipCycle)
        );
    }

    #[test]
    fn acyclic_retry_relationship_is_admissible() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let objective = generic_record(RecordKind::Objective, 226, identity(226), vec![]);
        let method = generic_record(RecordKind::Method, 227, identity(227), vec![]);
        let prior_attempt = generic_record(RecordKind::FailedAttempt, 228, identity(228), vec![]);

        authority.insert(objective.clone());
        authority.insert(method.clone());
        authority.insert(prior_attempt.clone());

        let payload = FailedAttemptPayload::new(
            objective.id(),
            method.id(),
            FailureKind::MethodFailure,
            "acyclic retry".to_owned(),
            vec![],
            Some(prior_attempt.id()),
        )
        .unwrap();

        let candidate = EvidenceRecord::new_failed_attempt(
            identity(229),
            identity(230),
            identity(231),
            vec![],
            payload,
        )
        .unwrap();

        authority.cycle_result = AuthoritativeRelationshipCycle::Acyclic;
        authority.expected_cycle_kind = Some(EvidenceRelationshipKind::Retry);
        authority.expected_cycle_candidate_id = Some(candidate.id());
        authority.expected_cycle_referenced_id = Some(prior_attempt.id());

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn supersession_relationship_cycle_is_rejected() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let decided = generic_record(RecordKind::Claim, 232, identity(232), vec![]);
        let superseded = generic_record(RecordKind::Disposition, 233, identity(233), vec![]);

        authority.insert(decided.clone());
        authority.insert(superseded.clone());

        let payload = DispositionPayload::new(
            decided.id(),
            DispositionDecision::Supersede,
            identity(234),
            identity(235),
            vec![],
            vec![],
            "supersession cycle".to_owned(),
            Some(superseded.id()),
        )
        .unwrap();

        let candidate = EvidenceRecord::new_disposition(
            identity(236),
            decided.subject_id(),
            identity(237),
            vec![],
            payload,
        )
        .unwrap();

        authority.cycle_result = AuthoritativeRelationshipCycle::Cycle;
        authority.expected_cycle_kind = Some(EvidenceRelationshipKind::Supersession);
        authority.expected_cycle_candidate_id = Some(candidate.id());
        authority.expected_cycle_referenced_id = Some(superseded.id());

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RelationshipCycle)
        );
    }

    #[test]
    fn resolution_relationship_cycle_is_rejected() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let (disputed, first_position, second_position, resolution) =
            dispute_resolution_fixture(&mut authority, 82);

        let candidate =
            resolved_dispute_candidate(&disputed, &first_position, &second_position, &resolution);

        authority.dispute_resolution = AuthoritativeDisputeResolution::Valid;
        authority.cycle_result = AuthoritativeRelationshipCycle::Cycle;
        authority.expected_cycle_kind = Some(EvidenceRelationshipKind::Resolution);
        authority.expected_cycle_candidate_id = Some(candidate.id());
        authority.expected_cycle_referenced_id = Some(resolution.id());

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RelationshipCycle)
        );
    }

    #[test]
    fn unavailable_cycle_relationship_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let objective = generic_record(RecordKind::Objective, 238, identity(238), vec![]);
        let method = generic_record(RecordKind::Method, 239, identity(239), vec![]);
        let prior_attempt = generic_record(RecordKind::FailedAttempt, 240, identity(240), vec![]);

        authority.insert(objective.clone());
        authority.insert(method.clone());
        authority.insert(prior_attempt.clone());

        let payload = FailedAttemptPayload::new(
            objective.id(),
            method.id(),
            FailureKind::MethodFailure,
            "cycle authority unavailable".to_owned(),
            vec![],
            Some(prior_attempt.id()),
        )
        .unwrap();

        let candidate = EvidenceRecord::new_failed_attempt(
            identity(241),
            identity(242),
            identity(243),
            vec![],
            payload,
        )
        .unwrap();

        authority.cycle_error =
            Some(EvidenceAdmissionAuthorityError::RequiredRelationalInformationUnavailable);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RequiredRelationalInformationUnavailable)
        );
    }

    #[test]
    fn failed_attempt_without_retry_does_not_query_cycle_relationship() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.forbid_cycle_lookup = true;

        let objective = generic_record(RecordKind::Objective, 244, identity(244), vec![]);
        let method = generic_record(RecordKind::Method, 245, identity(245), vec![]);

        authority.insert(objective.clone());
        authority.insert(method.clone());

        let payload = FailedAttemptPayload::new(
            objective.id(),
            method.id(),
            FailureKind::MethodFailure,
            "no retry".to_owned(),
            vec![],
            None,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_failed_attempt(
            identity(246),
            identity(247),
            identity(248),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn disposition_without_supersedes_does_not_query_cycle_relationship() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.forbid_cycle_lookup = true;

        let decided = generic_record(RecordKind::Claim, 249, identity(249), vec![]);
        authority.insert(decided.clone());

        let candidate = disposition_completeness_candidate(
            &decided,
            identity(250),
            identity(251),
            identity(252),
            vec![],
            vec![],
        );

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn unresolved_dispute_without_resolution_does_not_query_cycle_relationship() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());
        authority.forbid_cycle_lookup = true;

        let disputed = generic_record(RecordKind::Objective, 90, identity(90), vec![]);
        let first_position = generic_record(RecordKind::Claim, 91, identity(91), vec![]);
        let second_position = generic_record(RecordKind::ReviewerFinding, 92, identity(92), vec![]);

        authority.insert(disputed.clone());
        authority.insert(first_position.clone());
        authority.insert(second_position.clone());

        let payload = DisputePayload::new(
            disputed.id(),
            vec![first_position.id(), second_position.id()],
            identity(93),
            DisputeStatus::Open,
            None,
        )
        .unwrap();

        let candidate = EvidenceRecord::new_dispute(
            identity(94),
            disputed.subject_id(),
            identity(95),
            vec![],
            payload,
        )
        .unwrap();

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }
    fn method_a05_candidate(
        objective: &EvidenceRecord,
        budget_reference: Option<RecordId>,
        seed: u8,
    ) -> EvidenceRecord {
        let payload = crate::MethodPayload::new(
            objective.id(),
            "A05 admission method".to_owned(),
            vec![],
            vec![],
            identity(seed.wrapping_add(1)),
            crate::DigestAlgorithm::Sha256,
            [seed; 32],
            budget_reference,
        )
        .unwrap();

        EvidenceRecord::new_method(
            identity(seed.wrapping_add(2)),
            objective.subject_id(),
            identity(seed.wrapping_add(3)),
            vec![],
            payload,
        )
        .unwrap()
    }

    #[test]
    fn active_a05_reference_is_admissible_without_a04_record_resolution() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let objective = generic_record(RecordKind::Objective, 120, identity(120), vec![]);
        authority.insert(objective.clone());

        // Deliberately absent from authority.records: an A05 reference
        // must not silently become an A04 governed-record lookup.
        let future_reference = RecordId::from_bytes([0xA5; 32]);

        let candidate = method_a05_candidate(&objective, Some(future_reference), 121);

        authority.a05_activation = AuthoritativeA05ReferenceActivation::Active;
        authority.expected_a05_reference_id = Some(future_reference);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }

    #[test]
    fn inactive_a05_reference_is_rejected_as_premature() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let objective = generic_record(RecordKind::Objective, 122, identity(122), vec![]);
        authority.insert(objective.clone());

        let future_reference = RecordId::from_bytes([0xA6; 32]);

        let candidate = method_a05_candidate(&objective, Some(future_reference), 123);

        authority.a05_activation = AuthoritativeA05ReferenceActivation::Inactive;
        authority.expected_a05_reference_id = Some(future_reference);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::PrematureA05Reference)
        );
    }

    #[test]
    fn unavailable_a05_activation_fails_closed() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let objective = generic_record(RecordKind::Objective, 124, identity(124), vec![]);
        authority.insert(objective.clone());

        let future_reference = RecordId::from_bytes([0xA7; 32]);

        let candidate = method_a05_candidate(&objective, Some(future_reference), 125);

        authority.expected_a05_reference_id = Some(future_reference);
        authority.a05_activation_error =
            Some(EvidenceAdmissionAuthorityError::RequiredRelationalInformationUnavailable);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Err(EvidenceAdmissionError::RequiredRelationalInformationUnavailable)
        );
    }

    #[test]
    fn absent_budget_reference_does_not_query_a05_activation() {
        let state = TestStateRef(1);
        let mut authority = TestAuthority::new(state.clone());

        let objective = generic_record(RecordKind::Objective, 126, identity(126), vec![]);
        authority.insert(objective.clone());

        authority.forbid_a05_activation_lookup = true;

        let candidate = method_a05_candidate(&objective, None, 127);

        assert_eq!(
            evaluate_admission(&candidate, &authority, &state),
            Ok(EvidenceAdmissionResult::Admissible)
        );
    }
}
