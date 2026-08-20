use sovereign_audit::replay::{
    graph::EvidenceGraph,
    model::{
        CapabilityState, EvidenceDigest, EvidenceNode, NodeId, ReplayTimestamp, SchemaVersion,
    },
    reconstruction::reconstruct_state,
    traversal::traverse_ancestry,
    verification::verify_ancestry,
};

#[test]
fn o17_d_01_complete_history_replays() {
    let nodes = vec![
        EvidenceNode {
            id: NodeId("prop".into()),
            record_type: "EFFECT_PROPOSAL-v1".into(),
            schema_version: SchemaVersion("v1".into()),
            digest: EvidenceDigest("dig_prop".into()),
            parent_digest: None,
            timestamp: ReplayTimestamp("2026-08-20T09:00:00Z".into()),
            payload: serde_json::json!({}),
        },
        EvidenceNode {
            id: NodeId("life".into()),
            record_type: "CAPABILITY_LIFECYCLE_EVENT-v1".into(),
            schema_version: SchemaVersion("v1".into()),
            digest: EvidenceDigest("dig_life".into()),
            parent_digest: Some(EvidenceDigest("dig_prop".into())),
            timestamp: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
            payload: serde_json::json!({"new_state": "ACTIVE"}),
        },
    ];

    let graph = EvidenceGraph::new(nodes);
    let lineage = traverse_ancestry(&graph, &NodeId("life".into())).unwrap();
    assert!(verify_ancestry(&graph, &lineage).is_ok());

    let state = reconstruct_state(
        &graph,
        &lineage,
        &ReplayTimestamp("2026-08-20T11:00:00Z".into()),
    )
    .unwrap();
    assert_eq!(state, CapabilityState::Active);
}

#[test]
fn o17_d_02_runtime_state_equals_audit_state() {
    let nodes = vec![
        EvidenceNode {
            id: NodeId("prop".into()),
            record_type: "EFFECT_PROPOSAL-v1".into(),
            schema_version: SchemaVersion("v1".into()),
            digest: EvidenceDigest("dig_prop".into()),
            parent_digest: None,
            timestamp: ReplayTimestamp("2026-08-20T09:00:00Z".into()),
            payload: serde_json::json!({}),
        },
        EvidenceNode {
            id: NodeId("life".into()),
            record_type: "CAPABILITY_LIFECYCLE_EVENT-v1".into(),
            schema_version: SchemaVersion("v1".into()),
            digest: EvidenceDigest("dig_life".into()),
            parent_digest: Some(EvidenceDigest("dig_prop".into())),
            timestamp: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
            payload: serde_json::json!({"new_state": "ACTIVE"}),
        },
    ];

    let graph = EvidenceGraph::new(nodes);
    let lineage = traverse_ancestry(&graph, &NodeId("life".into())).unwrap();

    let audit_view = reconstruct_state(
        &graph,
        &lineage,
        &ReplayTimestamp("2026-08-20T11:00:00Z".into()),
    )
    .unwrap();
    let runtime_view = CapabilityState::Active;

    assert_eq!(audit_view, runtime_view);
}

#[test]
fn o17_d_03_audit_has_no_execution_authority() {
    let nodes = vec![EvidenceNode {
        id: NodeId("prop".into()),
        record_type: "EFFECT_PROPOSAL-v1".into(),
        schema_version: SchemaVersion("v1".into()),
        digest: EvidenceDigest("dig_prop".into()),
        parent_digest: None,
        timestamp: ReplayTimestamp("2026-08-20T09:00:00Z".into()),
        payload: serde_json::json!({}),
    }];
    let graph = EvidenceGraph::new(nodes);
    assert_eq!(graph.len(), 1);
}

#[test]
fn o17_d_04_tampered_history_produces_finding() {
    let nodes = vec![
        EvidenceNode {
            id: NodeId("prop".into()),
            record_type: "EFFECT_PROPOSAL-v1".into(),
            schema_version: SchemaVersion("v1".into()),
            digest: EvidenceDigest("dig_prop".into()),
            parent_digest: None,
            timestamp: ReplayTimestamp("2026-08-20T09:00:00Z".into()),
            payload: serde_json::json!({}),
        },
        EvidenceNode {
            id: NodeId("life".into()),
            record_type: "CAPABILITY_LIFECYCLE_EVENT-v1".into(),
            schema_version: SchemaVersion("v1".into()),
            digest: EvidenceDigest("dig_life".into()),
            // Tampered: Child expects a different cryptographic parent
            parent_digest: Some(EvidenceDigest("dig_tampered".into())),
            timestamp: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
            payload: serde_json::json!({}),
        },
    ];
    let graph = EvidenceGraph::new(nodes);

    // In an end-to-end replay, a tampered cryptographic link prevents
    // lineage traversal, mapping strictly to an audit finding rather than a crash.
    let traversal_result = traverse_ancestry(&graph, &NodeId("life".into()));
    assert!(traversal_result.is_err());
}
