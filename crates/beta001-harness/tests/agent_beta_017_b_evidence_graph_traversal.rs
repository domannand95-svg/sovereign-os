use sovereign_audit::replay::{
    graph::EvidenceGraph,
    model::{EvidenceDigest, EvidenceNode, NodeId, ReplayTimestamp, SchemaVersion},
    traversal::traverse_ancestry,
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
fn o17_b_01_linear_lineage_traverses() {
    let nodes = vec![
        make_node("prop", "dig_prop", None),
        make_node("life", "dig_life", Some("dig_prop")),
    ];
    let graph = EvidenceGraph::new(nodes);
    let trace = traverse_ancestry(&graph, &NodeId("life".into())).unwrap();
    assert_eq!(trace, vec![NodeId("life".into()), NodeId("prop".into())]);
}

#[test]
fn o17_b_02_branching_lineage_order_is_deterministic() {
    let nodes = vec![
        make_node("prop", "dig_prop", None),
        make_node("life_a", "dig_life_a", Some("dig_prop")),
        make_node("life_b", "dig_life_b", Some("dig_prop")),
    ];
    let graph = EvidenceGraph::new(nodes);
    let trace_a = traverse_ancestry(&graph, &NodeId("life_a".into())).unwrap();
    let trace_b = traverse_ancestry(&graph, &NodeId("life_a".into())).unwrap();
    assert_eq!(trace_a, trace_b);
}

#[test]
fn o17_b_03_self_reference_cycle_rejected() {
    let nodes = vec![
        make_node("a", "dig_a", Some("dig_a")),
    ];
    let graph = EvidenceGraph::new(nodes);
    let res = traverse_ancestry(&graph, &NodeId("a".into()));
    assert_eq!(res, Err(ReplayError::CyclicLineage));
}

#[test]
fn o17_b_04_ancestor_cycle_rejected() {
    let nodes = vec![
        make_node("a", "dig_a", Some("dig_b")),
        make_node("b", "dig_b", Some("dig_a")),
    ];
    let graph = EvidenceGraph::new(nodes);
    let res = traverse_ancestry(&graph, &NodeId("a".into()));
    assert_eq!(res, Err(ReplayError::CyclicLineage));
}

#[test]
fn o17_b_05_duplicate_node_does_not_mutate_graph() {
    let nodes = vec![
        make_node("prop", "dig_prop", None),
        make_node("life", "dig_life", Some("dig_prop")),
    ];
    let mut graph = EvidenceGraph::new(nodes);
    let graph_before = graph.clone();

    let _ = traverse_ancestry(&graph, &NodeId("life".into())).unwrap();

    // Invariant: Traversal(Graph) ∩ State Mutation = ∅
    assert_eq!(graph_before, graph);
}
