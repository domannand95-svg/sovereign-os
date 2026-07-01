use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Follower,
    Candidate,
    Leader,
}

#[derive(Debug, Clone)]
pub struct ConsensusState {
    pub node_id: Uuid,
    pub current_term: u64,
    pub voted_for: Option<Uuid>,
    pub role: NodeRole,
    pub current_leader: Option<Uuid>,
    pub votes_received: HashSet<Uuid>,
    pub cluster_peers: HashSet<Uuid>,
}

impl ConsensusState {
    pub fn new(node_id: Uuid, peers: HashSet<Uuid>) -> Self {
        Self {
            node_id,
            current_term: 0,
            voted_for: None,
            role: NodeRole::Follower,
            current_leader: None,
            votes_received: HashSet::new(),
            cluster_peers: peers,
        }
    }

    pub fn start_election(&mut self) {
        self.role = NodeRole::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.node_id);
        self.votes_received.clear();
        self.votes_received.insert(self.node_id);
        self.current_leader = None;
    }

    pub fn register_vote(&mut self, voter_id: Uuid) -> bool {
        if self.role != NodeRole::Candidate {
            return false;
        }

        if self.cluster_peers.contains(&voter_id) || voter_id == self.node_id {
            self.votes_received.insert(voter_id);
        }

        let total_nodes = self.cluster_peers.len() + 1;
        let quorum = (total_nodes / 2) + 1;

        if self.votes_received.len() >= quorum {
            self.role = NodeRole::Leader;
            self.current_leader = Some(self.node_id);
            true
        } else {
            false
        }
    }

    pub fn handle_term_telemetry(&mut self, remote_term: u64, remote_leader_id: Uuid) -> bool {
        if remote_term > self.current_term {
            self.current_term = remote_term;
            self.role = NodeRole::Follower;
            self.voted_for = None;
            self.votes_received.clear();
            self.current_leader = Some(remote_leader_id);
            return true;
        }

        if remote_term == self.current_term && self.role == NodeRole::Candidate {
            self.role = NodeRole::Follower;
            self.current_leader = Some(remote_leader_id);
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_election_initiation_and_quorum_victory() {
        let node_self = Uuid::new_v4();
        let peer_a = Uuid::new_v4();
        let peer_b = Uuid::new_v4();

        let mut peers = HashSet::new();
        peers.insert(peer_a);
        peers.insert(peer_b);

        let mut state = ConsensusState::new(node_self, peers);
        assert_eq!(state.role, NodeRole::Follower);

        state.start_election();
        assert_eq!(state.role, NodeRole::Candidate);
        assert_eq!(state.current_term, 1);
        assert_eq!(state.voted_for, Some(node_self));

        let quorum_achieved = state.register_vote(peer_a);
        assert!(quorum_achieved);
        assert_eq!(state.role, NodeRole::Leader);
        assert_eq!(state.current_leader, Some(node_self));
    }

    #[test]
    fn test_stale_term_yield_demotion() {
        let node_self = Uuid::new_v4();
        let peer_leader = Uuid::new_v4();

        let mut state = ConsensusState::new(node_self, HashSet::new());
        state.role = NodeRole::Leader;
        state.current_term = 2;
        state.current_leader = Some(node_self);

        let demoted = state.handle_term_telemetry(3, peer_leader);
        assert!(demoted);
        assert_eq!(state.role, NodeRole::Follower);
        assert_eq!(state.current_term, 3);
        assert_eq!(state.current_leader, Some(peer_leader));
        assert_eq!(state.voted_for, None);
    }
}
