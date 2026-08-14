use crate::{Caid, ObjectClass, RegistryEdge, RegistryError, RelationType};

pub const REGISTRY_NODE_MAGIC_V2: u8 = 0xFE;
pub const REGISTRY_EDGE_MAGIC_V2: u8 = 0xFD;
pub const REGISTRY_ENCODING_VERSION_V2: u16 = 0x0002;

const REGISTRY_NODE_DOMAIN_V2: &[u8] = b"SOVEREIGN_REGISTRY_NODE_V2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryGenesisPayloadV1 {
    environment_id: Vec<u8>,
    root_policy_caid: Option<Caid>,
}

impl RegistryGenesisPayloadV1 {
    pub fn decode(bytes: &[u8]) -> Result<Self, RegistryError> {
        if bytes.len() < 5 {
            return Err(RegistryError::MalformedGenesisPayload);
        }

        let environment_len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
        let environment_start = 2_usize;
        let environment_end = environment_start
            .checked_add(environment_len)
            .ok_or(RegistryError::MalformedGenesisPayload)?;

        let fixed_end = environment_end
            .checked_add(3)
            .ok_or(RegistryError::MalformedGenesisPayload)?;

        if bytes.len() < fixed_end {
            return Err(RegistryError::MalformedGenesisPayload);
        }

        let environment_id = &bytes[environment_start..environment_end];

        if std::str::from_utf8(environment_id).is_err() {
            return Err(RegistryError::MalformedGenesisPayload);
        }

        let protocol_version =
            u16::from_be_bytes([bytes[environment_end], bytes[environment_end + 1]]);

        if protocol_version != REGISTRY_ENCODING_VERSION_V2 {
            return Err(RegistryError::MalformedGenesisPayload);
        }

        let root_policy_present = bytes[environment_end + 2];

        let root_policy_caid = match root_policy_present {
            0x00 => {
                if bytes.len() != fixed_end {
                    return Err(RegistryError::MalformedGenesisPayload);
                }

                None
            }
            0x01 => {
                let expected_end = fixed_end
                    .checked_add(32)
                    .ok_or(RegistryError::MalformedGenesisPayload)?;

                if bytes.len() != expected_end {
                    return Err(RegistryError::MalformedGenesisPayload);
                }

                let mut caid = [0_u8; 32];
                caid.copy_from_slice(&bytes[fixed_end..expected_end]);
                Some(Caid(caid))
            }
            _ => return Err(RegistryError::MalformedGenesisPayload),
        };

        Ok(Self {
            environment_id: environment_id.to_vec(),
            root_policy_caid,
        })
    }

    pub fn environment_id(&self) -> &[u8] {
        &self.environment_id
    }

    pub const fn protocol_version(&self) -> u16 {
        REGISTRY_ENCODING_VERSION_V2
    }

    pub const fn root_policy_caid(&self) -> Option<Caid> {
        self.root_policy_caid
    }

