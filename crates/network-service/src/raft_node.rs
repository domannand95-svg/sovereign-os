use crate::commit_index::CommitIndexManager;
use crate::state_machine::StateMachine;

#[derive(Debug)]
pub struct RaftNode {
    state_machine: StateMachine,
    commit_index: CommitIndexManager,
    snapshot_directory: String,
}

impl RaftNode {
    pub fn new(snapshot_directory: impl Into<String>, cluster_size: usize) -> Self {
        Self {
            state_machine: StateMachine::new(),
            commit_index: CommitIndexManager::new(cluster_size),
            snapshot_directory: snapshot_directory.into(),
        }
    }

    pub fn state_machine(&self) -> &StateMachine {
        &self.state_machine
    }

    pub fn commit_index(&self) -> &CommitIndexManager {
        &self.commit_index
    }

    pub fn snapshot_directory(&self) -> &str {
        &self.snapshot_directory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_raft_node() {
        let node = RaftNode::new("/tmp/raft-snapshots", 5);

        assert_eq!(node.commit_index().quorum(), 3);
        assert_eq!(node.snapshot_directory(), "/tmp/raft-snapshots");
    }

    #[test]
    fn state_machine_starts_empty() {
        let node = RaftNode::new("/tmp/raft-snapshots", 3);

        assert_eq!(node.state_machine().last_applied_index(), 0);
        assert!(node.state_machine().applied_entries().is_empty());
    }
}
