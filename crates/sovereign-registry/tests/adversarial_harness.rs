use sovereign_registry::{
    validate_capability_governing_policy, validate_capability_identities,
    validate_capability_issuer, validate_capability_reference, validate_capability_references,
    validate_capability_temporal, Caid, CapabilityPayloadV1, GoverningPolicyAuthority, IdentityId,
    IdentityKind, IdentityRecord, IdentityResolver, IdentityStateRef, IssuerOperationalEligibility,
    IssuerStateRef, IssuerStateResolver, ObjectClass, PolicyAuthorizationOutcome, PolicyStateRef,
    RegistryError, RegistryGraph, ResolvedGoverningPolicy, ResolvedIdentity, ResolvedIssuerState,
    VersionedRegistryNode,
};
use std::cell::RefCell;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum HarnessOutcome {
    Approved,
    Forbidden,
    Unavailable,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ThreatClass {
    SingleAgentCircumvention,
    PrivilegePropagation,
    CollusiveBypass,
    ConfusedDeputy,
    IdentitySubstitution,
    StateReplay,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ActorRole {
    Requester,
    CapabilitySubject,
    Issuer,
    PeerAgent,
    Deputy,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct FixtureId(&'static str);

#[derive(Debug, Clone, PartialEq, Eq)]
struct AdversarialFixture {
    id: FixtureId,
    threat_class: ThreatClass,
    actors: Vec<ActorRole>,
    state_ref: [u8; 4],
    expected: HarnessOutcome,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum HarnessGap {
    UnmappedRegistryError(RegistryError),
}

fn map_validation_result(result: Result<(), RegistryError>) -> Result<HarnessOutcome, HarnessGap> {
    match result {
        Ok(()) => Ok(HarnessOutcome::Approved),

        Err(RegistryError::IdentityStateUnavailable) => Ok(HarnessOutcome::Unavailable),

        Err(
            RegistryError::IdentityNotFound
            | RegistryError::UnresolvedCapabilityReference
            | RegistryError::UnresolvedReference
            | RegistryError::UnauthorizedCapabilityIssuer
            | RegistryError::InvalidGoverningPolicy
            | RegistryError::CapabilitySemanticViolation
            | RegistryError::ObjectClassMismatch { .. }
            | RegistryError::ObjectClassUnavailable
            | RegistryError::MalformedCapabilityPayload,
        ) => Ok(HarnessOutcome::Forbidden),

        Err(error) => Err(HarnessGap::UnmappedRegistryError(error)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestIdentityStateRef([u8; 4]);

impl IdentityStateRef for TestIdentityStateRef {}

struct RecordingIdentityResolver {
    records: Vec<IdentityRecord>,
    calls: RefCell<Vec<(IdentityId, TestIdentityStateRef)>>,
    unavailable: bool,
}

impl RecordingIdentityResolver {
    fn new(records: Vec<IdentityRecord>) -> Self {
        Self {
            records,
            calls: RefCell::new(Vec::new()),
            unavailable: false,
        }
    }

    fn unavailable() -> Self {
        Self {
            records: Vec::new(),
            calls: RefCell::new(Vec::new()),
            unavailable: true,
        }
    }

    fn authoritative_identity_ids(&self) -> Vec<IdentityId> {
        self.records.iter().map(IdentityRecord::id).collect()
    }
}

impl IdentityResolver for RecordingIdentityResolver {
    type StateRef = TestIdentityStateRef;

    fn resolve(
        &self,
        identity_id: &IdentityId,
        state_ref: &Self::StateRef,
    ) -> Result<ResolvedIdentity, RegistryError> {
        self.calls
            .borrow_mut()
            .push((*identity_id, state_ref.clone()));

        if self.unavailable {
            return Err(RegistryError::IdentityStateUnavailable);
        }

        let record = self
            .records
            .iter()
            .find(|record| record.id() == *identity_id)
            .ok_or(RegistryError::IdentityNotFound)?;

        ResolvedIdentity::from_record(identity_id, record).ok_or(RegistryError::IdentityNotFound)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestIssuerStateRef([u8; 4]);

impl IssuerStateRef for TestIssuerStateRef {}

struct RecordingIssuerStateResolver {
    outcome: Result<ResolvedIssuerState, RegistryError>,
    calls: RefCell<Vec<(IdentityId, TestIssuerStateRef)>>,
}

impl RecordingIssuerStateResolver {
    fn resolved(
        eligibility: IssuerOperationalEligibility,
        capability_v1_issuer_authority: bool,
    ) -> Self {
        Self {
            outcome: Ok(ResolvedIssuerState::new(
                eligibility,
                capability_v1_issuer_authority,
            )),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn unavailable() -> Self {
        Self {
            outcome: Err(RegistryError::IdentityStateUnavailable),
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl IssuerStateResolver for RecordingIssuerStateResolver {
    type StateRef = TestIssuerStateRef;

    fn resolve_issuer_state(
        &self,
        issuer_identity: &IdentityId,
        state_ref: &Self::StateRef,
    ) -> Result<ResolvedIssuerState, RegistryError> {
        self.calls
            .borrow_mut()
            .push((*issuer_identity, state_ref.clone()));

        self.outcome
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct TestPolicyStateRef([u8; 4]);

impl PolicyStateRef for TestPolicyStateRef {}

struct RecordingPolicyAuthority {
    resolution: Result<ResolvedGoverningPolicy, RegistryError>,
    evaluation: Result<PolicyAuthorizationOutcome, RegistryError>,
    resolution_calls: RefCell<Vec<(Caid, TestPolicyStateRef)>>,
    evaluation_calls: RefCell<Vec<(Caid, TestPolicyStateRef)>>,
}

impl RecordingPolicyAuthority {
    fn new(
        node: VersionedRegistryNode,
        evaluation: Result<PolicyAuthorizationOutcome, RegistryError>,
    ) -> Self {
        Self {
            resolution: Ok(ResolvedGoverningPolicy::new(node)),
            evaluation,
            resolution_calls: RefCell::new(Vec::new()),
            evaluation_calls: RefCell::new(Vec::new()),
        }
    }

    fn resolution_failure(error: RegistryError) -> Self {
        Self {
            resolution: Err(error),
            evaluation: Ok(PolicyAuthorizationOutcome::Authorized),
            resolution_calls: RefCell::new(Vec::new()),
            evaluation_calls: RefCell::new(Vec::new()),
        }
    }
}

impl GoverningPolicyAuthority for RecordingPolicyAuthority {
    type StateRef = TestPolicyStateRef;

    fn resolve_policy(
        &self,
        governing_policy: &Caid,
        state_ref: &Self::StateRef,
    ) -> Result<ResolvedGoverningPolicy, RegistryError> {
        self.resolution_calls
            .borrow_mut()
            .push((*governing_policy, state_ref.clone()));

        self.resolution.clone()
    }

    fn evaluate_capability(
        &self,
        policy: &ResolvedGoverningPolicy,
        _capability: &CapabilityPayloadV1,
        state_ref: &Self::StateRef,
    ) -> Result<PolicyAuthorizationOutcome, RegistryError> {
        self.evaluation_calls
            .borrow_mut()
            .push((policy.node().caid(), state_ref.clone()));

        self.evaluation
    }
}

fn policy_node(seed: u8) -> VersionedRegistryNode {
    VersionedRegistryNode::new(ObjectClass::Policy, vec![Caid([0xA0; 32])], vec![seed]).unwrap()
}

fn capability_payload_for_policy_test(governing_policy: Caid) -> CapabilityPayloadV1 {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&[0x11; 32]);
    bytes.extend_from_slice(&[0x22; 32]);
    bytes.extend_from_slice(&1_u16.to_be_bytes());

    bytes.push(0x02);
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.push(b'x');

    bytes.push(0x00);

    bytes.push(0x01);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x00);

    bytes.push(0x00);
    bytes.push(0x00);

    bytes.extend_from_slice(&governing_policy.0);

    CapabilityPayloadV1::decode(&bytes).unwrap()
}
fn capability_payload_for_temporal_test(expiry: Option<u64>) -> CapabilityPayloadV1 {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&[0x11; 32]);
    bytes.extend_from_slice(&[0x22; 32]);
    bytes.extend_from_slice(&1_u16.to_be_bytes());

    bytes.push(0x02);
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.push(b'x');

    bytes.push(0x00);

    bytes.push(0x01);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x00);

    bytes.push(0x00);

    match expiry {
        Some(expiry) => {
            bytes.push(0x01);
            bytes.extend_from_slice(&expiry.to_be_bytes());
        }
        None => bytes.push(0x00),
    }

    bytes.extend_from_slice(&[0xFE; 32]);

    CapabilityPayloadV1::decode(&bytes).unwrap()
}
fn capability_payload_for_identity_test(
    issuer_identity: IdentityId,
    subject_identity: IdentityId,
) -> CapabilityPayloadV1 {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(issuer_identity.as_bytes());
    bytes.extend_from_slice(subject_identity.as_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());

    bytes.push(0x02);
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.push(b'x');

    bytes.push(0x00);

    bytes.push(0x01);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x00);

    bytes.push(0x00);
    bytes.push(0x00);

    bytes.extend_from_slice(&[0xFE; 32]);

    CapabilityPayloadV1::decode(&bytes).unwrap()
}

#[test]
fn scaffold_fixture_metadata_is_deterministic() {
    let threat_classes = [
        ThreatClass::SingleAgentCircumvention,
        ThreatClass::PrivilegePropagation,
        ThreatClass::CollusiveBypass,
        ThreatClass::ConfusedDeputy,
        ThreatClass::IdentitySubstitution,
        ThreatClass::StateReplay,
    ];

    let actors = vec![
        ActorRole::Requester,
        ActorRole::CapabilitySubject,
        ActorRole::Issuer,
        ActorRole::PeerAgent,
        ActorRole::Deputy,
    ];

    let fixture = AdversarialFixture {
        id: FixtureId("A05B-ID-001"),
        threat_class: ThreatClass::IdentitySubstitution,
        actors: actors.clone(),
        state_ref: [0xA5, 0xB0, 0x00, 0x01],
        expected: HarnessOutcome::Forbidden,
    };

    let replay = fixture.clone();

    assert_eq!(threat_classes.len(), 6);
    assert_eq!(actors.len(), 5);
    assert_eq!(fixture, replay);
}

#[test]
fn successful_identity_validation_maps_to_approved() {
    let issuer = IdentityRecord::new(IdentityKind::Agent, b"a05b:issuer".to_vec()).unwrap();
    let subject = IdentityRecord::new(IdentityKind::Tool, b"a05b:subject".to_vec()).unwrap();

    let capability = capability_payload_for_identity_test(issuer.id(), subject.id());

    let resolver = RecordingIdentityResolver::new(vec![issuer, subject]);

    let state_ref = TestIdentityStateRef([0xA5, 0xB0, 0x01, 0x01]);

    let result = validate_capability_identities(&resolver, &capability, &state_ref);

    assert_eq!(map_validation_result(result), Ok(HarnessOutcome::Approved));
}

#[test]
fn missing_identity_maps_to_forbidden() {
    let issuer = IdentityRecord::new(IdentityKind::Agent, b"a05b:missing-issuer".to_vec()).unwrap();

    let subject =
        IdentityRecord::new(IdentityKind::Tool, b"a05b:present-subject".to_vec()).unwrap();

    let capability = capability_payload_for_identity_test(issuer.id(), subject.id());

    let resolver = RecordingIdentityResolver::new(vec![subject]);

    let state_ref = TestIdentityStateRef([0xA5, 0xB0, 0x01, 0x02]);

    let result = validate_capability_identities(&resolver, &capability, &state_ref);

    assert_eq!(map_validation_result(result), Ok(HarnessOutcome::Forbidden));
}

#[test]
fn unavailable_identity_state_maps_to_unavailable() {
    let issuer = IdentityRecord::new(IdentityKind::Agent, b"a05b:issuer".to_vec()).unwrap();

    let subject = IdentityRecord::new(IdentityKind::Tool, b"a05b:subject".to_vec()).unwrap();

    let capability = capability_payload_for_identity_test(issuer.id(), subject.id());

    let resolver = RecordingIdentityResolver::unavailable();

    let state_ref = TestIdentityStateRef([0xA5, 0xB0, 0x01, 0x03]);

    let result = validate_capability_identities(&resolver, &capability, &state_ref);

    assert_eq!(
        map_validation_result(result),
        Ok(HarnessOutcome::Unavailable)
    );
}

#[test]
fn unmapped_production_error_is_a_harness_gap() {
    assert_eq!(
        map_validation_result(Err(RegistryError::DuplicateEntity)),
        Err(HarnessGap::UnmappedRegistryError(
            RegistryError::DuplicateEntity
        ))
    );
}

#[test]
fn identity_evaluation_replays_without_authoritative_mutation() {
    let issuer = IdentityRecord::new(IdentityKind::Agent, b"a05b:replay-issuer".to_vec()).unwrap();

    let subject = IdentityRecord::new(IdentityKind::Tool, b"a05b:replay-subject".to_vec()).unwrap();

    let capability = capability_payload_for_identity_test(issuer.id(), subject.id());

    let capability_before = capability.clone();

    let resolver = RecordingIdentityResolver::new(vec![issuer.clone(), subject.clone()]);

    let authoritative_before = resolver.authoritative_identity_ids();

    let state_ref = TestIdentityStateRef([0xA5, 0xB0, 0x01, 0x04]);

    let first = map_validation_result(validate_capability_identities(
        &resolver,
        &capability,
        &state_ref,
    ));

    let replay = map_validation_result(validate_capability_identities(
        &resolver,
        &capability,
        &state_ref,
    ));

    assert_eq!(first, Ok(HarnessOutcome::Approved));
    assert_eq!(replay, first);

    assert_eq!(capability, capability_before);
    assert_eq!(resolver.authoritative_identity_ids(), authoritative_before);

    let calls = resolver.calls.borrow();

    assert_eq!(
        calls.as_slice(),
        &[
            (issuer.id(), state_ref.clone()),
            (subject.id(), state_ref.clone()),
            (issuer.id(), state_ref.clone()),
            (subject.id(), state_ref),
        ]
    );
}

#[test]
fn authorized_issuer_maps_to_approved() {
    let issuer =
        IdentityRecord::new(IdentityKind::Agent, b"a05b:issuer-approved".to_vec()).unwrap();
    let subject = IdentityRecord::new(IdentityKind::Tool, b"a05b:issuer-subject".to_vec()).unwrap();

    let capability = capability_payload_for_identity_test(issuer.id(), subject.id());

    let resolver =
        RecordingIssuerStateResolver::resolved(IssuerOperationalEligibility::Eligible, true);

    let state_ref = TestIssuerStateRef([0xA5, 0xB0, 0x02, 0x01]);

    let result = validate_capability_issuer(&resolver, &capability, &state_ref);

    assert_eq!(map_validation_result(result), Ok(HarnessOutcome::Approved));

    assert_eq!(
        resolver.calls.borrow().as_slice(),
        &[(issuer.id(), state_ref)]
    );
}

#[test]
fn unauthorized_issuer_maps_to_forbidden() {
    let issuer =
        IdentityRecord::new(IdentityKind::Agent, b"a05b:issuer-forbidden".to_vec()).unwrap();
    let subject = IdentityRecord::new(IdentityKind::Tool, b"a05b:issuer-subject".to_vec()).unwrap();

    let capability = capability_payload_for_identity_test(issuer.id(), subject.id());

    let resolver =
        RecordingIssuerStateResolver::resolved(IssuerOperationalEligibility::Eligible, false);

    let state_ref = TestIssuerStateRef([0xA5, 0xB0, 0x02, 0x02]);

    let result = validate_capability_issuer(&resolver, &capability, &state_ref);

    assert_eq!(map_validation_result(result), Ok(HarnessOutcome::Forbidden));
}

#[test]
fn unavailable_issuer_state_remains_forbidden_at_public_validator_boundary() {
    let issuer =
        IdentityRecord::new(IdentityKind::Agent, b"a05b:issuer-unavailable".to_vec()).unwrap();
    let subject = IdentityRecord::new(IdentityKind::Tool, b"a05b:issuer-subject".to_vec()).unwrap();

    let capability = capability_payload_for_identity_test(issuer.id(), subject.id());

    let resolver = RecordingIssuerStateResolver::unavailable();

    let state_ref = TestIssuerStateRef([0xA5, 0xB0, 0x02, 0x03]);

    let production_result = validate_capability_issuer(&resolver, &capability, &state_ref);

    assert_eq!(
        production_result,
        Err(RegistryError::UnauthorizedCapabilityIssuer)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );

    assert_eq!(
        resolver.calls.borrow().as_slice(),
        &[(issuer.id(), state_ref)]
    );
}
#[test]
fn authorized_governing_policy_maps_to_approved() {
    let node = policy_node(0xC1);
    let governing_policy = node.caid();
    let capability = capability_payload_for_policy_test(governing_policy);

    let authority = RecordingPolicyAuthority::new(node, Ok(PolicyAuthorizationOutcome::Authorized));

    let state_ref = TestPolicyStateRef([0xA5, 0xB0, 0x03, 0x01]);

    let result = validate_capability_governing_policy(&authority, &capability, &state_ref);

    assert_eq!(map_validation_result(result), Ok(HarnessOutcome::Approved));

    assert_eq!(
        authority.resolution_calls.borrow().as_slice(),
        &[(governing_policy, state_ref.clone())]
    );

    assert_eq!(
        authority.evaluation_calls.borrow().as_slice(),
        &[(governing_policy, state_ref)]
    );
}

#[test]
fn policy_not_authorized_maps_to_forbidden() {
    let node = policy_node(0xC2);
    let capability = capability_payload_for_policy_test(node.caid());

    let authority =
        RecordingPolicyAuthority::new(node, Ok(PolicyAuthorizationOutcome::NotAuthorized));

    let state_ref = TestPolicyStateRef([0xA5, 0xB0, 0x03, 0x02]);

    let production_result =
        validate_capability_governing_policy(&authority, &capability, &state_ref);

    assert_eq!(
        production_result,
        Err(RegistryError::InvalidGoverningPolicy)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );
}

#[test]
fn unavailable_policy_resolution_remains_forbidden_at_public_validator_boundary() {
    let governing_policy = Caid([0xC3; 32]);
    let capability = capability_payload_for_policy_test(governing_policy);

    let authority =
        RecordingPolicyAuthority::resolution_failure(RegistryError::IdentityStateUnavailable);

    let state_ref = TestPolicyStateRef([0xA5, 0xB0, 0x03, 0x03]);

    let production_result =
        validate_capability_governing_policy(&authority, &capability, &state_ref);

    assert_eq!(
        production_result,
        Err(RegistryError::InvalidGoverningPolicy)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );

    assert!(authority.evaluation_calls.borrow().is_empty());
}
#[test]
fn temporal_validation_before_expiry_maps_to_approved() {
    let capability = capability_payload_for_temporal_test(Some(100));

    let result = validate_capability_temporal(&capability, 99);

    assert_eq!(map_validation_result(result), Ok(HarnessOutcome::Approved));
}

#[test]
fn temporal_validation_at_expiry_maps_to_forbidden() {
    let capability = capability_payload_for_temporal_test(Some(100));

    let production_result = validate_capability_temporal(&capability, 100);

    assert_eq!(
        production_result,
        Err(RegistryError::CapabilitySemanticViolation)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );
}

#[test]
fn temporal_validation_replays_deterministically_without_mutation() {
    let capability = capability_payload_for_temporal_test(Some(u64::MAX));
    let capability_before = capability.clone();

    let first = map_validation_result(validate_capability_temporal(&capability, u64::MAX - 1));

    let replay = map_validation_result(validate_capability_temporal(&capability, u64::MAX - 1));

    assert_eq!(first, Ok(HarnessOutcome::Approved));
    assert_eq!(replay, first);
    assert_eq!(capability, capability_before);
}
#[test]
fn unresolved_capability_reference_maps_to_forbidden() {
    let graph = RegistryGraph::default();
    let unresolved = Caid([0xD1; 32]);

    let production_result = validate_capability_reference(&graph, &unresolved);

    assert_eq!(
        production_result,
        Err(RegistryError::UnresolvedCapabilityReference)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );
}

#[test]
fn named_scope_without_governed_references_maps_to_approved() {
    let graph = RegistryGraph::default();

    let issuer = IdentityRecord::new(IdentityKind::Agent, b"a05b:ref-issuer".to_vec()).unwrap();
    let subject = IdentityRecord::new(IdentityKind::Tool, b"a05b:ref-subject".to_vec()).unwrap();

    let capability = capability_payload_for_identity_test(issuer.id(), subject.id());

    let production_result = validate_capability_references(&graph, &capability);

    assert_eq!(production_result, Ok(()));

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Approved)
    );
}

#[test]
fn unresolved_exact_object_reference_maps_to_forbidden() {
    let graph = RegistryGraph::default();
    let unresolved = Caid([0xD2; 32]);

    let mut bytes = Vec::new();

    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&[0x11; 32]);
    bytes.extend_from_slice(&[0x22; 32]);
    bytes.extend_from_slice(&1_u16.to_be_bytes());

    bytes.push(0x01);
    bytes.extend_from_slice(&unresolved.0);

    bytes.push(0x00);

    bytes.push(0x01);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x00);

    bytes.push(0x00);
    bytes.push(0x00);

    bytes.extend_from_slice(&[0xFE; 32]);

    let capability = CapabilityPayloadV1::decode(&bytes).unwrap();

    let production_result = validate_capability_references(&graph, &capability);

    assert_eq!(
        production_result,
        Err(RegistryError::UnresolvedCapabilityReference)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );
}

#[test]
fn reference_validation_replays_without_graph_mutation() {
    let graph = RegistryGraph::default();
    let unresolved = Caid([0xD3; 32]);

    let first = map_validation_result(validate_capability_reference(&graph, &unresolved));

    let replay = map_validation_result(validate_capability_reference(&graph, &unresolved));

    assert_eq!(first, Ok(HarnessOutcome::Forbidden));
    assert_eq!(replay, first);
}

#[test]
fn a05b_circ_003_create_exact_object_is_forbidden() {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&[0x11; 32]);
    bytes.extend_from_slice(&[0x22; 32]);

    // OperationCodeV1::Create
    bytes.extend_from_slice(&2_u16.to_be_bytes());

    // TargetScopeV1::ExactObject
    bytes.push(0x01);
    bytes.extend_from_slice(&[0x44; 32]);

    // No authorized executable.
    bytes.push(0x00);

    // ResourceConstraintsV1 with no granted resources.
    bytes.push(0x01);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x00);

    // No execution budget.
    bytes.push(0x00);

    // No expiry.
    bytes.push(0x00);

    bytes.extend_from_slice(&[0xFE; 32]);

    let capability = CapabilityPayloadV1::decode(&bytes).unwrap();

    let production_result = capability.validate_internal_coherence();

    assert_eq!(
        production_result,
        Err(RegistryError::CapabilitySemanticViolation)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );
}

#[test]
fn a05b_circ_004_executable_on_non_execute_operation_is_forbidden() {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&[0x11; 32]);
    bytes.extend_from_slice(&[0x22; 32]);

    // OperationCodeV1::Read
    bytes.extend_from_slice(&1_u16.to_be_bytes());

    // TargetScopeV1::NamedScope("x")
    bytes.push(0x02);
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.push(b'x');

    // An authorized executable is present despite a non-Execute operation.
    bytes.push(0x01);
    bytes.extend_from_slice(&[0x44; 32]);

    bytes.push(0x01);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x00);

    bytes.push(0x00);
    bytes.push(0x00);

    bytes.extend_from_slice(&[0xFE; 32]);

    let capability = CapabilityPayloadV1::decode(&bytes).unwrap();

    let production_result = capability.validate_internal_coherence();

    assert_eq!(
        production_result,
        Err(RegistryError::CapabilitySemanticViolation)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );
}

#[test]
fn a05b_circ_005_network_budget_without_network_scope_is_forbidden() {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&[0x11; 32]);
    bytes.extend_from_slice(&[0x22; 32]);

    // OperationCodeV1::Read
    bytes.extend_from_slice(&1_u16.to_be_bytes());

    bytes.push(0x02);
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.push(b'x');

    bytes.push(0x00);

    // No network/read/write scopes.
    bytes.push(0x01);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x00);

    // Execution budget present.
    bytes.push(0x01);
    bytes.push(0x01);
    bytes.extend_from_slice(&0_u64.to_be_bytes());
    bytes.extend_from_slice(&0_u64.to_be_bytes());

    // Network egress budget > 0 while network scope is absent.
    bytes.extend_from_slice(&1_u64.to_be_bytes());

    bytes.extend_from_slice(&0_u64.to_be_bytes());

    bytes.push(0x00);

    bytes.extend_from_slice(&[0xFE; 32]);

    let capability = CapabilityPayloadV1::decode(&bytes).unwrap();

    let production_result = capability.validate_internal_coherence();

    assert_eq!(
        production_result,
        Err(RegistryError::CapabilitySemanticViolation)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );
}

#[test]
fn a05b_circ_006_filesystem_write_budget_without_write_scope_is_forbidden() {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&[0x11; 32]);
    bytes.extend_from_slice(&[0x22; 32]);

    // OperationCodeV1::Read
    bytes.extend_from_slice(&1_u16.to_be_bytes());

    bytes.push(0x02);
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.push(b'x');

    bytes.push(0x00);

    // No network/read/write scopes.
    bytes.push(0x01);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x00);

    // Execution budget present.
    bytes.push(0x01);
    bytes.push(0x01);
    bytes.extend_from_slice(&0_u64.to_be_bytes());
    bytes.extend_from_slice(&0_u64.to_be_bytes());
    bytes.extend_from_slice(&0_u64.to_be_bytes());

    // Filesystem-write budget > 0 while write scope is absent.
    bytes.extend_from_slice(&1_u64.to_be_bytes());

    bytes.push(0x00);

    bytes.extend_from_slice(&[0xFE; 32]);

    let capability = CapabilityPayloadV1::decode(&bytes).unwrap();

    let production_result = capability.validate_internal_coherence();

    assert_eq!(
        production_result,
        Err(RegistryError::CapabilitySemanticViolation)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );
}

