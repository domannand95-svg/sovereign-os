use std::collections::HashMap;
use super::model::{EvidenceDigest, EvidenceNode, NodeId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceGraph {
    nodes: HashMap<NodeId, EvidenceNode>,
}

impl EvidenceGraph {
    pub fn new(nodes: Vec<EvidenceNode>) -> Self {
        let nodes = nodes
            .into_iter()
            .map(|node| (node.id.clone(), node))
            .collect();

        Self { nodes }
    }

    pub fn get(&self, id: &NodeId) -> Option<&EvidenceNode> {
        self.nodes.get(id)
    }

    pub fn find_by_digest(&self, digest: &EvidenceDigest) -> Option<&EvidenceNode> {
        self.nodes.values().find(|n| &n.digest == digest)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::model::{SchemaVersion, ReplayTimestamp};

    fn sample_node(id: &str, digest: &str, parent_digest: Option<&str>) -> EvidenceNode {
        EvidenceNode {
            id: NodeId(id.into()),
            record_type: "EFFECT_PROPOSAL-v1".into(),
            schema_version: SchemaVersion("EFFECT_PROPOSAL-v1".into()),
            digest: EvidenceDigest(digest.into()),
            parent_digest: parent_digest.map(|p| EvidenceDigest(p.into())),
            timestamp: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
        }
    }

    #[test]
    fn test_graph_ingestion_is_deterministic() {
        let nodes_a = vec![sample_node("a", "dig_a", None), sample_node("b", "dig_b", Some("dig_a"))];
        let nodes_b = vec![sample_node("b", "dig_b", Some("dig_a")), sample_node("a", "dig_a", None)];

        let graph_a = EvidenceGraph::new(nodes_a);
        let graph_b = EvidenceGraph::new(nodes_b);

        assert_eq!(graph_a, graph_b);
        assert_eq!(graph_a.len(), 2);
    }
}
