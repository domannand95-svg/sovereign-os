use crate::replay::error::ReplayError;
use crate::replay::graph::EvidenceGraph;
use crate::replay::model::{CapabilityState, NodeId, ReplayTimestamp};
use crate::replay::verification::verify_ancestry;

pub fn reconstruct_state(
    graph: &EvidenceGraph,
    lineage: &[NodeId],
    timestamp: &ReplayTimestamp,
) -> Result<CapabilityState, ReplayError> {
    // 1. Enforce verified ancestry before any interpretation
    verify_ancestry(graph, lineage)?;

    // 2. Locate lifecycle nodes in the verified lineage
    let mut lifecycle_nodes = Vec::new();
    for node_id in lineage {
        if let Some(node) = graph.get(node_id) {
            if node.record_type == "CAPABILITY_LIFECYCLE_EVENT-v1" {
                lifecycle_nodes.push(node);
            }
        }
    }

    if lifecycle_nodes.is_empty() {
        return Err(ReplayError::BrokenLineage);
    }

    // 3. Sort lifecycle nodes by timestamp to evaluate temporal progression
    lifecycle_nodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    // 4. Find the latest lifecycle event occurring at or before the supplied historical timestamp
    let mut active_state = CapabilityState::Unknown;

    for node in lifecycle_nodes {
        if node.timestamp <= *timestamp {
            if let Some(state_str) = node.payload.get("new_state").and_then(|v| v.as_str()) {
                active_state = match state_str {
                    "ACTIVE" => CapabilityState::Active,
                    "SUSPENDED" => CapabilityState::Suspended,
                    "EXPIRED" => CapabilityState::Expired,
                    "REVOKED" => CapabilityState::Revoked,
                    "INERT" => CapabilityState::Inert,
                    _ => CapabilityState::Unknown,
                };
            }
        } else {
            break;
        }
    }

    if active_state == CapabilityState::Unknown {
        return Err(ReplayError::BrokenLineage);
    }

    Ok(active_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::model::{EvidenceDigest, EvidenceNode, SchemaVersion};

    fn make_node(id: &str, record_type: &str, digest: &str, parent_digest: Option<&str>, timestamp: &str, payload: serde_json::Value) -> EvidenceNode {
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
    fn test_active_state_reconstructed_at_historical_timestamp() {
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
    fn test_revoked_state_reconstructed_after_revocation() {
        let nodes = vec![
            make_node("prop", "EFFECT_PROPOSAL-v1", "dig_prop", None, "2026-08-20T09:00:00Z", serde_json::json!({})),
            make_node("life1", "CAPABILITY_LIFECYCLE_EVENT-v1", "dig_life1", Some("dig_prop"), "2026-08-20T10:00:00Z", serde_json::json!({"new_state": "ACTIVE"})),
            make_node("life2", "CAPABILITY_LIFECYCLE_EVENT-v1", "dig_life2", Some("dig_life1"), "2026-08-21T10:00:00Z", serde_json::json!({"new_state": "REVOKED"})),
        ];
        let graph = EvidenceGraph::new(nodes);
        let lineage = vec![NodeId("life2".into()), NodeId("life1".into()), NodeId("prop".into())];
        let t = ReplayTimestamp("2026-08-21T12:00:00Z".into());

        let state = reconstruct_state(&graph, &lineage, &t).unwrap();
        assert_eq!(state, CapabilityState::Revoked);
    }

    #[test]
    fn test_missing_lifecycle_anchor_rejected() {
        let nodes = vec![
            make_node("prop", "EFFECT_PROPOSAL-v1", "dig_prop", None, "2026-08-20T09:00:00Z", serde_json::json!({})),
        ];
        let graph = EvidenceGraph::new(nodes);
        let lineage = vec![NodeId("prop".into())];
        let t = ReplayTimestamp("2026-08-20T11:00:00Z".into());

        let res = reconstruct_state(&graph, &lineage, &t);
        assert_eq!(res, Err(ReplayError::BrokenLineage));
    }
}
