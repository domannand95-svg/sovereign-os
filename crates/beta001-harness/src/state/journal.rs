//! ADAM-012-A: Copy-on-Write Transactional State Journal
//!
//! Provides isolated in-memory transaction staging with prepare, discard, and atomic apply semantics.

use serde::{Deserialize, Serialize};

use super::mutation::{compute_delta_digest, normalize_mutations, StateMutation};
use super::tree::StateTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalStageStatus {
    Open,
    Prepared,
    Committed,
    RolledBack,
}

/// Transactional staging journal isolating mutations from committed state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateJournal {
    staged_mutations: Vec<StateMutation>,
    status: JournalStageStatus,
}

impl Default for StateJournal {
    fn default() -> Self {
        Self::new()
    }
}

impl StateJournal {
    /// Opens a new transaction staging journal.
    pub fn new() -> Self {
        Self {
            staged_mutations: Vec::new(),
            status: JournalStageStatus::Open,
        }
    }

    pub fn status(&self) -> JournalStageStatus {
        self.status
    }

    pub fn staged_len(&self) -> usize {
        self.staged_mutations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.staged_mutations.is_empty()
    }

    /// Appends a mutation to the staging buffer.
    pub fn stage(&mut self, mutation: StateMutation) -> Result<(), &'static str> {
        if self.status != JournalStageStatus::Open {
            return Err("Cannot stage mutations to a non-open transaction journal");
        }
        self.staged_mutations.push(mutation);
        Ok(())
    }

    /// Appends multiple mutations to the staging buffer.
    pub fn stage_batch(
        &mut self,
        mutations: impl IntoIterator<Item = StateMutation>,
    ) -> Result<(), &'static str> {
        if self.status != JournalStageStatus::Open {
            return Err("Cannot stage mutations to a non-open transaction journal");
        }
        self.staged_mutations.extend(mutations);
        Ok(())
    }

    /// Normalizes staged mutations and computes delta digest, preparing the journal for commit.
    pub fn prepare(&mut self) -> Result<(Vec<StateMutation>, String), &'static str> {
        if self.status != JournalStageStatus::Open {
            return Err("Journal must be in Open state to prepare");
        }
        let normalized = normalize_mutations(&self.staged_mutations);
        let delta_digest = compute_delta_digest(&normalized);
        self.status = JournalStageStatus::Prepared;
        Ok((normalized, delta_digest))
    }

    /// Discards all staged mutations without applying them to the state tree.
    pub fn discard(&mut self) {
        self.staged_mutations.clear();
        self.status = JournalStageStatus::RolledBack;
    }

    /// Applies normalized staged mutations to the target state tree atomically.
    /// Returns true if an effective change was committed.
    pub fn apply(&mut self, tree: &mut StateTree) -> Result<bool, &'static str> {
        let (normalized, _) = match self.status {
            JournalStageStatus::Open => self.prepare()?,
            JournalStageStatus::Prepared => {
                let normalized = normalize_mutations(&self.staged_mutations);
                let delta_digest = compute_delta_digest(&normalized);
                (normalized, delta_digest)
            }
            JournalStageStatus::Committed | JournalStageStatus::RolledBack => {
                return Err("Cannot apply a closed transaction journal");
            }
        };

        let changed = tree.apply_normalized_mutations(&normalized);
        self.status = JournalStageStatus::Committed;
        Ok(changed)
    }
}