#[test]
fn a05b_circ_001_expiry_at_boundary_is_forbidden() {
    const EXPIRY: u64 = 1_000;
    const ADMISSION_CONTEXT_TIME: u64 = 1_000;

    let capability = capability_payload_for_temporal_test(Some(EXPIRY));
    let capability_before = capability.clone();

    let production_result = validate_capability_temporal(&capability, ADMISSION_CONTEXT_TIME);

    assert_eq!(
        production_result,
        Err(RegistryError::CapabilitySemanticViolation)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );

    assert_eq!(capability, capability_before);
}

#[test]
fn a05b_circ_002_expiry_after_boundary_is_forbidden() {
    const EXPIRY: u64 = 1_000;
    const ADMISSION_CONTEXT_TIME: u64 = 1_001;

    let capability = capability_payload_for_temporal_test(Some(EXPIRY));
    let capability_before = capability.clone();

    let first = validate_capability_temporal(&capability, ADMISSION_CONTEXT_TIME);

    let replay = validate_capability_temporal(&capability, ADMISSION_CONTEXT_TIME);

    assert_eq!(first, Err(RegistryError::CapabilitySemanticViolation));

    assert_eq!(replay, first);

    assert_eq!(map_validation_result(first), Ok(HarnessOutcome::Forbidden));

    assert_eq!(capability, capability_before);
}

