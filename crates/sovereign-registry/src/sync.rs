use crate::{Caid, RegistryError, RegistryGraph, RegistryNode, RegistryNodeType};
use sovereign_ledger::{EventRecord, EventType};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RegistryLedgerSync;

impl RegistryLedgerSync {
    pub fn serialize_node(node: &RegistryNode) -> Vec<u8> {
        let mut buffer = Vec::new();

        buffer.push(node.node_type().as_u8());

        let parents = node.parents();
        buffer.extend_from_slice(&(parents.len() as u32).to_be_bytes());

        for parent in parents {
            buffer.extend_from_slice(&parent.0);
        }

        buffer.extend_from_slice(&(node.payload().len() as u32).to_be_bytes());
        buffer.extend_from_slice(node.payload());

        buffer
    }

    pub fn deserialize_node(bytes: &[u8]) -> Result<RegistryNode, RegistryError> {
        if bytes.len() < 9 {
            return Err(RegistryError::SchemaViolation);
        }

        let node_type = match bytes[0] {
            0x01 => RegistryNodeType::Capability,
            0x02 => RegistryNodeType::Actor,
            0x03 => RegistryNodeType::DataManifest,
            _ => return Err(RegistryError::SchemaViolation),
        };

        let mut count_bytes = [0_u8; 4];
        count_bytes.copy_from_slice(&bytes[1..5]);
        let parent_count = u32::from_be_bytes(count_bytes) as usize;

        let min_len = 5 + parent_count * 32 + 4;
        if bytes.len() < min_len {
            return Err(RegistryError::SchemaViolation);
        }

        let mut cursor = 5;
        let mut parents = Vec::with_capacity(parent_count);

        for _ in 0..parent_count {
            let mut caid = [0_u8; 32];
            caid.copy_from_slice(&bytes[cursor..cursor + 32]);
            parents.push(Caid(caid));
            cursor += 32;
        }

        let mut payload_len_bytes = [0_u8; 4];
        payload_len_bytes.copy_from_slice(&bytes[cursor..cursor + 4]);
        cursor += 4;

        let payload_len = u32::from_be_bytes(payload_len_bytes) as usize;

        if bytes.len() != cursor + payload_len {
            return Err(RegistryError::SchemaViolation);
        }

        RegistryNode::new(
            node_type,
            bytes[cursor..cursor + payload_len].to_vec(),
            parents,
        )
    }

    pub fn ingest_record(
        graph: &mut RegistryGraph,
        record: &EventRecord<'_>,
    ) -> Result<(), RegistryError> {
        match record.event_type {
            EventType::RegistryMutation | EventType::CapabilityPromotion => {
                let node = Self::deserialize_node(record.payload)?;
                graph.insert_node(node)
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sovereign_ledger::Lsn;

    #[test]
    fn node_wire_format_symmetry() {
        let node = RegistryNode::new(
            RegistryNodeType::Capability,
            b"isolated_firewall_gate_control_vector".to_vec(),
            vec![],
        )
        .unwrap();

        let serialized = RegistryLedgerSync::serialize_node(&node);
        let deserialized = RegistryLedgerSync::deserialize_node(&serialized).unwrap();

        assert_eq!(node.caid(), deserialized.caid());
        assert_eq!(node.node_type(), deserialized.node_type());
        assert_eq!(node.parents(), deserialized.parents());
        assert_eq!(node.payload(), deserialized.payload());
    }

    #[test]
    fn legacy_v1_capability_fixed_wire_vector() {
        let parent = Caid([0xAA; 32]);
        let node =
            RegistryNode::new(RegistryNodeType::Capability, vec![0xFF], vec![parent]).unwrap();

        let encoded = RegistryLedgerSync::serialize_node(&node);

        let mut expected = Vec::new();
        expected.push(0x01);
        expected.extend_from_slice(&1_u32.to_be_bytes());
        expected.extend_from_slice(&[0xAA; 32]);
        expected.extend_from_slice(&1_u32.to_be_bytes());
        expected.push(0xFF);

        assert_eq!(encoded.len(), 42);
        assert_eq!(encoded, expected);
    }

    #[test]
    fn legacy_v1_capability_fixed_caid_vector() {
        let node = RegistryNode::new(
            RegistryNodeType::Capability,
            vec![0xFF],
            vec![Caid([0xAA; 32])],
        )
        .unwrap();

        let expected = Caid([
            0xB6, 0xDB, 0x69, 0x14, 0xF7, 0x80, 0x92, 0x98, 0x0C, 0x96, 0x23, 0x61, 0xC8, 0x7B,
            0x21, 0xD9, 0xE3, 0xA6, 0x37, 0x15, 0x41, 0xD9, 0xD1, 0xD2, 0x76, 0x17, 0x91, 0x68,
            0x75, 0x90, 0xCB, 0x46,
        ]);

        assert_eq!(node.caid(), expected);
    }

    #[test]
    fn legacy_v1_round_trip_is_byte_identical() {
        let node = RegistryNode::new(
            RegistryNodeType::Capability,
            vec![0xFF],
            vec![Caid([0xAA; 32])],
        )
        .unwrap();

        let original = RegistryLedgerSync::serialize_node(&node);
        let decoded = RegistryLedgerSync::deserialize_node(&original).unwrap();
        let reencoded = RegistryLedgerSync::serialize_node(&decoded);

        assert_eq!(reencoded, original);
        assert_eq!(decoded.caid(), node.caid());
    }

    #[test]
    fn legacy_v1_type_discriminators_remain_frozen() {
        let cases = [
            (0x01, RegistryNodeType::Capability),
            (0x02, RegistryNodeType::Actor),
            (0x03, RegistryNodeType::DataManifest),
        ];

        for (tag, expected_type) in cases {
            let mut bytes = Vec::new();
            bytes.push(tag);
            bytes.extend_from_slice(&0_u32.to_be_bytes());
            bytes.extend_from_slice(&1_u32.to_be_bytes());
            bytes.push(0xFF);

            let node = RegistryLedgerSync::deserialize_node(&bytes).unwrap();
            assert_eq!(node.node_type(), expected_type);
        }
    }

    #[test]
    fn ingest_spurious_record_types_skipped_safely() {
        let mut graph = RegistryGraph::new();

        let record = EventRecord {
            lsn: Lsn(0),
            event_type: EventType::KernelDirective,
            payload: b"raw_hardware_register_reboot_vector",
            checksum: 12345,
        };

        assert!(RegistryLedgerSync::ingest_record(&mut graph, &record).is_ok());
        assert_eq!(graph.len(), 0);
    }

    #[test]
    fn malformed_registry_payload_fails_closed() {
        let mut graph = RegistryGraph::new();

        let record = EventRecord {
            lsn: Lsn(1),
            event_type: EventType::RegistryMutation,
            payload: b"invalid_truncated_garbage_bytes",
            checksum: 54321,
        };

        assert_eq!(
            RegistryLedgerSync::ingest_record(&mut graph, &record),
            Err(RegistryError::SchemaViolation)
        );
    }
}
