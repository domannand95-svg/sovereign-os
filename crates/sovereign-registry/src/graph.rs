use crate::{Caid, RegistryError, RegistryNode};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct RegistryGraph {
    nodes: HashMap<Caid, RegistryNode>,
    dependents: HashMap<Caid, Vec<Caid>>,
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

    pub fn insert_node(&mut self, node: RegistryNode) -> Result<(), RegistryError> {
        node.validate_identity()?;

        let node_caid = node.caid();

        if self.nodes.contains_key(&node_caid) {
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

        if let Some(children) = self.dependents.get(current) {
            for child in children {
                self.dfs_cycle_walk(child, visited, stack)?;
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
    use crate::{RegistryNode, RegistryNodeType};

    fn node(node_type: RegistryNodeType, payload: &[u8], parents: Vec<Caid>) -> RegistryNode {
        RegistryNode::new(node_type, payload.to_vec(), parents).unwrap()
    }

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
}
