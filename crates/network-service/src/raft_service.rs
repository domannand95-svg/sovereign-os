use crate::append_entries_handler::AppendEntriesHandler;
use crate::raft_node::RaftNode;
use crate::request_vote::RequestVoteHandler;

pub struct RaftService {
    pub node: RaftNode,
}

impl RaftService {
    pub fn new(node: RaftNode) -> Self {
        Self { node }
    }

    pub fn append_entries(&mut self) {
        let _ = &AppendEntriesHandler;
    }

    pub fn request_vote(&mut self) {
        let _ = &RequestVoteHandler;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft_node::RaftNode;

    #[test]
    fn service_constructs() {
        let node = RaftNode::new("/tmp/raft-snapshots", 3);
        let _service = RaftService::new(node);
    }
}
