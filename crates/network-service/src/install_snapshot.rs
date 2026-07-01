use serde::{Deserialize, Serialize};

/// Snapshot installation request as defined by the Raft protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallSnapshotRequest {
    /// Current leader term.
    pub term: u64,

    /// Leader identifier.
    pub leader_id: String,

    /// Index of the last log entry included in the snapshot.
    pub last_included_index: u64,

    /// Term of the last log entry included in the snapshot.
    pub last_included_term: u64,

    /// Byte offset of this chunk.
    pub offset: u64,

    /// Raw snapshot bytes.
    pub data: Vec<u8>,

    /// True if this is the final chunk.
    pub done: bool,
}

/// Snapshot installation response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallSnapshotResponse {
    /// Current follower term.
    pub term: u64,

    /// Whether the chunk was accepted.
    pub success: bool,
}

/// Metadata describing a completed snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotMetadata {
    pub last_included_index: u64,
    pub last_included_term: u64,
}

/// Basic chunk accumulator used until streaming snapshot
/// installation is fully implemented.
#[derive(Debug, Default)]
pub struct SnapshotAssembler {
    buffer: Vec<u8>,
}

impl SnapshotAssembler {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn append(&mut self, request: &InstallSnapshotRequest) -> Option<Vec<u8>> {
        self.buffer.extend_from_slice(&request.data);

        if request.done {
            Some(std::mem::take(&mut self.buffer))
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_serializes_and_deserializes() {
        let request = InstallSnapshotRequest {
            term: 5,
            leader_id: "leader-a".to_string(),
            last_included_index: 128,
            last_included_term: 4,
            offset: 0,
            data: vec![1, 2, 3, 4],
            done: false,
        };

        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: InstallSnapshotRequest = serde_json::from_str(&encoded).unwrap();

        assert_eq!(request, decoded);
    }

    #[test]
    fn response_serializes_and_deserializes() {
        let response = InstallSnapshotResponse {
            term: 6,
            success: true,
        };

        let encoded = serde_json::to_string(&response).unwrap();
        let decoded: InstallSnapshotResponse = serde_json::from_str(&encoded).unwrap();

        assert_eq!(response, decoded);
    }

    #[test]
    fn assembler_collects_multiple_chunks() {
        let mut assembler = SnapshotAssembler::new();

        let first = InstallSnapshotRequest {
            term: 1,
            leader_id: "leader".into(),
            last_included_index: 32,
            last_included_term: 1,
            offset: 0,
            data: vec![1, 2, 3],
            done: false,
        };

        let second = InstallSnapshotRequest {
            term: 1,
            leader_id: "leader".into(),
            last_included_index: 32,
            last_included_term: 1,
            offset: 3,
            data: vec![4, 5, 6],
            done: true,
        };

        assert!(assembler.append(&first).is_none());

        let snapshot = assembler.append(&second).unwrap();

        assert_eq!(snapshot, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn assembler_clear_discards_buffer() {
        let mut assembler = SnapshotAssembler::new();

        assembler.buffer.extend_from_slice(&[9, 9, 9]);

        assembler.clear();

        assert!(assembler.buffer.is_empty());
    }
}
