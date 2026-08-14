use sovereign_registry::{ObjectClass, RegistryError, RegistryGraph, VersionedRegistryNode};

#[test]
fn ordinary_v2_zero_parent_fails_missing_provenance() {
    let mut graph = RegistryGraph::new();

    let node =
        VersionedRegistryNode::new(ObjectClass::Workflow, vec![], b"ordinary-v2-node".to_vec())
            .unwrap();

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::MissingProvenance)
    );
}

#[test]
fn genesis_with_parent_fails_invalid_genesis_provenance() {
    let mut graph = RegistryGraph::new();

    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![sovereign_registry::Caid([0xAA; 32])],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x00],
    )
    .unwrap();

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::InvalidGenesisProvenance)
    );
}

#[test]
fn wrong_authorized_genesis_caid_fails_closed() {
    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x00],
    )
    .unwrap();

    let mut expected_bytes = node.caid().0;
    expected_bytes[0] ^= 0xFF;

    let config =
        sovereign_registry::RegistryBootstrapConfig::new(sovereign_registry::Caid(expected_bytes));

    let mut graph = RegistryGraph::new_v2_bootstrap(config);

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::UnauthorizedGenesis)
    );
}

#[test]
fn genesis_invalid_root_policy_marker_fails_malformed_payload() {
    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x02],
    )
    .unwrap();

    let config = sovereign_registry::RegistryBootstrapConfig::new(node.caid());
    let mut graph = RegistryGraph::new_v2_bootstrap(config);

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::MalformedGenesisPayload)
    );
}

#[test]
fn genesis_trailing_byte_fails_malformed_payload() {
    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x00, 0xAA],
    )
    .unwrap();

    let config = sovereign_registry::RegistryBootstrapConfig::new(node.caid());
    let mut graph = RegistryGraph::new_v2_bootstrap(config);

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::MalformedGenesisPayload)
    );
}

#[test]
fn genesis_invalid_utf8_fails_malformed_payload() {
    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x01, 0xFF, 0x00, 0x02, 0x00],
    )
    .unwrap();

    let config = sovereign_registry::RegistryBootstrapConfig::new(node.caid());
    let mut graph = RegistryGraph::new_v2_bootstrap(config);

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::MalformedGenesisPayload)
    );
}

#[test]
fn genesis_wrong_protocol_version_fails_malformed_payload() {
    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x03, 0x00],
    )
    .unwrap();

    let config = sovereign_registry::RegistryBootstrapConfig::new(node.caid());
    let mut graph = RegistryGraph::new_v2_bootstrap(config);

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::MalformedGenesisPayload)
    );
}

#[test]
fn genesis_truncated_payload_fails_malformed_payload() {
    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02],
    )
    .unwrap();

    let config = sovereign_registry::RegistryBootstrapConfig::new(node.caid());
    let mut graph = RegistryGraph::new_v2_bootstrap(config);

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::MalformedGenesisPayload)
    );
}

#[test]
fn canonical_genesis_payload_round_trip_is_byte_identical() {
    let encoded = vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x00];

    let payload = sovereign_registry::RegistryGenesisPayloadV1::decode(&encoded).unwrap();

    assert_eq!(payload.environment_id(), b"test");
    assert_eq!(payload.protocol_version(), 0x0002);
    assert_eq!(payload.root_policy_caid(), None);
    assert_eq!(payload.encode(), encoded);
}

#[test]
fn legacy_v1_zero_parent_remains_admissible() {
    let mut graph = RegistryGraph::new();

    let node = sovereign_registry::RegistryNode::new(
        sovereign_registry::RegistryNodeType::Capability,
        b"historical-v1-root".to_vec(),
        vec![],
    )
    .unwrap();

    assert_eq!(graph.insert_node(node), Ok(()));
}

#[test]
fn authorized_genesis_bootstraps_empty_v2_graph() {
    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x00],
    )
    .unwrap();

    let caid = node.caid();
    let config = sovereign_registry::RegistryBootstrapConfig::new(caid);
    let mut graph = RegistryGraph::new_v2_bootstrap(config);

    assert_eq!(graph.insert_versioned_node(node), Ok(()));
    assert_eq!(
        graph.object_class(&caid),
        Some(ObjectClass::RegistryGenesis)
    );
}

#[test]
fn established_genesis_cannot_be_reestablished() {
    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x00],
    )
    .unwrap();

    let config = sovereign_registry::RegistryBootstrapConfig::new(node.caid());
    let mut graph = RegistryGraph::new_v2_bootstrap(config);

    assert_eq!(graph.insert_versioned_node(node.clone()), Ok(()));

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::GenesisAlreadyEstablished)
    );
}

#[test]
fn genesis_cannot_be_added_to_populated_graph() {
    let genesis = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x00],
    )
    .unwrap();

    let config = sovereign_registry::RegistryBootstrapConfig::new(genesis.caid());
    let mut graph = RegistryGraph::new_v2_bootstrap(config);

    let legacy = sovereign_registry::RegistryNode::new(
        sovereign_registry::RegistryNodeType::Capability,
        b"historical-populated-root".to_vec(),
        vec![],
    )
    .unwrap();

    assert_eq!(graph.insert_node(legacy), Ok(()));

    assert_eq!(
        graph.insert_versioned_node(genesis),
        Err(RegistryError::GenesisNotPermittedInExistingGraph)
    );
}

#[test]
fn genesis_without_bootstrap_configuration_fails_closed() {
    let node = VersionedRegistryNode::new(
        ObjectClass::RegistryGenesis,
        vec![],
        vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x00],
    )
    .unwrap();

    let mut graph = RegistryGraph::new();

    assert_eq!(
        graph.insert_versioned_node(node),
        Err(RegistryError::UnauthorizedGenesis)
    );
}

#[test]
fn canonical_genesis_payload_with_root_policy_round_trip_is_byte_identical() {
    let mut encoded = vec![0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x02, 0x01];
    encoded.extend_from_slice(&[0xAA; 32]);

    let payload = sovereign_registry::RegistryGenesisPayloadV1::decode(&encoded).unwrap();

    assert_eq!(payload.environment_id(), b"test");
    assert_eq!(payload.protocol_version(), 0x0002);
    assert_eq!(
        payload.root_policy_caid(),
        Some(sovereign_registry::Caid([0xAA; 32]))
    );
    assert_eq!(payload.encode(), encoded);
}
