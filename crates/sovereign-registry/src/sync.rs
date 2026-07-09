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
