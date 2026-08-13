use crate::{Caid, ObjectClass, RegistryEdge, RegistryError, RegistryNode, VersionedRegistryNode};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct RegistryGraph {
    nodes: HashMap<Caid, RegistryNode>,
    versioned_nodes: HashMap<Caid, VersionedRegistryNode>,
    dependents: HashMap<Caid, Vec<Caid>>,
    edges: HashSet<RegistryEdge>,
}

impl RegistryGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains(&self, caid: &Caid) -> bool {
        self.nodes.contains_key(caid)
    }

    pub fn get_node(&self, caid: &Caid) -> Option<&RegistryNode> {
        self.nodes.get(caid)
    }

    pub fn get_versioned_node(&self, caid: &Caid) -> Option<&VersionedRegistryNode> {
        self.versioned_nodes.get(caid)
    }

    pub fn object_class(&self, caid: &Caid) -> Option<ObjectClass> {
        self.versioned_nodes
            .get(caid)
            .map(VersionedRegistryNode::class)
    }

    pub fn insert_versioned_node(
        &mut self,
        node: VersionedRegistryNode,
    ) -> Result<(), RegistryError> {
        let node_caid = node.caid();

        if self.nodes.contains_key(&node_caid) || self.versioned_nodes.contains_key(&node_caid) {
            return Err(RegistryError::DuplicateEntity);
        }

        for parent in node.parents() {
            if !self.contains_admitted_node(parent) {
                return Err(RegistryError::UnresolvedReference);
            }
        }

        self.assert_acyclic_insertion(&node_caid, node.parents())?;

        for parent in node.parents() {
            self.dependents.entry(*parent).or_default().push(node_caid);
        }

        self.versioned_nodes.insert(node_caid, node);

        Ok(())
    }

    fn contains_admitted_node(&self, caid: &Caid) -> bool {
        self.nodes.contains_key(caid) || self.versioned_nodes.contains_key(caid)
    }

    pub fn insert_node(&mut self, node: RegistryNode) -> Result<(), RegistryError> {
        node.validate_identity()?;

        let node_caid = node.caid();

        if self.contains_admitted_node(&node_caid) {
            return Err(RegistryError::DuplicateEntity);
        }

        for parent in node.parents() {
            if !self.nodes.contains_key(parent) {
                return Err(RegistryError::UnresolvedReference);
            }
        }

        self.assert_acyclic_insertion(&node_caid, node.parents())?;

        for parent in node.parents() {
            self.dependents.entry(*parent).or_default().push(node_caid);
        }

        self.nodes.insert(node_caid, node);
        Ok(())
    }

    pub fn insert_edge(&mut self, edge: RegistryEdge) -> Result<(), RegistryError> {
        let parent = edge.parent();
        let child = edge.child();

        // 1. Both endpoints must already exist in the admitted graph.
        if !self.contains_admitted_node(&parent) || !self.contains_admitted_node(&child) {
            return Err(RegistryError::UnresolvedReference);
        }

        // 2. Exact duplicate insertion is an idempotent no-op.
        //
        // This deliberately occurs before cycle analysis because an already
        // admitted semantic tuple does not represent a new graph transition.
        if self.edges.contains(&edge) {
            return Ok(());
        }

        // 3. A new semantic edge must preserve the single unified DAG formed
        // by legacy provenance relationships plus typed semantic edges.
        self.assert_acyclic_edge_insertion(&edge)?;

        // 4. Different relation types between the same endpoints remain
        // distinct RegistryEdge values and may coexist.
        self.edges.insert(edge);

        Ok(())
    }

    pub fn contains_edge(&self, edge: &RegistryEdge) -> bool {
        self.edges.contains(edge)
    }

    pub fn outgoing_edges(&self, parent: &Caid) -> Vec<&RegistryEdge> {
        let mut edges: Vec<&RegistryEdge> = self
            .edges
            .iter()
            .filter(|edge| edge.parent() == *parent)
            .collect();

        // Never expose HashSet iteration order through the public API.
        edges.sort();
        edges
    }

    pub fn incoming_edges(&self, child: &Caid) -> Vec<&RegistryEdge> {
        let mut edges: Vec<&RegistryEdge> = self
            .edges
            .iter()
            .filter(|edge| edge.child() == *child)
            .collect();

        // Never expose HashSet iteration order through the public API.
        edges.sort();
        edges
    }

    fn assert_acyclic_insertion(
        &self,
        target: &Caid,
        parents: &[Caid],
    ) -> Result<(), RegistryError> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        stack.insert(*target);

        for parent in parents {
            self.dfs_cycle_walk(parent, &mut visited, &mut stack)?;
        }

        Ok(())
    }

    fn assert_acyclic_edge_insertion(&self, edge: &RegistryEdge) -> Result<(), RegistryError> {
        let parent = edge.parent();
        let child = edge.child();

        // A -> A is a directed cycle of length one.
        if parent == child {
            return Err(RegistryError::GraphCycleDetected);
        }

        // Adding parent -> child creates a cycle exactly when the existing
        // combined graph already contains a path child -> ... -> parent.
        let mut visited = HashSet::new();

        if self.combined_path_exists(&child, &parent, &mut visited) {
            return Err(RegistryError::GraphCycleDetected);
        }

        Ok(())
    }

    fn combined_path_exists(
        &self,
        current: &Caid,
        target: &Caid,
        visited: &mut HashSet<Caid>,
    ) -> bool {
        if current == target {
            return true;
        }

        if !visited.insert(*current) {
            return false;
        }

        // Legacy provenance direction:
        //
        // parent -> dependent child
        if let Some(children) = self.dependents.get(current) {
            for child in children {
                if self.combined_path_exists(child, target, visited) {
                    return true;
                }
            }
        }

        // Typed semantic-edge direction:
        //
        // edge.parent -> edge.child
        for edge in &self.edges {
            if edge.parent() == *current {
                let child = edge.child();

                if self.combined_path_exists(&child, target, visited) {
                    return true;
                }
            }
        }

        false
    }

    fn dfs_cycle_walk(
        &self,
        current: &Caid,
        visited: &mut HashSet<Caid>,
        stack: &mut HashSet<Caid>,
    ) -> Result<(), RegistryError> {
        if stack.contains(current) {
            return Err(RegistryError::GraphCycleDetected);
        }

        if visited.contains(current) {
            return Ok(());
        }

        stack.insert(*current);

        // Legacy structural relationships.
        if let Some(children) = self.dependents.get(current) {
            for child in children {
                self.dfs_cycle_walk(child, visited, stack)?;
            }
        }

        // Typed semantic relationships participate in the same DAG invariant.
        for edge in &self.edges {
            if edge.parent() == *current {
                let child = edge.child();
                self.dfs_cycle_walk(&child, visited, stack)?;
            }
        }

        stack.remove(current);
        visited.insert(*current);

        Ok(())
    }

    pub fn resolve_lineage(&self, target: &Caid) -> Result<Vec<Caid>, RegistryError> {
        if !self.nodes.contains_key(target) {
            return Err(RegistryError::UnresolvedReference);
        }

        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        self.topological_sort_walk(target, &mut visited, &mut stack, &mut order)?;

        Ok(order)
    }

    fn topological_sort_walk(
        &self,
        current: &Caid,
        visited: &mut HashSet<Caid>,
        stack: &mut HashSet<Caid>,
        order: &mut Vec<Caid>,
    ) -> Result<(), RegistryError> {
        if stack.contains(current) {
            return Err(RegistryError::GraphCycleDetected);
        }

        if visited.contains(current) {
            return Ok(());
        }

        stack.insert(*current);

        let node = self
            .nodes
            .get(current)
            .ok_or(RegistryError::UnresolvedReference)?;

        // Legacy lineage remains defined exclusively by RegistryNode::parents().
        // Typed semantic edges do not alter this public compatibility surface.
        for parent in node.parents() {
            self.topological_sort_walk(parent, visited, stack, order)?;
        }

        stack.remove(current);
        visited.insert(*current);
        order.push(*current);

        Ok(())
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
    use crate::{RegistryEdge, RegistryNode, RegistryNodeType, RelationType};

    fn node(node_type: RegistryNodeType, payload: &[u8], parents: Vec<Caid>) -> RegistryNode {
        RegistryNode::new(node_type, payload.to_vec(), parents).unwrap()
    }

    fn insert_test_node(graph: &mut RegistryGraph, seed: u8) -> Caid {
        let test_node = node(RegistryNodeType::Capability, &[seed], vec![]);

        let caid = test_node.caid();
        graph.insert_node(test_node).unwrap();
        caid
    }

    fn insert_test_node_with_parents(
        graph: &mut RegistryGraph,
        seed: u8,
        parents: Vec<Caid>,
    ) -> Caid {
        let test_node = node(RegistryNodeType::Capability, &[seed], parents);

        let caid = test_node.caid();
        graph.insert_node(test_node).unwrap();
        caid
    }

    // -------------------------------------------------------------------------
    // Existing Legacy v1 graph tests
    // -------------------------------------------------------------------------

    #[test]
    fn empty_graph_state() {
        let graph = RegistryGraph::new();

        assert_eq!(graph.len(), 0);
        assert!(graph.is_empty());
    }

    #[test]
    fn valid_topological_node_insertion() {
        let mut graph = RegistryGraph::new();

        let root = node(RegistryNodeType::Actor, b"genesis_identity", vec![]);
        let root_caid = root.caid();
        graph.insert_node(root).unwrap();

        let child = node(
            RegistryNodeType::Capability,
            b"kernel_gate_0",
            vec![root_caid],
        );
        let child_caid = child.caid();
        graph.insert_node(child).unwrap();

        assert_eq!(graph.len(), 2);
        assert!(graph.contains(&root_caid));
        assert!(graph.contains(&child_caid));
    }

    #[test]
    fn duplicate_insertion_fails() {
        let mut graph = RegistryGraph::new();

        let manifest = node(RegistryNodeType::DataManifest, b"static_payload", vec![]);

        graph.insert_node(manifest.clone()).unwrap();

        assert_eq!(
            graph.insert_node(manifest).unwrap_err(),
            RegistryError::DuplicateEntity
        );
    }

    #[test]
    fn unresolved_reference_protection() {
        let mut graph = RegistryGraph::new();
        let phantom = Caid([0xFF; 32]);

        let orphan = node(RegistryNodeType::Capability, b"orphan_gate", vec![phantom]);

        assert_eq!(
            graph.insert_node(orphan).unwrap_err(),
            RegistryError::UnresolvedReference
        );
    }

    #[test]
    fn lineage_resolution_ordering() {
        let mut graph = RegistryGraph::new();

        let n0 = node(RegistryNodeType::Actor, b"layer_0", vec![]);
        let c0 = n0.caid();
        graph.insert_node(n0).unwrap();

        let n1 = node(RegistryNodeType::DataManifest, b"layer_1", vec![c0]);
        let c1 = n1.caid();
        graph.insert_node(n1).unwrap();

        let n2 = node(RegistryNodeType::Capability, b"layer_2", vec![c1]);
        let c2 = n2.caid();
        graph.insert_node(n2).unwrap();

        assert_eq!(graph.resolve_lineage(&c2).unwrap(), vec![c0, c1, c2]);
    }

    // -------------------------------------------------------------------------
    // Registry v2 typed semantic-edge admission tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_edge_insertion_requires_existing_endpoints() {
        let mut graph = RegistryGraph::default();

        let parent = insert_test_node(&mut graph, 1);
        let unknown = Caid([99; 32]);

        let missing_child = RegistryEdge::new(parent, unknown, RelationType::DerivedFrom);

        assert!(matches!(
            graph.insert_edge(missing_child),
            Err(RegistryError::UnresolvedReference)
        ));

        let child = insert_test_node(&mut graph, 2);

        let missing_parent = RegistryEdge::new(unknown, child, RelationType::DerivedFrom);

        assert!(matches!(
            graph.insert_edge(missing_parent),
            Err(RegistryError::UnresolvedReference)
        ));
    }

    #[test]
    fn test_edge_insertion_success_and_query() {
        let mut graph = RegistryGraph::default();

        let parent = insert_test_node(&mut graph, 1);
        let child = insert_test_node(&mut graph, 2);

        let edge = RegistryEdge::new(parent, child, RelationType::Governs);

        assert!(!graph.contains_edge(&edge));

        graph.insert_edge(edge).unwrap();

        assert!(graph.contains_edge(&edge));

        let outgoing = graph.outgoing_edges(&parent);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0], &edge);

        let incoming = graph.incoming_edges(&child);
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0], &edge);
    }

    #[test]
    fn test_edge_insertion_idempotence() {
        let mut graph = RegistryGraph::default();

        let parent = insert_test_node(&mut graph, 1);
        let child = insert_test_node(&mut graph, 2);

        let edge = RegistryEdge::new(parent, child, RelationType::DependsOn);

        assert!(graph.insert_edge(edge).is_ok());

        let before = graph.outgoing_edges(&parent);
        assert_eq!(before.len(), 1);

        // Exact tuple re-insertion is a successful no-op.
        assert!(graph.insert_edge(edge).is_ok());

        let after = graph.outgoing_edges(&parent);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0], &edge);
    }

    #[test]
    fn test_edge_relation_multiplicity() {
        let mut graph = RegistryGraph::default();

        let parent = insert_test_node(&mut graph, 1);
        let child = insert_test_node(&mut graph, 2);

        let consumes = RegistryEdge::new(parent, child, RelationType::Consumes);

        let produces = RegistryEdge::new(parent, child, RelationType::Produces);

        graph.insert_edge(consumes).unwrap();
        graph.insert_edge(produces).unwrap();

        assert!(graph.contains_edge(&consumes));
        assert!(graph.contains_edge(&produces));

        let outgoing = graph.outgoing_edges(&parent);
        assert_eq!(outgoing.len(), 2);
    }

    #[test]
    fn test_typed_edge_cycle_rejection() {
        let mut graph = RegistryGraph::default();

        let a = insert_test_node(&mut graph, 1);
        let b = insert_test_node(&mut graph, 2);

        let edge_ab = RegistryEdge::new(a, b, RelationType::Governs);

        graph.insert_edge(edge_ab).unwrap();

        let edge_ba = RegistryEdge::new(b, a, RelationType::Governs);

        assert!(matches!(
            graph.insert_edge(edge_ba),
            Err(RegistryError::GraphCycleDetected)
        ));
    }

    #[test]
    fn test_typed_edge_self_cycle_rejection() {
        let mut graph = RegistryGraph::default();

        let node_caid = insert_test_node(&mut graph, 1);

        let edge = RegistryEdge::new(node_caid, node_caid, RelationType::DependsOn);

        assert!(matches!(
            graph.insert_edge(edge),
            Err(RegistryError::GraphCycleDetected)
        ));
    }

    #[test]
    fn test_typed_edge_rejects_cycle_through_legacy_lineage() {
        let mut graph = RegistryGraph::default();

        let parent = insert_test_node(&mut graph, 1);

        let child = insert_test_node_with_parents(&mut graph, 2, vec![parent]);

        // Legacy provenance already establishes:
        //
        // parent -> child
        //
        // This typed edge would close:
        //
        // parent -> child -> parent
        let edge = RegistryEdge::new(child, parent, RelationType::DependsOn);

        assert!(matches!(
            graph.insert_edge(edge),
            Err(RegistryError::GraphCycleDetected)
        ));
    }

    #[test]
    fn test_outgoing_edge_query_determinism() {
        let mut graph = RegistryGraph::default();

        let parent = insert_test_node(&mut graph, 1);

        let child1 = insert_test_node(&mut graph, 2);
        let child2 = insert_test_node(&mut graph, 3);
        let child3 = insert_test_node(&mut graph, 4);

        let edge1 = RegistryEdge::new(parent, child1, RelationType::Verifies);

        let edge2 = RegistryEdge::new(parent, child2, RelationType::Verifies);

        let edge3 = RegistryEdge::new(parent, child3, RelationType::Verifies);

        // Deliberately non-canonical insertion order.
        graph.insert_edge(edge3).unwrap();
        graph.insert_edge(edge1).unwrap();
        graph.insert_edge(edge2).unwrap();

        let outgoing = graph.outgoing_edges(&parent);

        let mut expected = vec![&edge1, &edge2, &edge3];
        expected.sort();

        assert_eq!(outgoing, expected);
    }

    #[test]
    fn test_incoming_edge_query_determinism() {
        let mut graph = RegistryGraph::default();

        let parent1 = insert_test_node(&mut graph, 1);
        let parent2 = insert_test_node(&mut graph, 2);
        let parent3 = insert_test_node(&mut graph, 3);

        let child = insert_test_node(&mut graph, 4);

        let edge1 = RegistryEdge::new(parent1, child, RelationType::Verifies);

        let edge2 = RegistryEdge::new(parent2, child, RelationType::Verifies);

        let edge3 = RegistryEdge::new(parent3, child, RelationType::Verifies);

        // Deliberately non-canonical insertion order.
        graph.insert_edge(edge3).unwrap();
        graph.insert_edge(edge1).unwrap();
        graph.insert_edge(edge2).unwrap();

        let incoming = graph.incoming_edges(&child);

        let mut expected = vec![&edge1, &edge2, &edge3];
        expected.sort();

        assert_eq!(incoming, expected);
    }

    #[test]
    fn test_semantic_edges_do_not_modify_legacy_lineage() {
        let mut graph = RegistryGraph::default();

        let legacy_parent = insert_test_node(&mut graph, 1);

        let legacy_child = insert_test_node_with_parents(&mut graph, 2, vec![legacy_parent]);

        let semantic_parent = insert_test_node(&mut graph, 3);

        let parent_before = graph.resolve_lineage(&legacy_parent).unwrap();

        let child_before = graph.resolve_lineage(&legacy_child).unwrap();

        graph
            .insert_edge(RegistryEdge::new(
                semantic_parent,
                legacy_child,
                RelationType::Verifies,
            ))
            .unwrap();

        let parent_after = graph.resolve_lineage(&legacy_parent).unwrap();

        let child_after = graph.resolve_lineage(&legacy_child).unwrap();

        assert_eq!(parent_after, parent_before);
        assert_eq!(child_after, child_before);
    }
}

