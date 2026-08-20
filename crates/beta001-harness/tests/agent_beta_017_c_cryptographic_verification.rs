use sovereign_audit::replay::{
    graph::EvidenceGraph,
    model::{EvidenceDigest, EvidenceNode, NodeId, ReplayTimestamp, SchemaVersion},
    verification::verify_ancestry,
    error::ReplayError,
};

fn make_node(id: &str, digest: &str, parent_digest: Option<&str>) -> EvidenceNode {
    EvidenceNode {
        id: NodeId(id.into()),
        record_type: "RECORD".into(),
        schema_version: SchemaVersion("v1".into()),
        digest: EvidenceDigest(digest.into()),
        parent_digest: parent_digest.map(|p| EvidenceDigest(p.into())),
        timestamp: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
        payload: serde_json::json!({}),
    }
}

#[test]
fn o17_c_01_valid_digest_chain_passes() {
    let nodes = vec![
        make_node("prop", "dig_prop", None),
        make_node("life", "dig_life", Some("dig_prop")),
    ];
    let graph = EvidenceGraph::new(nodes);
    let lineage = vec![NodeId("life".into()), NodeId("prop".into())];
    assert!(verify_ancestry(&graph, &lineage).is_ok());
}

#[test]
fn o17_c_02_modified_payload_breaks_chain() {
    let nodes = vec![
        make_node("prop", "dig_prop", None),
        make_node("life", "dig_life", Some("dig_tampered")),
    ];
    let graph = EvidenceGraph::new(nodes);
    let lineage = vec![NodeId("life".into()), NodeId("prop".into())];
    assert_eq!(verify_ancestry(&graph, &lineage), Err(ReplayError::InvalidCryptographicAncestry));
}

#[test]
fn o17_c_03_child_identity_mismatch_behaviour() {
    let nodes = vec![
        make_node("prop", "dig_prop", None),
        make_node("life", "dig_wrong_digest", Some("dig_prop")),
    ];
    let graph = EvidenceGraph::new(nodes);
    let lineage = vec![NodeId("life".into()), NodeId("prop".into())];
    assert!(verify_ancestry(&graph, &lineage).is_ok());
}

#[test]
fn o17_c_04_missing_parent_fails_closed() {
    let nodes = vec![
        make_node("life", "dig_life", Some("dig_missing")),
    ];
    let graph = EvidenceGraph::new(nodes);
    // Evaluating a lineage where the node expects a parent that is absent from the graph / lineage
    let lineage = vec![NodeId("life".into()), NodeId("missing_parent".into())];
    assert!(verify_ancestry(&graph, &lineage).is_err());
}
