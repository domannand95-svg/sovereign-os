use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusRole {
    Follower,
    Candidate,
    Leader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub index: u64,
    pub term: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusState {
    pub current_term: u64,
    pub voted_for: Option<[u8; 16]>,
    pub commit_index: u64,
    pub last_applied: u64,
    pub role: ConsensusRole,
}

impl ConsensusState {
    pub fn new() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
            commit_index: 0,
            last_applied: 0,
            role: ConsensusRole::Follower,
        }
    }

    pub fn step_down(&mut self, new_term: u64) {
        if new_term <= self.current_term {
            return;
        }

        self.current_term = new_term;
        self.voted_for = None;
        self.role = ConsensusRole::Follower;
    }

    pub fn validate_append_request(
        &self,
        prev_log_index: u64,
        prev_log_term: u64,
        current_log: &[LogEntry],
    ) -> bool {
        if prev_log_index == 0 {
            return true;
        }

        current_log
            .iter()
            .any(|entry| entry.index == prev_log_index && entry.term == prev_log_term)
    }

    pub fn update_commit_boundary(
        &mut self,
        leader_commit: u64,
        last_new_entry_index: u64,
    ) -> u64 {
        let next_commit = leader_commit.min(last_new_entry_index);

        if next_commit > self.commit_index {
            self.commit_index = next_commit;
        }

        self.commit_index
    }
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: u64, term: u64) -> LogEntry {
        LogEntry {
            index,
            term,
            payload: vec![index as u8, term as u8],
        }
    }

    #[test]
    fn new_state_starts_as_follower() {
        let state = ConsensusState::new();

        assert_eq!(state.current_term, 0);
        assert_eq!(state.voted_for, None);
        assert_eq!(state.commit_index, 0);
        assert_eq!(state.last_applied, 0);
        assert_eq!(state.role, ConsensusRole::Follower);
    }

    #[test]
    fn step_down_ignores_stale_terms() {
        let mut state = ConsensusState {
            current_term: 5,
            voted_for: Some([1; 16]),
            commit_index: 0,
            last_applied: 0,
            role: ConsensusRole::Leader,
        };

        state.step_down(5);

        assert_eq!(state.current_term, 5);
        assert_eq!(state.voted_for, Some([1; 16]));
        assert_eq!(state.role, ConsensusRole::Leader);
    }

    #[test]
    fn step_down_accepts_newer_term() {
        let mut state = ConsensusState {
            current_term: 5,
            voted_for: Some([1; 16]),
            commit_index: 0,
            last_applied: 0,
            role: ConsensusRole::Leader,
        };

        state.step_down(6);

        assert_eq!(state.current_term, 6);
        assert_eq!(state.voted_for, None);
        assert_eq!(state.role, ConsensusRole::Follower);
    }

    #[test]
    fn append_request_accepts_genesis_boundary() {
        let state = ConsensusState::new();

        assert!(state.validate_append_request(0, 0, &[]));
    }

    #[test]
    fn append_request_accepts_matching_previous_entry() {
        let state = ConsensusState::new();
        let log = vec![entry(1, 1), entry(2, 1)];

        assert!(state.validate_append_request(2, 1, &log));
    }

    #[test]
    fn append_request_rejects_missing_previous_entry() {
        let state = ConsensusState::new();
        let log = vec![entry(1, 1)];

        assert!(!state.validate_append_request(2, 1, &log));
    }

    #[test]
    fn append_request_rejects_term_mismatch() {
        let state = ConsensusState::new();
        let log = vec![entry(1, 1), entry(2, 2)];

        assert!(!state.validate_append_request(2, 1, &log));
    }

    #[test]
    fn commit_boundary_advances_to_minimum_safe_index() {
        let mut state = ConsensusState::new();

        let commit = state.update_commit_boundary(10, 7);

        assert_eq!(commit, 7);
        assert_eq!(state.commit_index, 7);
    }

    #[test]
    fn commit_boundary_never_moves_backward() {
        let mut state = ConsensusState::new();
        state.commit_index = 8;

        let commit = state.update_commit_boundary(5, 5);

        assert_eq!(commit, 8);
        assert_eq!(state.commit_index, 8);
    }
}
