use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CommitIndexManager {
    commit_index: u64,
    total_nodes: usize,
}

impl CommitIndexManager {
    pub fn new(total_nodes: usize) -> Self {
        Self {
            commit_index: 0,
            total_nodes,
        }
    }

    pub fn commit_index(&self) -> u64 {
        self.commit_index
    }

    pub fn quorum(&self) -> usize {
        (self.total_nodes / 2) + 1
    }

    pub fn advance_commit_index(
        &mut self,
        leader_last_index: u64,
        match_indexes: &HashMap<Uuid, u64>,
    ) -> u64 {
        let quorum = self.quorum();

        for candidate_index in (self.commit_index + 1..=leader_last_index).rev() {
            let replicated_count = match_indexes
                .values()
                .filter(|&&match_index| match_index >= candidate_index)
                .count()
                + 1;

            if replicated_count >= quorum {
                self.commit_index = candidate_index;
                break;
            }
        }

        self.commit_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quorum_is_calculated_correctly() {
        let manager = CommitIndexManager::new(5);
        assert_eq!(manager.quorum(), 3);
    }

    #[test]
    fn advances_commit_index_when_majority_replicated() {
        let mut manager = CommitIndexManager::new(5);

        let mut match_indexes = HashMap::new();
        match_indexes.insert(Uuid::new_v4(), 5);
        match_indexes.insert(Uuid::new_v4(), 5);
        match_indexes.insert(Uuid::new_v4(), 4);
        match_indexes.insert(Uuid::new_v4(), 2);

        assert_eq!(manager.advance_commit_index(5, &match_indexes), 5);
        assert_eq!(manager.commit_index(), 5);
    }

    #[test]
    fn does_not_advance_without_quorum() {
        let mut manager = CommitIndexManager::new(5);

        let mut match_indexes = HashMap::new();
        match_indexes.insert(Uuid::new_v4(), 2);
        match_indexes.insert(Uuid::new_v4(), 2);
        match_indexes.insert(Uuid::new_v4(), 1);
        match_indexes.insert(Uuid::new_v4(), 1);

        assert_eq!(manager.advance_commit_index(5, &match_indexes), 2);
    }
}
