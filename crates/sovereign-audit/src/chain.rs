use crate::ledger::{AuditLedgerEntry, AuditLedgerError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditLedgerChainError {
    InvalidSequence { expected: u64, actual: u64 },
    PreviousDigestMismatch,
    EntryIntegrityFailure { sequence: u64 },
    Ledger(AuditLedgerError),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuditLedgerChain {
    entries: Vec<AuditLedgerEntry>,
}

impl AuditLedgerChain {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn head(&self) -> Option<&AuditLedgerEntry> {
        self.entries.last()
    }

    pub fn entries(&self) -> &[AuditLedgerEntry] {
        &self.entries
    }

    pub fn append(&mut self, entry: AuditLedgerEntry) -> Result<(), AuditLedgerChainError> {
        let expected_sequence = self.entries.len() as u64;

        if entry.sequence != expected_sequence {
            return Err(AuditLedgerChainError::InvalidSequence {
                expected: expected_sequence,
                actual: entry.sequence,
            });
        }

        if let Some(previous) = self.entries.last() {
            if entry.previous_entry_digest != previous.entry_digest {
                return Err(AuditLedgerChainError::PreviousDigestMismatch);
            }
        }

        if !entry.verify_integrity() {
            return Err(AuditLedgerChainError::EntryIntegrityFailure {
                sequence: entry.sequence,
            });
        }

        self.entries.push(entry);

        Ok(())
    }

    pub fn verify_chain(&self) -> Result<(), AuditLedgerChainError> {
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.sequence != index as u64 {
                return Err(AuditLedgerChainError::InvalidSequence {
                    expected: index as u64,
                    actual: entry.sequence,
                });
            }

            if !entry.verify_integrity() {
                return Err(AuditLedgerChainError::EntryIntegrityFailure {
                    sequence: entry.sequence,
                });
            }

            if index > 0 {
                let previous = &self.entries[index - 1];

                if entry.previous_entry_digest != previous.entry_digest {
                    return Err(AuditLedgerChainError::PreviousDigestMismatch);
                }
            }
        }

        Ok(())
    }
}
