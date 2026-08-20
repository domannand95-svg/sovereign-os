use crate::replay::error::ReplayError;
use crate::replay::graph::EvidenceGraph;
use crate::replay::model::NodeId;

pub fn verify_ancestry(graph: &EvidenceGraph, lineage: &[NodeId]) -> Result<(), ReplayError> {
    if lineage.is_empty() {
        return Err(ReplayError::InvalidEvidenceGraph);
    }

    for pair in lineage.windows(2) {
        let child_id = &pair[0];
        let parent_id = &pair[1];

        let child_node = graph.get(child_id).ok_or(ReplayError::BrokenLineage)?;
        let parent_node = graph.get(parent_id).ok_or(ReplayError::BrokenLineage)?;

        // Verify that child's parent_digest matches parent's digest
        if let Some(ref parent_dig) = child_node.parent_digest {
            if parent_dig != &parent_node.digest {
                return Err(ReplayError::InvalidCryptographicAncestry);
            }
        } else {
            // A non-root child in the trace missing a parent_digest link is a broken lineage
            return Err(ReplayError::BrokenLineage);
        }
    }

    // Verify root node has no parent_digest (or if it does, it doesn't resolve)
    if let Some(root_id) = lineage.last() {
        if let Some(root_node) = graph.get(root_id) {
            if root_node.parent_digest.is_some() {
                // Ensure root's parent digest doesn't point to an existing node in the graph
                if let Some(ref p_dig) = root_node.parent_digest {
                    if graph.find_by_digest(p_dig).is_some() {
                        return Err(ReplayError::InvalidCryptographicAncestry);
                    }
                }
            }
        }
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::model::{EvidenceDigest, EvidenceNode, ReplayTimestamp, SchemaVersion};

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
    fn test_valid_digest_chain_verifies() {
        let nodes = vec![
            make_node("prop", "dig_prop", None),
            make_node("eval", "dig_eval", Some("dig_prop")),
            make_node("comp", "dig_comp", Some("dig_eval")),
            make_node("adm", "dig_adm", Some("dig_comp")),
            make_node("life", "dig_life", Some("dig_adm")),
        ];
        let graph = EvidenceGraph::new(nodes);
        let lineage = vec![
            NodeId("life".into()),
            NodeId("adm".into()),
            NodeId("comp".into()),
            NodeId("eval".into()),
            NodeId("prop".into()),
        ];

        let res = verify_ancestry(&graph, &lineage);
        assert!(res.is_ok());
    }

    #[test]
    fn test_invalid_parent_digest_rejected() {
        let nodes = vec![
            make_node("prop", "dig_prop", None),
            make_node("life", "dig_life", Some("dig_tampered")),
        ];
        let graph = EvidenceGraph::new(nodes);
        let lineage = vec![NodeId("life".into()), NodeId("prop".into())];

        let res = verify_ancestry(&graph, &lineage);
        assert_eq!(res, Err(ReplayError::InvalidCryptographicAncestry));
    }

    #[test]
    fn test_root_node_without_parent_is_valid() {
        let nodes = vec![make_node("prop", "dig_prop", None)];
        let graph = EvidenceGraph::new(nodes);
        let lineage = vec![NodeId("prop".into())];

        let res = verify_ancestry(&graph, &lineage);
        assert!(res.is_ok());
    }

    #[test]
    fn test_missing_digest_reference_rejected() {
        let nodes = vec![make_node("life", "dig_life", Some("dig_nonexistent"))];
        let graph = EvidenceGraph::new(nodes);
        let lineage = vec![NodeId("life".into()), NodeId("nonexistent".into())];

        let res = verify_ancestry(&graph, &lineage);
        assert!(res.is_err());
    }
}