#[cfg(test)]
mod v2_node_admission_tests {
    use super::*;
    use crate::{ObjectClass, RegistryNodeType, VersionedRegistryNode};

    fn insert_legacy_node(graph: &mut RegistryGraph, seed: u8) -> Caid {
        let node = RegistryNode::new(RegistryNodeType::Capability, vec![seed], vec![]).unwrap();

        let caid = node.caid();
        graph.insert_node(node).unwrap();
        caid
    }

    fn versioned_node(class: ObjectClass, seed: u8, parents: Vec<Caid>) -> VersionedRegistryNode {
        VersionedRegistryNode::new(class, parents, vec![seed]).unwrap()
    }

    #[test]
    fn v2_node_can_be_admitted_and_retrieved() {
        let mut graph = RegistryGraph::new();

        let node = versioned_node(ObjectClass::Workflow, 0x11, vec![]);
        let caid = node.caid();

        graph.insert_versioned_node(node.clone()).unwrap();

        assert_eq!(graph.get_versioned_node(&caid), Some(&node));
        assert_eq!(graph.object_class(&caid), Some(ObjectClass::Workflow));
    }

    #[test]
    fn v2_node_rejects_unresolved_provenance_parent() {
        let mut graph = RegistryGraph::new();

        let unknown_parent = Caid([0xAA; 32]);

        let node = versioned_node(ObjectClass::EvidencePackage, 0x22, vec![unknown_parent]);

        assert_eq!(
            graph.insert_versioned_node(node),
            Err(RegistryError::UnresolvedReference)
        );
    }