    pub fn encode(&self) -> Vec<u8> {
        let environment_len = u16::try_from(self.environment_id.len())
            .expect("decoded Genesis environment length must remain representable as u16");

        let root_policy_len = if self.root_policy_caid.is_some() {
            32
        } else {
            0
        };
        let mut buffer = Vec::with_capacity(5 + self.environment_id.len() + root_policy_len);

        buffer.extend_from_slice(&environment_len.to_be_bytes());
        buffer.extend_from_slice(&self.environment_id);
        buffer.extend_from_slice(&REGISTRY_ENCODING_VERSION_V2.to_be_bytes());

        match self.root_policy_caid {
            Some(caid) => {
                buffer.push(0x01);
                buffer.extend_from_slice(&caid.0);
            }
            None => buffer.push(0x00),
        }

        buffer
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedRegistryNode {
    class: ObjectClass,
    parents: Vec<Caid>,
    payload: Vec<u8>,
    caid: Caid,
}

impl VersionedRegistryNode {
    pub fn new(
        class: ObjectClass,
        parents: Vec<Caid>,
        payload: Vec<u8>,
    ) -> Result<Self, RegistryError> {
        if payload.is_empty() {
            return Err(RegistryError::SchemaViolation);
        }

        let encoded = encode_node_parts(class, &parents, &payload)?;
        let caid = derive_v2_node_caid(&encoded);

        Ok(Self {
            class,
            parents,
            payload,
            caid,
        })
    }

    pub const fn class(&self) -> ObjectClass {
        self.class
    }

    pub fn parents(&self) -> &[Caid] {
        &self.parents
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub const fn caid(&self) -> Caid {
        self.caid
    }

    pub fn encode(&self) -> Vec<u8> {
        encode_node_parts(self.class, &self.parents, &self.payload)
            .expect("validated VersionedRegistryNode must remain encodable")
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, RegistryError> {
        if bytes.len() < 12 {
            return Err(RegistryError::SchemaViolation);
        }

        if bytes[0] != REGISTRY_NODE_MAGIC_V2 {
            return Err(RegistryError::SchemaViolation);
        }

        let version = u16::from_be_bytes([bytes[1], bytes[2]]);
        if version != REGISTRY_ENCODING_VERSION_V2 {
            return Err(RegistryError::SchemaViolation);
        }

        let class = ObjectClass::from_u8(bytes[3]).ok_or(RegistryError::SchemaViolation)?;

        let parent_count = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;

        let parents_len = parent_count
            .checked_mul(32)
            .ok_or(RegistryError::SchemaViolation)?;

        let payload_len_offset = 8_usize
            .checked_add(parents_len)
            .ok_or(RegistryError::SchemaViolation)?;

        let minimum_len = payload_len_offset
            .checked_add(4)
            .ok_or(RegistryError::SchemaViolation)?;

        if bytes.len() < minimum_len {
            return Err(RegistryError::SchemaViolation);
        }

        let mut parents = Vec::with_capacity(parent_count);
        let mut cursor: usize = 8;

        for _ in 0..parent_count {
            let end = cursor
                .checked_add(32)
                .ok_or(RegistryError::SchemaViolation)?;

            let mut caid = [0_u8; 32];
            caid.copy_from_slice(&bytes[cursor..end]);
            parents.push(Caid(caid));
            cursor = end;
        }

        let payload_len = u32::from_be_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;

        let expected_len = cursor
            .checked_add(payload_len)
            .ok_or(RegistryError::SchemaViolation)?;

        if bytes.len() != expected_len || payload_len == 0 {
            return Err(RegistryError::SchemaViolation);
        }

        Self::new(class, parents, bytes[cursor..expected_len].to_vec())
    }
}

pub fn serialize_edge_v2(edge: &RegistryEdge) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(68);

    buffer.push(REGISTRY_EDGE_MAGIC_V2);
    buffer.extend_from_slice(&REGISTRY_ENCODING_VERSION_V2.to_be_bytes());
    buffer.push(edge.relation().as_u8());
    buffer.extend_from_slice(&edge.parent().0);
    buffer.extend_from_slice(&edge.child().0);

    buffer
}

pub fn deserialize_edge_v2(bytes: &[u8]) -> Result<RegistryEdge, RegistryError> {
    if bytes.len() != 68 {
        return Err(RegistryError::SchemaViolation);
    }

    if bytes[0] != REGISTRY_EDGE_MAGIC_V2 {
        return Err(RegistryError::SchemaViolation);
    }

    let version = u16::from_be_bytes([bytes[1], bytes[2]]);
    if version != REGISTRY_ENCODING_VERSION_V2 {
        return Err(RegistryError::SchemaViolation);
    }

    let relation = RelationType::from_u8(bytes[3]).ok_or(RegistryError::SchemaViolation)?;

    let mut parent = [0_u8; 32];
    parent.copy_from_slice(&bytes[4..36]);

    let mut child = [0_u8; 32];
    child.copy_from_slice(&bytes[36..68]);

    Ok(RegistryEdge::new(Caid(parent), Caid(child), relation))
}

fn encode_node_parts(
    class: ObjectClass,
    parents: &[Caid],
    payload: &[u8],
) -> Result<Vec<u8>, RegistryError> {
    if payload.is_empty() {
        return Err(RegistryError::SchemaViolation);
    }

    let parent_count = u32::try_from(parents.len()).map_err(|_| RegistryError::SchemaViolation)?;
    let payload_len = u32::try_from(payload.len()).map_err(|_| RegistryError::SchemaViolation)?;

    let parents_bytes = parents
        .len()
        .checked_mul(32)
        .ok_or(RegistryError::SchemaViolation)?;

    let capacity = 12_usize
        .checked_add(parents_bytes)
        .and_then(|value| value.checked_add(payload.len()))
        .ok_or(RegistryError::SchemaViolation)?;

    let mut buffer = Vec::with_capacity(capacity);

    buffer.push(REGISTRY_NODE_MAGIC_V2);
    buffer.extend_from_slice(&REGISTRY_ENCODING_VERSION_V2.to_be_bytes());
    buffer.push(class.as_u8());
    buffer.extend_from_slice(&parent_count.to_be_bytes());

    for parent in parents {
        buffer.extend_from_slice(&parent.0);
    }

    buffer.extend_from_slice(&payload_len.to_be_bytes());
    buffer.extend_from_slice(payload);

    Ok(buffer)
}

fn derive_v2_node_caid(encoded: &[u8]) -> Caid {
    let mut hasher = blake3::Hasher::new();
    hasher.update(REGISTRY_NODE_DOMAIN_V2);
    hasher.update(encoded);
    Caid(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2_node_fixed_encoding_and_identity_vector() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Specification,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let mut expected = Vec::new();
        expected.push(0xFE);
        expected.extend_from_slice(&0x0002_u16.to_be_bytes());
        expected.push(0x02);
        expected.extend_from_slice(&1_u32.to_be_bytes());
        expected.extend_from_slice(&[0xAA; 32]);
        expected.extend_from_slice(&1_u32.to_be_bytes());
        expected.push(0xFF);

        assert_eq!(expected.len(), 45);
        assert_eq!(node.encode(), expected);

        let expected_caid = Caid([
            0x8D, 0x3D, 0x7A, 0x43, 0xB7, 0xD6, 0x4D, 0x83, 0x2F, 0x16, 0x9B, 0x4D, 0x0D, 0x45,
            0x12, 0xBF, 0x79, 0x89, 0x54, 0x7F, 0xDB, 0x0A, 0xD5, 0xDA, 0x3A, 0xBB, 0x53, 0x75,
            0x16, 0x49, 0x12, 0x5D,
        ]);

        assert_eq!(node.caid(), expected_caid);
    }

    #[test]
    fn v2_node_round_trip_is_byte_identical() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Workflow,
            vec![Caid([0x11; 32]), Caid([0x22; 32])],
            b"deterministic-evaluation-workflow".to_vec(),
        )
        .unwrap();

        let encoded = node.encode();
        let decoded = VersionedRegistryNode::decode(&encoded).unwrap();

        assert_eq!(decoded, node);
        assert_eq!(decoded.encode(), encoded);
        assert_eq!(decoded.caid(), node.caid());
    }

    #[test]
    fn v2_edge_fixed_vector_is_exactly_68_bytes() {
        let edge = RegistryEdge::new(
            Caid([0xAA; 32]),
            Caid([0xBB; 32]),
            RelationType::DerivedFrom,
        );

        let encoded = serialize_edge_v2(&edge);

        let mut expected = Vec::new();
        expected.push(0xFD);
        expected.extend_from_slice(&0x0002_u16.to_be_bytes());
        expected.push(0x01);
        expected.extend_from_slice(&[0xAA; 32]);
        expected.extend_from_slice(&[0xBB; 32]);

        assert_eq!(encoded.len(), 68);
        assert_eq!(encoded, expected);
        assert_eq!(deserialize_edge_v2(&encoded).unwrap(), edge);
    }

    #[test]
    fn node_and_edge_magic_are_strictly_isolated() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Specification,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let edge = RegistryEdge::new(
            Caid([0xAA; 32]),
            Caid([0xBB; 32]),
            RelationType::DerivedFrom,
        );

        assert_eq!(
            VersionedRegistryNode::decode(&serialize_edge_v2(&edge)),
            Err(RegistryError::SchemaViolation)
        );

        assert_eq!(
            deserialize_edge_v2(&node.encode()),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn unsupported_v2_version_fails_closed() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Specification,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let mut encoded = node.encode();
        encoded[1] = 0x00;
        encoded[2] = 0x03;

        assert_eq!(
            VersionedRegistryNode::decode(&encoded),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn unknown_object_class_fails_closed() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Specification,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let mut encoded = node.encode();
        encoded[3] = 0x0A;

        assert_eq!(
            VersionedRegistryNode::decode(&encoded),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn unsupported_edge_v2_version_fails_closed() {
        let edge = RegistryEdge::new(
            Caid([0xAA; 32]),
            Caid([0xBB; 32]),
            RelationType::DerivedFrom,
        );

        let mut encoded = serialize_edge_v2(&edge);
        encoded[1] = 0x00;
        encoded[2] = 0x03;

        assert_eq!(
            deserialize_edge_v2(&encoded),
            Err(RegistryError::SchemaViolation)
        );
    }
    #[test]
    fn unknown_relation_type_fails_closed() {
        let edge = RegistryEdge::new(
            Caid([0xAA; 32]),
            Caid([0xBB; 32]),
            RelationType::DerivedFrom,
        );

        let mut encoded = serialize_edge_v2(&edge);
        encoded[3] = 0x08;

        assert_eq!(
            deserialize_edge_v2(&encoded),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn v2_node_rejects_truncation_and_trailing_bytes() {
        let node = VersionedRegistryNode::new(
            ObjectClass::EvidencePackage,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let encoded = node.encode();

        assert_eq!(
            VersionedRegistryNode::decode(&encoded[..encoded.len() - 1]),
            Err(RegistryError::SchemaViolation)
        );

        let mut trailing = encoded;
        trailing.push(0x00);

        assert_eq!(
            VersionedRegistryNode::decode(&trailing),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn adding_edge_does_not_change_endpoint_node_caids() {
        let parent = VersionedRegistryNode::new(
            ObjectClass::Dataset,
            vec![Caid([0x11; 32])],
            b"dataset".to_vec(),
        )
        .unwrap();

        let child = VersionedRegistryNode::new(
            ObjectClass::Workflow,
            vec![parent.caid()],
            b"workflow".to_vec(),
        )
        .unwrap();

        let parent_before = parent.caid();
        let child_before = child.caid();

        let _edge = RegistryEdge::new(parent.caid(), child.caid(), RelationType::Consumes);

        assert_eq!(parent.caid(), parent_before);
        assert_eq!(child.caid(), child_before);
    }
}
