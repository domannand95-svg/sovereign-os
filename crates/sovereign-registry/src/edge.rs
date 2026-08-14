use crate::Caid;

/// Version 1.0 first-class object taxonomy defined by SPEC-REG-001.
///
/// These discriminators are distinct from the historical `RegistryNodeType`
/// tags used by Legacy Registry Encoding v1.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ObjectClass {
    Capability = 0x01,
    Specification = 0x02,
    Policy = 0x03,
    Event = 0x04,
    Dataset = 0x05,
    Workflow = 0x06,
    VerificationArtifact = 0x07,
    EvidencePackage = 0x08,
    RegistryGenesis = 0x09,
}

impl ObjectClass {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Capability),
            0x02 => Some(Self::Specification),
            0x03 => Some(Self::Policy),
            0x04 => Some(Self::Event),
            0x05 => Some(Self::Dataset),
            0x06 => Some(Self::Workflow),
            0x07 => Some(Self::VerificationArtifact),
            0x08 => Some(Self::EvidencePackage),
            0x09 => Some(Self::RegistryGenesis),
            _ => None,
        }
    }
}

/// Typed semantic relation categories defined by SPEC-REG-001.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RelationType {
    DerivedFrom = 0x01,
    DependsOn = 0x02,
    Supersedes = 0x03,
    Verifies = 0x04,
    Governs = 0x05,
    Produces = 0x06,
    Consumes = 0x07,
}

impl RelationType {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::DerivedFrom),
            0x02 => Some(Self::DependsOn),
            0x03 => Some(Self::Supersedes),
            0x04 => Some(Self::Verifies),
            0x05 => Some(Self::Governs),
            0x06 => Some(Self::Produces),
            0x07 => Some(Self::Consumes),
            _ => None,
        }
    }
}

/// Immutable semantic relationship between two registry nodes.
///
/// Registry edges do not possess independent CAIDs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegistryEdge {
    parent: Caid,
    child: Caid,
    relation: RelationType,
}

impl RegistryEdge {
    pub const fn new(parent: Caid, child: Caid, relation: RelationType) -> Self {
        Self {
            parent,
            child,
            relation,
        }
    }

    pub const fn parent(&self) -> Caid {
        self.parent
    }

    pub const fn child(&self) -> Caid {
        self.child
    }

    pub const fn relation(&self) -> RelationType {
        self.relation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_class_discriminators_are_frozen() {
        let cases = [
            (ObjectClass::Capability, 0x01),
            (ObjectClass::Specification, 0x02),
            (ObjectClass::Policy, 0x03),
            (ObjectClass::Event, 0x04),
            (ObjectClass::Dataset, 0x05),
            (ObjectClass::Workflow, 0x06),
            (ObjectClass::VerificationArtifact, 0x07),
            (ObjectClass::EvidencePackage, 0x08),
            (ObjectClass::RegistryGenesis, 0x09),
        ];

        for (class, tag) in cases {
            assert_eq!(class.as_u8(), tag);
            assert_eq!(ObjectClass::from_u8(tag), Some(class));
        }

        assert_eq!(ObjectClass::from_u8(0x00), None);
        assert_eq!(ObjectClass::from_u8(0x0A), None);
    }

    #[test]
    fn relation_type_discriminators_are_frozen() {
        let cases = [
            (RelationType::DerivedFrom, 0x01),
            (RelationType::DependsOn, 0x02),
            (RelationType::Supersedes, 0x03),
            (RelationType::Verifies, 0x04),
            (RelationType::Governs, 0x05),
            (RelationType::Produces, 0x06),
            (RelationType::Consumes, 0x07),
        ];

        for (relation, tag) in cases {
            assert_eq!(relation.as_u8(), tag);
            assert_eq!(RelationType::from_u8(tag), Some(relation));
        }

        assert_eq!(RelationType::from_u8(0x00), None);
        assert_eq!(RelationType::from_u8(0x08), None);
    }

    #[test]
    fn registry_edge_preserves_endpoints_and_relation() {
        let parent = Caid([0xAA; 32]);
        let child = Caid([0xBB; 32]);
        let edge = RegistryEdge::new(parent, child, RelationType::DerivedFrom);

        assert_eq!(edge.parent(), parent);
        assert_eq!(edge.child(), child);
        assert_eq!(edge.relation(), RelationType::DerivedFrom);
    }
}
