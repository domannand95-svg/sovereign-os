use crate::{Caid, ObjectClass, RegistryError, RegistryGraph};

pub fn validate_governed_reference(
    graph: &RegistryGraph,
    artifact_caid: &Caid,
    expected_class: ObjectClass,
) -> Result<(), RegistryError> {
    if let Some(node) = graph.get_versioned_node(artifact_caid) {
        let actual = node.class();

        if actual == expected_class {
            Ok(())
        } else {
            Err(RegistryError::ObjectClassMismatch {
                expected: expected_class,
                actual,
            })
        }
    } else if graph.get_node(artifact_caid).is_some() {
        Err(RegistryError::ObjectClassUnavailable)
    } else {
        Err(RegistryError::UnresolvedReference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RegistryNode, RegistryNodeType, VersionedRegistryNode};

    fn insert_v2_node(graph: &mut RegistryGraph, class: ObjectClass, seed: u8) -> Caid {
        let legacy_parent =
            RegistryNode::new(RegistryNodeType::Capability, vec![0xA0, seed], vec![]).unwrap();
        let parent_caid = legacy_parent.caid();
        graph.insert_node(legacy_parent).unwrap();

        let node = VersionedRegistryNode::new(class, vec![parent_caid], vec![seed]).unwrap();
        let caid = node.caid();

        graph.insert_versioned_node(node).unwrap();

        caid
    }

    #[test]
    fn unresolved_caid_fails_closed() {
        let graph = RegistryGraph::default();
        let unknown_caid = Caid([0x40; 32]);

        assert_eq!(
            validate_governed_reference(&graph, &unknown_caid, ObjectClass::Dataset,),
            Err(RegistryError::UnresolvedReference)
        );
    }

    #[test]
    fn matching_object_class_is_admissible() {
        let mut graph = RegistryGraph::default();

        let caid = insert_v2_node(&mut graph, ObjectClass::Dataset, 0x51);

        assert_eq!(
            validate_governed_reference(&graph, &caid, ObjectClass::Dataset,),
            Ok(())
        );
    }

    #[test]
    fn same_width_wrong_object_class_fails_closed() {
        let mut graph = RegistryGraph::default();

        let caid = insert_v2_node(&mut graph, ObjectClass::Policy, 0x61);

        let result = validate_governed_reference(&graph, &caid, ObjectClass::Dataset);

        match result {
            Err(RegistryError::ObjectClassMismatch { expected, actual }) => {
                assert_eq!(expected, ObjectClass::Dataset);
                assert_eq!(actual, ObjectClass::Policy);
            }

            other => panic!("expected ObjectClassMismatch, got {other:?}"),
        }
    }

    #[test]
    fn legacy_node_is_not_silently_assigned_a_v2_object_class() {
        let mut graph = RegistryGraph::default();

        let node = RegistryNode::new(RegistryNodeType::Capability, vec![0x71], vec![]).unwrap();

        let caid = node.caid();

        graph.insert_node(node).unwrap();

        assert_eq!(graph.object_class(&caid), None);
    }

    #[test]
    fn legacy_governed_reference_fails_when_object_class_is_unavailable() {
        let mut graph = RegistryGraph::default();

        let node = RegistryNode::new(RegistryNodeType::Capability, vec![0x72], vec![]).unwrap();

        let caid = node.caid();
        graph.insert_node(node).unwrap();

        assert_eq!(
            validate_governed_reference(&graph, &caid, ObjectClass::Dataset,),
            Err(RegistryError::ObjectClassUnavailable)
        );
    }
}
