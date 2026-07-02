use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppliedCommand {
    pub index: u64,
    pub term: u64,
    pub command: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct StateMachine {
    applied: Vec<AppliedCommand>,
    last_applied_index: u64,
}

impl StateMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_entry(&mut self, index: u64, term: u64, command: Vec<u8>) {
        self.last_applied_index = index;

        self.applied.push(AppliedCommand {
            index,
            term,
            command,
        });
    }

    pub fn last_applied_index(&self) -> u64 {
        self.last_applied_index
    }

    pub fn force_reset_applied(&mut self, index: u64) {
        self.last_applied_index = index;
    }

    pub fn applied_entries(&self) -> &[AppliedCommand] {
        &self.applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_single_entry() {
        let mut sm = StateMachine::new();

        sm.apply_entry(1, 1, b"set x=1".to_vec());

        assert_eq!(sm.last_applied_index(), 1);
        assert_eq!(sm.applied_entries().len(), 1);
        assert_eq!(sm.applied_entries()[0].command, b"set x=1".to_vec());
    }

    #[test]
    fn apply_multiple_entries() {
        let mut sm = StateMachine::new();

        sm.apply_entry(1, 1, b"a".to_vec());
        sm.apply_entry(2, 1, b"b".to_vec());
        sm.apply_entry(3, 2, b"c".to_vec());

        assert_eq!(sm.last_applied_index(), 3);
        assert_eq!(sm.applied_entries().len(), 3);

        assert_eq!(sm.applied_entries()[0].index, 1);
        assert_eq!(sm.applied_entries()[1].index, 2);
        assert_eq!(sm.applied_entries()[2].index, 3);

        assert_eq!(sm.applied_entries()[2].term, 2);
        assert_eq!(sm.applied_entries()[2].command, b"c".to_vec());
    }
}
