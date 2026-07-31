use crate::{Caid, RegistryError};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RegistryNodeType {
    Capability,
    Actor,
    DataManifest,
}

impl RegistryNodeType {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Capability => 0x01,
            Self::Actor => 0x02,
            Self::DataManifest => 0x03,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryNode {
    caid: Caid,
    node_type: RegistryNodeType,
    payload: Vec<u8>,
    parents: Vec<Caid>,
}

impl RegistryNode {
    pub fn new(
        node_type: RegistryNodeType,
        payload: Vec<u8>,
        parents: Vec<Caid>,
    ) -> Result<Self, RegistryError> {
        if payload.is_empty() {
            return Err(RegistryError::SchemaViolation);
        }

        let caid = Self::derive_caid(node_type, &payload, &parents);

        Ok(Self {
            caid,
            node_type,
            payload,
            parents,
        })
    }

    pub fn derive_caid(node_type: RegistryNodeType, payload: &[u8], parents: &[Caid]) -> Caid {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[node_type.as_u8()]);
        hasher.update(&(payload.len() as u64).to_be_bytes());
        hasher.update(payload);

        for parent in parents {
            hasher.update(&parent.0);
        }

        Caid(*hasher.finalize().as_bytes())
    }

    pub fn validate_identity(&self) -> Result<(), RegistryError> {
        let expected = Self::derive_caid(self.node_type, &self.payload, &self.parents);

        if expected != self.caid {
            return Err(RegistryError::SchemaViolation);
        }

        Ok(())
    }

    pub const fn caid(&self) -> Caid {
        self.caid
    }

    pub const fn node_type(&self) -> RegistryNodeType {
        self.node_type
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn parents(&self) -> &[Caid] {
        &self.parents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_identity_is_deterministic() {
        let parent = Caid::from_payload(b"parent");

        let a = RegistryNode::new(
            RegistryNodeType::Capability,
            b"payload".to_vec(),
            vec![parent],
        )
        .unwrap();

        let b = RegistryNode::new(
            RegistryNodeType::Capability,
            b"payload".to_vec(),
            vec![parent],
        )
        .unwrap();

        assert_eq!(a.caid(), b.caid());
        assert!(a.validate_identity().is_ok());
    }

    #[test]
    fn node_type_changes_identity() {
        let capability =
            RegistryNode::new(RegistryNodeType::Capability, b"payload".to_vec(), vec![]).unwrap();

        let actor =
            RegistryNode::new(RegistryNodeType::Actor, b"payload".to_vec(), vec![]).unwrap();

        assert_ne!(capability.caid(), actor.caid());
    }

    #[test]
    fn parent_order_changes_identity() {
        let parent_a = Caid::from_payload(b"parent-a");
        let parent_b = Caid::from_payload(b"parent-b");

        let first = RegistryNode::new(
            RegistryNodeType::DataManifest,
            b"payload".to_vec(),
            vec![parent_a, parent_b],
        )
        .unwrap();

        let second = RegistryNode::new(
            RegistryNodeType::DataManifest,
            b"payload".to_vec(),
            vec![parent_b, parent_a],
        )
        .unwrap();

        assert_ne!(first.caid(), second.caid());
    }

    #[test]
    fn empty_payload_fails_schema_validation() {
        assert_eq!(
            RegistryNode::new(RegistryNodeType::Actor, vec![], vec![]).unwrap_err(),
            RegistryError::SchemaViolation
        );
    }
}
