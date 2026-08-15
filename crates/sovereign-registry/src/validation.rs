use crate::{
    Caid, CapabilityPayloadV1, FilesystemReadScopeV1, FilesystemWriteScopeV1, IdentityResolver,
    IssuerOperationalEligibility, IssuerStateResolver, NetworkScopeV1, ObjectClass, RegistryError,
    RegistryGraph, TargetScopeV1,
};

pub fn validate_governed_reference(
    graph: &RegistryGraph,
    artifact_caid: &Caid,
    expected_class: ObjectClass,
) -> Result<(), RegistryError> {
    if let Some(node) = graph.get_versioned_node(artifact_caid) {
        let actual = node.class();

        if actual == expected_class {
            Ok(())
        } else {
            Err(RegistryError::ObjectClassMismatch {
                expected: expected_class,
                actual,
            })
        }
    } else if graph.get_node(artifact_caid).is_some() {
        Err(RegistryError::ObjectClassUnavailable)
    } else {
        Err(RegistryError::UnresolvedReference)
    }
}

pub fn validate_capability_reference(
    graph: &RegistryGraph,
    caid: &Caid,
) -> Result<(), RegistryError> {
    if graph.contains_admitted_reference(caid) {
        Ok(())
    } else {
        Err(RegistryError::UnresolvedCapabilityReference)
    }
}

pub fn validate_capability_references(
    graph: &RegistryGraph,
    capability: &CapabilityPayloadV1,
) -> Result<(), RegistryError> {
    if let TargetScopeV1::ExactObject(caid) = capability.target_scope() {
        validate_capability_reference(graph, caid)?;
    }

    if let Some(caid) = capability.authorized_executable() {
        validate_capability_reference(graph, &caid)?;
    }

    if let Some(scope) = capability.resource_constraints().network() {
        match scope {
            NetworkScopeV1::GovernedService(caid) => {
                validate_capability_reference(graph, caid)?;
            }
        }
    }

    if let Some(scope) = capability.resource_constraints().filesystem_read() {
        match scope {
            FilesystemReadScopeV1::ExactObject(caid)
            | FilesystemReadScopeV1::GovernedNamespace(caid) => {
                validate_capability_reference(graph, caid)?;
            }
        }
    }

    if let Some(scope) = capability.resource_constraints().filesystem_write() {
        match scope {
            FilesystemWriteScopeV1::GovernedNamespace(caid) => {
                validate_capability_reference(graph, caid)?;
            }
        }
    }

    Ok(())
}

pub fn validate_capability_identities<R: IdentityResolver>(
    resolver: &R,
    capability: &CapabilityPayloadV1,
    state_ref: &R::StateRef,
) -> Result<(), RegistryError> {
    resolver.resolve(&capability.issuer_identity(), state_ref)?;
    resolver.resolve(&capability.subject_identity(), state_ref)?;

    Ok(())
}

pub fn validate_capability_temporal(
    capability: &CapabilityPayloadV1,
    admission_context_time: u64,
) -> Result<(), RegistryError> {
    if let Some(expiry) = capability.expiry() {
        if admission_context_time >= expiry {
            return Err(RegistryError::CapabilitySemanticViolation);
        }
    }

    Ok(())
}