#[test]
fn a05b_circ_007_nonexistent_subject_identity_is_forbidden() {
    let issuer =
        IdentityRecord::new(IdentityKind::Agent, b"a05b:circ-007:issuer".to_vec()).unwrap();

    let absent_subject =
        IdentityRecord::new(IdentityKind::Tool, b"a05b:circ-007:absent-subject".to_vec()).unwrap();

    let capability = capability_payload_for_identity_test(issuer.id(), absent_subject.id());

    let capability_before = capability.clone();

    // The authoritative state contains the declared issuer but not the
    // capability's declared subject identity.
    let resolver = RecordingIdentityResolver::new(vec![issuer.clone()]);
    let authoritative_before = resolver.authoritative_identity_ids();

    let state_ref = TestIdentityStateRef([0xA5, 0xB0, 0x04, 0x07]);

    let production_result = validate_capability_identities(&resolver, &capability, &state_ref);

    assert_eq!(production_result, Err(RegistryError::IdentityNotFound));

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );

    assert_eq!(capability, capability_before);

    assert_eq!(resolver.authoritative_identity_ids(), authoritative_before);

    assert_eq!(
        resolver.calls.borrow().as_slice(),
        &[
            (issuer.id(), state_ref.clone()),
            (absent_subject.id(), state_ref),
        ]
    );
}

