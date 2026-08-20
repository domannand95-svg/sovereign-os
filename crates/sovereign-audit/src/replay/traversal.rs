use std::collections::HashSet;
use crate::replay::error::ReplayError;
use crate::replay::graph::EvidenceGraph;
use crate::replay::model::NodeId;

pub fn traverse_ancestry(
    graph: &EvidenceGraph,
    start: &NodeId,
) -> Result<Vec<NodeId>, ReplayError> {
    let mut trace = Vec::new();
    let mut visited_nodes = HashSet::new();
    let mut visited_digests = HashSet::new();

    let mut current_id = start.clone();

    while let Some(node) = graph.get(&current_id) {
        if !visited_nodes.insert(node.id.clone()) {
            return Err(ReplayError::CyclicLineage);
        }
        if let Some(ref dig) = node.digest {
            if !visited_digests.insert(dig.clone()) {
                return Err(ReplayError::CyclicLineage);
            }
        }

        trace.push(node.id.clone());

        if let Some(ref parent_dig) = node.parent_digest {
            if let Some(p) = graph.find_by_digest(parent_dig) {
                current_id = p.id.clone();
            } else {
                return Err(ReplayError::BrokenLineage);
            }
        } else {
            break;
        }
    }

    Ok(trace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::model::{EvidenceDigest, EvidenceNode, SchemaVersion, ReplayTimestamp};

    fn make_node(id: &str, digest: &str, parent_digest: Option<&str>) -> EvidenceNode {
        EvidenceNode {
            id: NodeId(id.into()),
            record_type: "RECORD".into(),
            schema_version: SchemaVersion("v1".into()),
            digest: EvidenceDigest(digest.into()),
            parent_digest: parent_digest.map(|p| EvidenceDigest(p.into())),
            timestamp: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
        }
    }

    #[test]
    fn test_valid_five_level_ancestry_traversal() {
        let nodes = vec![
            make_node("prop", "dig_prop", None),
            make_node("eval", "dig_eval", Some("dig_prop")),
            make_node("comp", "dig_comp", Some("dig_eval")),
            make_node("adm", "dig_adm", Some("dig_comp")),
            make_node("life", "dig_life", Some("dig_adm")),
        ];
        let graph = EvidenceGraph::new(nodes);
        let trace = traverse_ancestry(&graph, &NodeId("life".into())).unwrap();

        assert_eq!(
            trace,
            vec![
                NodeId("life".into()),
                NodeId("adm".into()),
                NodeId("comp".into()),
                NodeId("eval".into()),
                NodeId("prop".into())
            ]
        );
    }

    #[test]
    fn test_missing_parent_breaks_lineage() {
        let nodes = vec![
            make_node("life", "dig_life", Some("dig_missing")),
        ];
        let graph = EvidenceGraph::new(nodes);
        let res = traverse_ancestry(&graph, &NodeId("life".into()));

        assert_eq!(res, Err(ReplayError::BrokenLineage));
    }

    #[test]
    fn test_cycle_detection_rejects_graph() {
        let nodes = vec![
            make_node("a", "dig_a", Some("dig_b")),
            make_node("b", "dig_b", Some("dig_a")),
        ];
        let graph = EvidenceGraph::new(nodes);
        let res = traverse_ancestry(&graph, &NodeId("a".into()));

        assert_eq!(res, Err(ReplayError::CyclicLineage));
    }

    #[test]
    fn test_identical_graphs_produce_identical_trace() {
        let nodes_1 = vec![
            make_node("prop", "dig_prop", None),
            make_node("life", "dig_life", Some("dig_prop")),
        ];
        let nodes_2 = vec![
            make_node("life", "dig_life", Some("dig_prop")),
            make_node("prop", "dig_prop", None),
        ];

        let g1 = EvidenceGraph::new(nodes_1);
        let g2 = EvidenceGraph::new(nodes_2);

        let t1 = traverse_ancestry(&g1, &NodeId("life".into())).unwrap();
        let t2 = traverse_ancestry(&g2, &NodeId("life".into())).unwrap();

        assert_eq!(t1, t2);
    }
}
