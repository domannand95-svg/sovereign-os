use sovereign_audit::replay::{
    graph::EvidenceGraph,
    model::{CapabilityState, EvidenceDigest, EvidenceNode, NodeId, ReplayTimestamp, SchemaVersion},
    reconstruction::reconstruct_state,
    error::ReplayError,
};

fn make_node(
    id: &str,
    record_type: &str,
    digest: &str,
    parent_digest: Option<&str>,
    timestamp: &str,
    payload: serde_json::Value,
) -> EvidenceNode {
    EvidenceNode {
        id: NodeId(id.into()),
        record_type: record_type.into(),
        schema_version: SchemaVersion("v1".into()),
        digest: EvidenceDigest(digest.into()),
        parent_digest: parent_digest.map(|p| EvidenceDigest(p.into())),
        timestamp: ReplayTimestamp(timestamp.into()),
        payload,
    }
}

#[test]
fn o17_a_01_valid_history_reconstructs_state() {
    let nodes = vec![
        make_node("prop", "EFFECT_PROPOSAL-v1", "dig_prop", None, "2026-08-20T09:00:00Z", serde_json::json!({})),
        make_node("life", "CAPABILITY_LIFECYCLE_EVENT-v1", "dig_life", Some("dig_prop"), "2026-08-20T10:00:00Z", serde_json::json!({"new_state": "ACTIVE"})),
    ];
    let graph = EvidenceGraph::new(nodes);
    let lineage = vec![NodeId("life".into()), NodeId("prop".into())];
    let t = ReplayTimestamp("2026-08-20T11:00:00Z".into());

    let state = reconstruct_state(&graph, &lineage, &t).unwrap();
    assert_eq!(state, CapabilityState::Active);
}

#[test]
fn o17_a_02_truncated_record_fails_closed() {
    let nodes = vec![
        make_node("prop", "EFFECT_PROPOSAL-v1", "dig_prop", None, "2026-08-20T09:00:00Z", serde_json::json!({})),
    ];
    let graph = EvidenceGraph::new(nodes);
    let lineage = vec![NodeId("prop".into())];
    let t = ReplayTimestamp("2026-08-20T11:00:00Z".into());

    let res = reconstruct_state(&graph, &lineage, &t);
    assert_eq!(res, Err(ReplayError::BrokenLineage));
}

#[test]
fn o17_a_03_interleaved_invalid_event_fails_closed() {
    let nodes = vec![
        make_node("prop", "EFFECT_PROPOSAL-v1", "dig_prop", None, "2026-08-20T09:00:00Z", serde_json::json!({})),
        make_node("life", "CAPABILITY_LIFECYCLE_EVENT-v1", "dig_life", Some("dig_prop"), "2026-08-20T10:00:00Z", serde_json::json!({"new_state": "CORRUPT_STATE"})),
    ];
    let graph = EvidenceGraph::new(nodes);
    let lineage = vec![NodeId("life".into()), NodeId("prop".into())];
    let t = ReplayTimestamp("2026-08-20T11:00:00Z".into());

    let res = reconstruct_state(&graph, &lineage, &t);
    assert_eq!(res, Err(ReplayError::BrokenLineage));
}

#[test]
fn o17_a_04_replay_twice_produces_identical_state() {
    let nodes = vec![
        make_node("prop", "EFFECT_PROPOSAL-v1", "dig_prop", None, "2026-08-20T09:00:00Z", serde_json::json!({})),
        make_node("life", "CAPABILITY_LIFECYCLE_EVENT-v1", "dig_life", Some("dig_prop"), "2026-08-20T10:00:00Z", serde_json::json!({"new_state": "ACTIVE"})),
    ];
    let graph = EvidenceGraph::new(nodes);
    let lineage = vec![NodeId("life".into()), NodeId("prop".into())];
    let t = ReplayTimestamp("2026-08-20T11:00:00Z".into());

    let state_1 = reconstruct_state(&graph, &lineage, &t).unwrap();
    let state_2 = reconstruct_state(&graph, &lineage, &t).unwrap();
    assert_eq!(state_1, state_2);
}