    #[test]
    fn v2_node_accepts_admitted_legacy_parent() {
        let mut graph = RegistryGraph::new();

        let legacy_parent = insert_legacy_node(&mut graph, 0x31);

        let node = versioned_node(ObjectClass::Specification, 0x32, vec![legacy_parent]);

        let caid = node.caid();

        graph.insert_versioned_node(node).unwrap();

        assert!(graph.get_versioned_node(&caid).is_some());
        assert_eq!(graph.object_class(&caid), Some(ObjectClass::Specification));
    }

    #[test]
    fn v2_node_accepts_admitted_v2_parent() {
        let mut graph = RegistryGraph::new();

        let parent = versioned_node(ObjectClass::Dataset, 0x41, vec![]);
        let parent_caid = parent.caid();

        graph.insert_versioned_node(parent).unwrap();

        let child = versioned_node(ObjectClass::Workflow, 0x42, vec![parent_caid]);

        let child_caid = child.caid();

        graph.insert_versioned_node(child).unwrap();

        assert_eq!(graph.object_class(&parent_caid), Some(ObjectClass::Dataset));

        assert_eq!(graph.object_class(&child_caid), Some(ObjectClass::Workflow));
    }

    #[test]
    fn duplicate_v2_node_admission_fails_closed() {
        let mut graph = RegistryGraph::new();

        let node = versioned_node(ObjectClass::Policy, 0x51, vec![]);

        graph.insert_versioned_node(node.clone()).unwrap();

        assert_eq!(
            graph.insert_versioned_node(node),
            Err(RegistryError::DuplicateEntity)
        );
    }

