use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    pub index: u64,
    pub term: u64,
    pub command: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AppendEntriesRequest {
    pub term: u64,
    pub leader_id: Uuid,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AppendEntriesResponse {
    pub term: u64,
    pub follower_id: Uuid,
    pub success: bool,
    pub match_index: u64,
}

#[derive(Debug, Clone)]
pub struct ReplicationState {
    pub node_id: Uuid,
    pub log: Vec<LogEntry>,
    pub commit_index: u64,
    pub last_applied: u64,
    pub next_index: HashMap<Uuid, u64>,
    pub match_index: HashMap<Uuid, u64>,
}

impl ReplicationState {
    pub fn new(node_id: Uuid) -> Self {
        Self {
            node_id,
            log: Vec::new(),
            commit_index: 0,
            last_applied: 0,
            next_index: HashMap::new(),
            match_index: HashMap::new(),
        }
    }

    pub fn append_local_command(&mut self, term: u64, command: impl Into<String>) -> LogEntry {
        let entry = LogEntry {
            index: self.last_log_index() + 1,
            term,
            command: command.into(),
        };

        self.log.push(entry.clone());
        entry
    }

    pub fn last_log_index(&self) -> u64 {
        self.log.last().map(|entry| entry.index).unwrap_or(0)
    }

    pub fn last_log_term(&self) -> u64 {
        self.log.last().map(|entry| entry.term).unwrap_or(0)
    }

    pub fn entry_at(&self, index: u64) -> Option<&LogEntry> {
        if index == 0 {
            return None;
        }

        self.log.iter().find(|entry| entry.index == index)
    }

    pub fn initialize_leader_state(&mut self, peers: impl IntoIterator<Item = Uuid>) {
        let next = self.last_log_index() + 1;

        for peer in peers {
            self.next_index.insert(peer, next);
            self.match_index.insert(peer, 0);
        }
    }

    pub fn build_append_entries_request(
        &self,
        leader_id: Uuid,
        peer_id: Uuid,
        term: u64,
    ) -> AppendEntriesRequest {
        let next_index = self
            .next_index
            .get(&peer_id)
            .copied()
            .unwrap_or_else(|| self.last_log_index() + 1);

        let prev_log_index = next_index.saturating_sub(1);
        let prev_log_term = self
            .entry_at(prev_log_index)
            .map(|entry| entry.term)
            .unwrap_or(0);

        let entries = self
            .log
            .iter()
            .filter(|entry| entry.index >= next_index)
            .cloned()
            .collect();

        AppendEntriesRequest {
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit: self.commit_index,
        }
    }

    pub fn apply_append_entries(
        &mut self,
        request: AppendEntriesRequest,
        follower_id: Uuid,
    ) -> AppendEntriesResponse {
        if request.prev_log_index > 0 {
            match self.entry_at(request.prev_log_index) {
                Some(entry) if entry.term == request.prev_log_term => {}
                _ => {
                    return AppendEntriesResponse {
                        term: request.term,
                        follower_id,
                        success: false,
                        match_index: self.last_log_index(),
                    };
                }
            }
        }

        for incoming in request.entries {
            if let Some(existing_pos) = self
                .log
                .iter()
                .position(|entry| entry.index == incoming.index)
            {
                if self.log[existing_pos].term != incoming.term {
                    self.log.truncate(existing_pos);
                    self.log.push(incoming);
                }
            } else {
                self.log.push(incoming);
            }
        }

        if request.leader_commit > self.commit_index {
            self.commit_index = request.leader_commit.min(self.last_log_index());
        }

        AppendEntriesResponse {
            term: request.term,
            follower_id,
            success: true,
            match_index: self.last_log_index(),
        }
    }

    pub fn record_append_response(&mut self, peer_id: Uuid, response: AppendEntriesResponse) {
        if response.success {
            self.match_index.insert(peer_id, response.match_index);
            self.next_index.insert(peer_id, response.match_index + 1);
        } else {
            let current_next = self.next_index.get(&peer_id).copied().unwrap_or(1);
            self.next_index
                .insert(peer_id, current_next.saturating_sub(1).max(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_local_command_assigns_monotonic_indexes() {
        let node_id = Uuid::new_v4();
        let mut state = ReplicationState::new(node_id);

        let first = state.append_local_command(1, "allocate:a");
        let second = state.append_local_command(1, "allocate:b");

        assert_eq!(first.index, 1);
        assert_eq!(second.index, 2);
        assert_eq!(state.last_log_index(), 2);
        assert_eq!(state.last_log_term(), 1);
    }

    #[test]
    fn test_build_append_entries_request_for_peer() {
        let leader_id = Uuid::new_v4();
        let peer_id = Uuid::new_v4();
        let mut state = ReplicationState::new(leader_id);

        state.append_local_command(3, "cmd-1");
        state.append_local_command(3, "cmd-2");
        state.initialize_leader_state(vec![peer_id]);

        let request = state.build_append_entries_request(leader_id, peer_id, 3);

        assert_eq!(request.term, 3);
        assert_eq!(request.leader_id, leader_id);
        assert_eq!(request.prev_log_index, 2);
        assert_eq!(request.prev_log_term, 3);
        assert!(request.entries.is_empty());
    }

    #[test]
    fn test_apply_append_entries_successfully_replica_syncs() {
        let leader_id = Uuid::new_v4();
        let follower_id = Uuid::new_v4();

        let mut follower_state = ReplicationState::new(follower_id);

        let request = AppendEntriesRequest {
            term: 4,
            leader_id,
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![
                LogEntry {
                    index: 1,
                    term: 4,
                    command: "schedule:x".to_string(),
                },
                LogEntry {
                    index: 2,
                    term: 4,
                    command: "schedule:y".to_string(),
                },
            ],
            leader_commit: 2,
        };

        let response = follower_state.apply_append_entries(request, follower_id);

        assert!(response.success);
        assert_eq!(response.match_index, 2);
        assert_eq!(follower_state.last_log_index(), 2);
        assert_eq!(follower_state.commit_index, 2);
    }

    #[test]
    fn test_apply_append_entries_rejects_prev_log_mismatch() {
        let leader_id = Uuid::new_v4();
        let follower_id = Uuid::new_v4();

        let mut follower_state = ReplicationState::new(follower_id);
        follower_state.append_local_command(1, "old-entry");

        let request = AppendEntriesRequest {
            term: 2,
            leader_id,
            prev_log_index: 1,
            prev_log_term: 99,
            entries: vec![],
            leader_commit: 0,
        };

        let response = follower_state.apply_append_entries(request, follower_id);

        assert!(!response.success);
        assert_eq!(follower_state.last_log_index(), 1);
    }

    #[test]
    fn test_record_append_response_advances_peer_indexes() {
        let leader_id = Uuid::new_v4();
        let peer_id = Uuid::new_v4();

        let mut state = ReplicationState::new(leader_id);
        state.initialize_leader_state(vec![peer_id]);

        let response = AppendEntriesResponse {
            term: 1,
            follower_id: peer_id,
            success: true,
            match_index: 5,
        };

        state.record_append_response(peer_id, response);

        assert_eq!(state.match_index.get(&peer_id), Some(&5));
        assert_eq!(state.next_index.get(&peer_id), Some(&6));
    }
}
