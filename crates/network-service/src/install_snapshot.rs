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

/// Aligns local Raft progress markers after a completed snapshot has been applied.
pub fn apply_snapshot_indices(
    commit_index: &mut crate::commit_index::CommitIndexManager,
    state_machine: &mut crate::state_machine::StateMachine,
    snapshot_index: u64,
) {
    commit_index.force_reset(snapshot_index);
    state_machine.force_reset_applied(snapshot_index);
}

#[cfg(test)]
mod tests {

    #[test]
    fn handler_replicates_chunked_snapshot_to_multiple_followers() {
        let first = InstallSnapshotRequest {
            term: 4,
            leader_id: "leader-a".to_string(),
            last_included_index: 256,
            last_included_term: 4,
            offset: 0,
            data: vec![1, 2, 3],
            done: false,
        };

        let second = InstallSnapshotRequest {
            term: 4,
            leader_id: "leader-a".to_string(),
            last_included_index: 256,
            last_included_term: 4,
            offset: 3,
            data: vec![4, 5, 6],
            done: true,
        };

        let mut follower_a = InstallSnapshotHandler::new();
        let mut follower_b = InstallSnapshotHandler::new();

        let (a_first_response, a_first_snapshot) = follower_a.handle(&first, 4);
        let (b_first_response, b_first_snapshot) = follower_b.handle(&first, 4);

        assert!(a_first_response.success);
        assert!(b_first_response.success);
        assert!(a_first_snapshot.is_none());
        assert!(b_first_snapshot.is_none());

        let (a_second_response, a_snapshot) = follower_a.handle(&second, 4);
        let (b_second_response, b_snapshot) = follower_b.handle(&second, 4);

        assert!(a_second_response.success);
        assert!(b_second_response.success);
        assert_eq!(a_snapshot, Some(vec![1, 2, 3, 4, 5, 6]));
        assert_eq!(b_snapshot, Some(vec![1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn handler_replicates_snapshot_across_multiple_followers() {
        let request = InstallSnapshotRequest {
            term: 3,
            leader_id: "leader-a".to_string(),
            last_included_index: 64,
            last_included_term: 3,
            offset: 0,
            data: vec![10, 20, 30, 40],
            done: true,
        };

        let mut follower_a = InstallSnapshotHandler::new();
        let mut follower_b = InstallSnapshotHandler::new();

        let (response_a, snapshot_a) = follower_a.handle(&request, 3);
        let (response_b, snapshot_b) = follower_b.handle(&request, 3);

        assert!(response_a.success);
        assert!(response_b.success);
        assert_eq!(snapshot_a, Some(vec![10, 20, 30, 40]));
        assert_eq!(snapshot_b, Some(vec![10, 20, 30, 40]));
    }

    #[test]
    fn apply_snapshot_accepts_non_empty_payload() {
        let mut handler = InstallSnapshotHandler::new();

        assert!(handler.apply_snapshot(vec![1, 2, 3]).is_ok());
    }

    #[test]
    fn apply_snapshot_rejects_empty_payload() {
        let mut handler = InstallSnapshotHandler::new();

        assert!(handler.apply_snapshot(Vec::new()).is_err());
    }

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
    fn handler_returns_snapshot_after_final_chunk() {
        let mut handler = InstallSnapshotHandler::new();

        let first = InstallSnapshotRequest {
            term: 2,
            leader_id: "leader".into(),
            last_included_index: 10,
            last_included_term: 2,
            offset: 0,
            data: vec![1, 2, 3],
            done: false,
        };

        let second = InstallSnapshotRequest {
            term: 2,
            leader_id: "leader".into(),
            last_included_index: 10,
            last_included_term: 2,
            offset: 3,
            data: vec![4, 5, 6],
            done: true,
        };

        let (_, snapshot) = handler.handle(&first, 1);
        assert!(snapshot.is_none());

        let (_, snapshot) = handler.handle(&second, 1);
        assert_eq!(snapshot.unwrap(), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn assembler_clear_discards_buffer() {
        let mut assembler = SnapshotAssembler::new();

        assembler.buffer.extend_from_slice(&[9, 9, 9]);

        assembler.clear();

        assert!(assembler.buffer.is_empty());
    }
}

/// Handles InstallSnapshot request validation and chunk assembly.
pub struct InstallSnapshotHandler {
    assembler: SnapshotAssembler,
}

impl InstallSnapshotHandler {
    pub fn new() -> Self {
        Self {
            assembler: SnapshotAssembler::new(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: Vec<u8>) -> Result<(), String> {
        if snapshot.is_empty() {
            return Err("snapshot payload is empty".to_string());
        }

        Ok(())
    }

    pub fn handle(
        &mut self,
        request: &InstallSnapshotRequest,
        current_term: u64,
    ) -> (InstallSnapshotResponse, Option<Vec<u8>>) {
        if request.term < current_term {
            return (
                InstallSnapshotResponse {
                    term: current_term,
                    success: false,
                },
                None,
            );
        }

        let completed_snapshot = self.assembler.append(request);

        (
            InstallSnapshotResponse {
                term: request.term,
                success: true,
            },
            completed_snapshot,
        )
    }
}