    #[test]
    fn v2_node_does_not_masquerade_as_legacy_registry_node() {
        let mut graph = RegistryGraph::new();

        let node = versioned_node(ObjectClass::Event, 0x61, vec![]);
        let caid = node.caid();

        graph.insert_versioned_node(node).unwrap();

        assert!(graph.get_versioned_node(&caid).is_some());

        // Historical compatibility API remains Legacy-v1-only.
        assert!(graph.get_node(&caid).is_none());
    }
}

#[cfg(test)]
mod v2_cross_version_graph_tests {
    use super::*;
    use crate::{ObjectClass, RegistryNodeType, RelationType, VersionedRegistryNode};

    fn insert_legacy_node(graph: &mut RegistryGraph, seed: u8) -> Caid {
        let node = RegistryNode::new(RegistryNodeType::Capability, vec![seed], vec![]).unwrap();

        let caid = node.caid();
        graph.insert_node(node).unwrap();
        caid
    }

    fn insert_v2_node(
        graph: &mut RegistryGraph,
        class: ObjectClass,
        seed: u8,
        parents: Vec<Caid>,
    ) -> Caid {
        let node = VersionedRegistryNode::new(class, parents, vec![seed]).unwrap();

        let caid = node.caid();
        graph.insert_versioned_node(node).unwrap();
        caid
    }

