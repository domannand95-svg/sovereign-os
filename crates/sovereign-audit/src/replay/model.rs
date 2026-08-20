use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceDigest(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReplayTimestamp(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceNode {
    pub id: NodeId,
    pub record_type: String,
    pub schema_version: SchemaVersion,
    pub digest: EvidenceDigest,
    pub parent_digest: Option<EvidenceDigest>,
    pub timestamp: ReplayTimestamp,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityState {
    Active,
    Suspended,
    Expired,
    Revoked,
    Inert,
    Unknown,
}
