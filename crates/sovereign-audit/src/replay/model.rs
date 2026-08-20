#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceDigest(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReplayTimestamp(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityState {
    Active,
    Suspended,
    Expired,
    Revoked,
    Inert,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceNode {
    pub id: NodeId,
    pub record_type: String,
    pub schema_version: SchemaVersion,
    pub digest: EvidenceDigest,
    pub parent_digest: Option<EvidenceDigest>,
    pub timestamp: ReplayTimestamp,
    pub payload: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_node_preserves_identity() {
        let node = EvidenceNode {
            id: NodeId("node_001".into()),
            record_type: "EFFECT_PROPOSAL-v1".into(),
            schema_version: SchemaVersion("EFFECT_PROPOSAL-v1".into()),
            digest: EvidenceDigest("dig_001".into()),
            parent_digest: None,
            timestamp: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
            payload: serde_json::json!({}),
        };
        assert_eq!(node.id.0, "node_001");
        assert_eq!(node.schema_version.0, "EFFECT_PROPOSAL-v1");
    }
}