#[test]
fn a05b_circ_008_unresolved_exact_target_is_forbidden() {
    let graph = RegistryGraph::default();
    let unresolved_target = Caid([0xD8; 32]);

    let mut bytes = Vec::new();

    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&[0x11; 32]);
    bytes.extend_from_slice(&[0x22; 32]);

    // OperationCodeV1::Read
    bytes.extend_from_slice(&1_u16.to_be_bytes());

    // TargetScopeV1::ExactObject with an unresolved governed CAID.
    bytes.push(0x01);
    bytes.extend_from_slice(&unresolved_target.0);

    // No authorized executable.
    bytes.push(0x00);

    // No governed resource references.
    bytes.push(0x01);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x00);

    // No execution budget.
    bytes.push(0x00);

    // No expiry.
    bytes.push(0x00);

    bytes.extend_from_slice(&[0xFE; 32]);

    let capability = CapabilityPayloadV1::decode(&bytes).unwrap();
    let capability_before = capability.clone();

    let production_result = validate_capability_references(&graph, &capability);

    assert_eq!(
        production_result,
        Err(RegistryError::UnresolvedCapabilityReference)
    );

    assert_eq!(
        map_validation_result(production_result),
        Ok(HarnessOutcome::Forbidden)
    );

    assert_eq!(capability, capability_before);
}