pub fn validate_capability_issuer<R: IssuerStateResolver>(
    resolver: &R,
    capability: &CapabilityPayloadV1,
    state_ref: &R::StateRef,
) -> Result<(), RegistryError> {
    let issuer_state = resolver
        .resolve_issuer_state(&capability.issuer_identity(), state_ref)
        .map_err(|_| RegistryError::UnauthorizedCapabilityIssuer)?;

    if issuer_state.operational_eligibility() != IssuerOperationalEligibility::Eligible {
        return Err(RegistryError::UnauthorizedCapabilityIssuer);
    }

    if !issuer_state.has_capability_v1_issuer_authority() {
        return Err(RegistryError::UnauthorizedCapabilityIssuer);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        IdentityId, IdentityKind, IdentityRecord, IdentityStateRef, IssuerStateRef, RegistryNode,
        RegistryNodeType, ResolvedIdentity, ResolvedIssuerState, VersionedRegistryNode,
    };
    use std::cell::RefCell;

    fn insert_v2_node(graph: &mut RegistryGraph, class: ObjectClass, seed: u8) -> Caid {
        let legacy_parent =
            RegistryNode::new(RegistryNodeType::Capability, vec![0xA0, seed], vec![]).unwrap();
        let parent_caid = legacy_parent.caid();
        graph.insert_node(legacy_parent).unwrap();

        let node = VersionedRegistryNode::new(class, vec![parent_caid], vec![seed]).unwrap();
        let caid = node.caid();

        graph.insert_versioned_node(node).unwrap();

        caid
    }

    #[test]
    fn unresolved_caid_fails_closed() {
        let graph = RegistryGraph::default();
        let unknown_caid = Caid([0x40; 32]);

        assert_eq!(
            validate_governed_reference(&graph, &unknown_caid, ObjectClass::Dataset,),
            Err(RegistryError::UnresolvedReference)
        );
    }

    #[test]
    fn matching_object_class_is_admissible() {
        let mut graph = RegistryGraph::default();

        let caid = insert_v2_node(&mut graph, ObjectClass::Dataset, 0x51);

        assert_eq!(
            validate_governed_reference(&graph, &caid, ObjectClass::Dataset,),
            Ok(())
        );
    }

    #[test]
    fn same_width_wrong_object_class_fails_closed() {
        let mut graph = RegistryGraph::default();

        let caid = insert_v2_node(&mut graph, ObjectClass::Policy, 0x61);

        let result = validate_governed_reference(&graph, &caid, ObjectClass::Dataset);

        match result {
            Err(RegistryError::ObjectClassMismatch { expected, actual }) => {
                assert_eq!(expected, ObjectClass::Dataset);
                assert_eq!(actual, ObjectClass::Policy);
            }

            other => panic!("expected ObjectClassMismatch, got {other:?}"),
        }
    }

    #[test]
    fn legacy_node_is_not_silently_assigned_a_v2_object_class() {
        let mut graph = RegistryGraph::default();

        let node = RegistryNode::new(RegistryNodeType::Capability, vec![0x71], vec![]).unwrap();

        let caid = node.caid();

        graph.insert_node(node).unwrap();

        assert_eq!(graph.object_class(&caid), None);
    }

    #[test]
    fn legacy_governed_reference_fails_when_object_class_is_unavailable() {
        let mut graph = RegistryGraph::default();

        let node = RegistryNode::new(RegistryNodeType::Capability, vec![0x72], vec![]).unwrap();

        let caid = node.caid();
        graph.insert_node(node).unwrap();

        assert_eq!(
            validate_governed_reference(&graph, &caid, ObjectClass::Dataset,),
            Err(RegistryError::ObjectClassUnavailable)
        );
    }

    #[test]
    fn capability_reference_accepts_admitted_legacy_v1_object() {
        let mut graph = RegistryGraph::default();

        let node = RegistryNode::new(RegistryNodeType::Capability, vec![0x81], vec![]).unwrap();
        let caid = node.caid();

        graph.insert_node(node).unwrap();

        assert_eq!(validate_capability_reference(&graph, &caid), Ok(()));
    }

    #[test]
    fn capability_reference_accepts_admitted_v2_object() {
        let mut graph = RegistryGraph::default();

        let legacy_parent =
            RegistryNode::new(RegistryNodeType::Capability, vec![0x82], vec![]).unwrap();
        let parent_caid = legacy_parent.caid();
        graph.insert_node(legacy_parent).unwrap();

        let node = VersionedRegistryNode::new(ObjectClass::Dataset, vec![parent_caid], vec![0x83])
            .unwrap();
        let caid = node.caid();

        graph.insert_versioned_node(node).unwrap();

        assert_eq!(validate_capability_reference(&graph, &caid), Ok(()));
    }

    #[test]
    fn capability_reference_rejects_unresolved_caid() {
        let graph = RegistryGraph::default();
        let caid = Caid([0x84; 32]);

        assert_eq!(
            validate_capability_reference(&graph, &caid),
            Err(RegistryError::UnresolvedCapabilityReference)
        );
    }

    #[test]
    fn capability_reference_validation_ignores_named_scope_and_governing_policy() {
        let payload = capability_payload_for_reference_test(
            TargetScopeV1::NamedScope("named-scope".to_owned()),
            None,
            None,
            None,
            None,
        );

        let graph = RegistryGraph::default();

        assert_eq!(validate_capability_references(&graph, &payload), Ok(()));
    }

    #[test]
    fn capability_reference_validation_checks_all_registry_reference_positions() {
        let unresolved = Caid([0x91; 32]);

        let cases = [
            capability_payload_for_reference_test(
                TargetScopeV1::ExactObject(unresolved),
                None,
                None,
                None,
                None,
            ),
            capability_payload_for_reference_test(
                TargetScopeV1::NamedScope("scope".to_owned()),
                Some(unresolved),
                None,
                None,
                None,
            ),
            capability_payload_for_reference_test(
                TargetScopeV1::NamedScope("scope".to_owned()),
                None,
                Some(NetworkScopeV1::GovernedService(unresolved)),
                None,
                None,
            ),
            capability_payload_for_reference_test(
                TargetScopeV1::NamedScope("scope".to_owned()),
                None,
                None,
                Some(FilesystemReadScopeV1::ExactObject(unresolved)),
                None,
            ),
            capability_payload_for_reference_test(
                TargetScopeV1::NamedScope("scope".to_owned()),
                None,
                None,
                Some(FilesystemReadScopeV1::GovernedNamespace(unresolved)),
                None,
            ),
            capability_payload_for_reference_test(
                TargetScopeV1::NamedScope("scope".to_owned()),
                None,
                None,
                None,
                Some(FilesystemWriteScopeV1::GovernedNamespace(unresolved)),
            ),
        ];

        let graph = RegistryGraph::default();

        for payload in cases {
            assert_eq!(
                validate_capability_references(&graph, &payload),
                Err(RegistryError::UnresolvedCapabilityReference)
            );
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

            self.records
                .iter()
                .find(|record| record.id() == *identity_id)
                .and_then(|record| ResolvedIdentity::from_record(identity_id, record))
                .ok_or(RegistryError::IdentityNotFound)
        }
    }

    #[test]
    fn capability_identity_validation_resolves_issuer_then_subject_against_same_state() {
        let issuer = IdentityRecord::new(IdentityKind::Agent, b"gate3b:issuer".to_vec()).unwrap();
        let subject = IdentityRecord::new(IdentityKind::Tool, b"gate3b:subject".to_vec()).unwrap();

        let payload = capability_payload_for_identity_test(issuer.id(), subject.id());
        let resolver = RecordingIdentityResolver::new(vec![issuer.clone(), subject.clone()]);
        let state_ref = TestIdentityStateRef([0xA1, 0x03, 0x0B, 0x01]);

        assert_eq!(
            validate_capability_identities(&resolver, &payload, &state_ref),
            Ok(())
        );

        assert_eq!(
            resolver.calls.borrow().as_slice(),
            &[
                (issuer.id(), state_ref.clone()),
                (subject.id(), state_ref.clone()),
            ]
        );
    }

    #[test]
    fn capability_identity_validation_fails_closed_when_issuer_is_missing() {
        let issuer =
            IdentityRecord::new(IdentityKind::Agent, b"gate3b:missing-issuer".to_vec()).unwrap();
        let subject =
            IdentityRecord::new(IdentityKind::Tool, b"gate3b:subject-present".to_vec()).unwrap();

        let payload = capability_payload_for_identity_test(issuer.id(), subject.id());
        let resolver = RecordingIdentityResolver::new(vec![subject]);
        let state_ref = TestIdentityStateRef([0xA1, 0x03, 0x0B, 0x02]);

        assert_eq!(
            validate_capability_identities(&resolver, &payload, &state_ref),
            Err(RegistryError::IdentityNotFound)
        );

        assert_eq!(resolver.calls.borrow().len(), 1);
        assert_eq!(resolver.calls.borrow()[0].0, issuer.id());
    }

    #[test]
    fn capability_identity_validation_fails_closed_when_subject_is_missing() {
        let issuer =
            IdentityRecord::new(IdentityKind::Agent, b"gate3b:issuer-present".to_vec()).unwrap();
        let subject =
            IdentityRecord::new(IdentityKind::Tool, b"gate3b:missing-subject".to_vec()).unwrap();

        let payload = capability_payload_for_identity_test(issuer.id(), subject.id());
        let resolver = RecordingIdentityResolver::new(vec![issuer.clone()]);
        let state_ref = TestIdentityStateRef([0xA1, 0x03, 0x0B, 0x03]);

        assert_eq!(
            validate_capability_identities(&resolver, &payload, &state_ref),
            Err(RegistryError::IdentityNotFound)
        );

        assert_eq!(
            resolver.calls.borrow().as_slice(),
            &[
                (issuer.id(), state_ref.clone()),
                (subject.id(), state_ref.clone()),
            ]
        );
    }

    #[test]
    fn capability_identity_validation_propagates_state_unavailable() {
        let issuer =
            IdentityRecord::new(IdentityKind::Agent, b"gate3b:issuer-unavailable".to_vec())
                .unwrap();
        let subject =
            IdentityRecord::new(IdentityKind::Tool, b"gate3b:subject-unavailable".to_vec())
                .unwrap();

        let payload = capability_payload_for_identity_test(issuer.id(), subject.id());
        let resolver = RecordingIdentityResolver::unavailable();
        let state_ref = TestIdentityStateRef([0xA1, 0x03, 0x0B, 0x04]);

        assert_eq!(
            validate_capability_identities(&resolver, &payload, &state_ref),
            Err(RegistryError::IdentityStateUnavailable)
        );

        assert_eq!(resolver.calls.borrow().len(), 1);
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

    #[test]
    fn capability_issuer_validation_accepts_eligible_authorized_issuer_and_replays() {
        let issuer = IdentityRecord::new(IdentityKind::Agent, b"gate5:issuer".to_vec()).unwrap();
        let subject = IdentityRecord::new(IdentityKind::Tool, b"gate5:subject".to_vec()).unwrap();
        let payload = capability_payload_for_identity_test(issuer.id(), subject.id());
        let resolver =
            RecordingIssuerStateResolver::resolved(IssuerOperationalEligibility::Eligible, true);
        let state_ref = TestIssuerStateRef([0xA1, 0x05, 0x0B, 0x01]);

        let first = validate_capability_issuer(&resolver, &payload, &state_ref);
        let replay = validate_capability_issuer(&resolver, &payload, &state_ref);

        assert_eq!(first, Ok(()));
        assert_eq!(replay, first);
        assert_eq!(
            resolver.calls.borrow().as_slice(),
            &[
                (issuer.id(), state_ref.clone()),
                (issuer.id(), state_ref.clone()),
            ]
        );
    }

    #[test]
    fn capability_issuer_validation_rejects_ineligible_issuer() {
        let issuer =
            IdentityRecord::new(IdentityKind::Agent, b"gate5:ineligible".to_vec()).unwrap();
        let subject = IdentityRecord::new(IdentityKind::Tool, b"gate5:subject".to_vec()).unwrap();
        let payload = capability_payload_for_identity_test(issuer.id(), subject.id());
        let resolver =
            RecordingIssuerStateResolver::resolved(IssuerOperationalEligibility::Ineligible, true);
        let state_ref = TestIssuerStateRef([0xA1, 0x05, 0x0B, 0x02]);

        assert_eq!(
            validate_capability_issuer(&resolver, &payload, &state_ref),
            Err(RegistryError::UnauthorizedCapabilityIssuer)
        );
    }

    #[test]
    fn capability_issuer_validation_rejects_missing_issuer_authority() {
        let issuer =
            IdentityRecord::new(IdentityKind::Agent, b"gate5:no-authority".to_vec()).unwrap();
        let subject = IdentityRecord::new(IdentityKind::Tool, b"gate5:subject".to_vec()).unwrap();
        let payload = capability_payload_for_identity_test(issuer.id(), subject.id());
        let resolver =
            RecordingIssuerStateResolver::resolved(IssuerOperationalEligibility::Eligible, false);
        let state_ref = TestIssuerStateRef([0xA1, 0x05, 0x0B, 0x03]);

        assert_eq!(
            validate_capability_issuer(&resolver, &payload, &state_ref),
            Err(RegistryError::UnauthorizedCapabilityIssuer)
        );
    }

    #[test]
    fn capability_issuer_validation_rejects_ineligible_unauthorized_issuer() {
        let issuer = IdentityRecord::new(IdentityKind::Agent, b"gate5:neither".to_vec()).unwrap();
        let subject = IdentityRecord::new(IdentityKind::Tool, b"gate5:subject".to_vec()).unwrap();
        let payload = capability_payload_for_identity_test(issuer.id(), subject.id());
        let resolver =
            RecordingIssuerStateResolver::resolved(IssuerOperationalEligibility::Ineligible, false);
        let state_ref = TestIssuerStateRef([0xA1, 0x05, 0x0B, 0x04]);

        assert_eq!(
            validate_capability_issuer(&resolver, &payload, &state_ref),
            Err(RegistryError::UnauthorizedCapabilityIssuer)
        );
    }

    #[test]
    fn capability_issuer_validation_maps_resolver_failure_to_unauthorized_issuer() {
        let issuer =
            IdentityRecord::new(IdentityKind::Agent, b"gate5:unavailable".to_vec()).unwrap();
        let subject = IdentityRecord::new(IdentityKind::Tool, b"gate5:subject".to_vec()).unwrap();
        let payload = capability_payload_for_identity_test(issuer.id(), subject.id());
        let resolver = RecordingIssuerStateResolver::unavailable();
        let state_ref = TestIssuerStateRef([0xA1, 0x05, 0x0B, 0x05]);

        assert_eq!(
            validate_capability_issuer(&resolver, &payload, &state_ref),
            Err(RegistryError::UnauthorizedCapabilityIssuer)
        );
        assert_eq!(
            resolver.calls.borrow().as_slice(),
            &[(issuer.id(), state_ref)]
        );
    }

    #[test]
    fn capability_temporal_validation_accepts_absent_expiry() {
        let payload = capability_payload_for_temporal_test(None);

        assert_eq!(validate_capability_temporal(&payload, u64::MAX), Ok(()));
    }

    #[test]
    fn capability_temporal_validation_accepts_time_before_expiry() {
        let payload = capability_payload_for_temporal_test(Some(100));

        assert_eq!(validate_capability_temporal(&payload, 99), Ok(()));
    }

    #[test]
    fn capability_temporal_validation_rejects_time_equal_to_expiry() {
        let payload = capability_payload_for_temporal_test(Some(100));

        assert_eq!(
            validate_capability_temporal(&payload, 100),
            Err(RegistryError::CapabilitySemanticViolation)
        );
    }

    #[test]
    fn capability_temporal_validation_rejects_time_after_expiry() {
        let payload = capability_payload_for_temporal_test(Some(100));

        assert_eq!(
            validate_capability_temporal(&payload, 101),
            Err(RegistryError::CapabilitySemanticViolation)
        );
    }

    #[test]
    fn capability_temporal_validation_preserves_max_value_and_replay() {
        let payload = capability_payload_for_temporal_test(Some(u64::MAX));

        let first = validate_capability_temporal(&payload, u64::MAX - 1);
        let replay = validate_capability_temporal(&payload, u64::MAX - 1);

        assert_eq!(first, Ok(()));
        assert_eq!(replay, first);
        assert_eq!(
            validate_capability_temporal(&payload, u64::MAX),
            Err(RegistryError::CapabilitySemanticViolation)
        );
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

    fn capability_payload_for_reference_test(
        target_scope: TargetScopeV1,
        authorized_executable: Option<Caid>,
        network: Option<NetworkScopeV1>,
        filesystem_read: Option<FilesystemReadScopeV1>,
        filesystem_write: Option<FilesystemWriteScopeV1>,
    ) -> CapabilityPayloadV1 {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&[0x11; 32]);
        bytes.extend_from_slice(&[0x22; 32]);
        bytes.extend_from_slice(&1_u16.to_be_bytes());

        match target_scope {
            TargetScopeV1::ExactObject(caid) => {
                bytes.push(0x01);
                bytes.extend_from_slice(&caid.0);
            }
            TargetScopeV1::NamedScope(name) => {
                bytes.push(0x02);
                bytes.extend_from_slice(&(name.len() as u16).to_be_bytes());
                bytes.extend_from_slice(name.as_bytes());
            }
        }

        match authorized_executable {
            Some(caid) => {
                bytes.push(0x01);
                bytes.extend_from_slice(&caid.0);
            }
            None => bytes.push(0x00),
        }

        bytes.push(0x01);

        match network {
            Some(NetworkScopeV1::GovernedService(caid)) => {
                bytes.push(0x01);
                bytes.push(0x01);
                bytes.extend_from_slice(&caid.0);
            }
            None => bytes.push(0x00),
        }

        match filesystem_read {
            Some(FilesystemReadScopeV1::ExactObject(caid)) => {
                bytes.push(0x01);
                bytes.push(0x01);
                bytes.extend_from_slice(&caid.0);
            }
            Some(FilesystemReadScopeV1::GovernedNamespace(caid)) => {
                bytes.push(0x01);
                bytes.push(0x02);
                bytes.extend_from_slice(&caid.0);
            }
            None => bytes.push(0x00),
        }

        match filesystem_write {
            Some(FilesystemWriteScopeV1::GovernedNamespace(caid)) => {
                bytes.push(0x01);
                bytes.push(0x01);
                bytes.extend_from_slice(&caid.0);
            }
            None => bytes.push(0x00),
        }

        bytes.push(0x00);
        bytes.push(0x00);

        bytes.extend_from_slice(&[0xFE; 32]);

        CapabilityPayloadV1::decode(&bytes).unwrap()
    }
}