    #[test]
    fn semantic_edge_accepts_v2_to_v2_endpoints() {
        let mut graph = RegistryGraph::new();

        let parent = insert_v2_node(&mut graph, ObjectClass::Specification, 0x71, vec![]);

        let child = insert_v2_node(&mut graph, ObjectClass::Workflow, 0x72, vec![]);

        let edge = RegistryEdge::new(parent, child, RelationType::Governs);

        graph.insert_edge(edge).unwrap();

        assert!(graph.contains_edge(&edge));
    }

    #[test]
    fn semantic_edge_accepts_legacy_to_v2_endpoints() {
        let mut graph = RegistryGraph::new();

        let parent = insert_legacy_node(&mut graph, 0x81);

        let child = insert_v2_node(&mut graph, ObjectClass::EvidencePackage, 0x82, vec![]);

        let edge = RegistryEdge::new(parent, child, RelationType::Produces);

        graph.insert_edge(edge).unwrap();

        assert!(graph.contains_edge(&edge));
    }

    #[test]
    fn semantic_edge_accepts_v2_to_legacy_endpoints() {
        let mut graph = RegistryGraph::new();

        let parent = insert_v2_node(&mut graph, ObjectClass::Policy, 0x91, vec![]);

        let child = insert_legacy_node(&mut graph, 0x92);

        let edge = RegistryEdge::new(parent, child, RelationType::Governs);

        graph.insert_edge(edge).unwrap();

        assert!(graph.contains_edge(&edge));
    }

    #[test]
    fn semantic_edge_rejects_cycle_through_v2_provenance() {
        let mut graph = RegistryGraph::new();

        let parent = insert_v2_node(&mut graph, ObjectClass::Dataset, 0xA1, vec![]);

        let child = insert_v2_node(&mut graph, ObjectClass::Workflow, 0xA2, vec![parent]);

        // v2 provenance already establishes:
        //
        // parent -> child
        //
        // This semantic relation would close:
        //
        // parent -> child -> parent
        let edge = RegistryEdge::new(child, parent, RelationType::DependsOn);

        assert_eq!(
            graph.insert_edge(edge),
            Err(RegistryError::GraphCycleDetected)
        );
    }
}
