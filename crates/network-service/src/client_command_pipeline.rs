use crate::raft_service::RaftService;

pub struct ClientCommandPipeline {
    service: RaftService,
}

impl ClientCommandPipeline {
    pub fn new(service: RaftService) -> Self {
        Self { service }
    }

    pub fn submit(&mut self, _command: Vec<u8>) {
        self.service.append_entries();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft_node::RaftNode;
    use crate::raft_service::RaftService;

    #[test]
    fn submits_command() {
        let node = RaftNode::new("/tmp/raft-pipeline", 3);
        let service = RaftService::new(node);

        let mut pipeline = ClientCommandPipeline::new(service);
        pipeline.submit(b"set x=1".to_vec());
    }
}
