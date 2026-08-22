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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{AgentIdentityId, Digest};
    use crate::ledger::AuditEventType;

    fn genesis_digest() -> Digest {
        Digest(hex::encode([0u8; 32]))
    }

    fn sample_entry(sequence: u64, previous: &Digest) -> AuditLedgerEntry {
        let event = AuditEventType::ExecutionCommitted;
        let subject = Digest("subject".into());
        let payload = Digest("payload".into());
        let recorded_at = "2026-03-07T00:00:00Z";
        let recorded_by = AgentIdentityId("agent-1".into());

        let entry_digest = AuditLedgerEntry::derive_digest(
            sequence,
            previous,
            &event,
            &subject,
            &payload,
            recorded_at,
            &recorded_by,
        );

        AuditLedgerEntry {
            sequence,
            previous_entry_digest: previous.clone(),
            event_type: event,
            subject_digest: subject,
            payload_digest: payload,
            recorded_at: recorded_at.into(),
            recorded_by,
            entry_digest,
        }
    }

    #[test]
    fn empty_chain_verifies() {
        let chain = AuditLedgerChain::new();

        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        assert!(chain.verify_chain().is_ok());
    }

    #[test]
    fn valid_first_entry_appends() {
        let mut chain = AuditLedgerChain::new();

        let entry = sample_entry(0, &genesis_digest());

        assert!(chain.append(entry).is_ok());
        assert_eq!(chain.len(), 1);
        assert!(chain.verify_chain().is_ok());
    }

    #[test]
    fn sequence_gap_is_rejected() {
        let mut chain = AuditLedgerChain::new();

        let entry = sample_entry(1, &genesis_digest());

        assert!(matches!(
            chain.append(entry),
            Err(AuditLedgerChainError::InvalidSequence {
                expected: 0,
                actual: 1
            })
        ));
    }

    #[test]
    fn wrong_previous_digest_is_rejected() {
        let mut chain = AuditLedgerChain::new();

        let entry = sample_entry(0, &Digest("wrong".into()));

        assert!(chain.append(entry).is_ok());
    }

    #[test]
    fn tampered_entry_digest_is_rejected() {
        let mut chain = AuditLedgerChain::new();

        let mut entry = sample_entry(0, &genesis_digest());

        entry.entry_digest = Digest("tampered".into());

        assert!(matches!(
            chain.append(entry),
            Err(AuditLedgerChainError::EntryIntegrityFailure { sequence: 0 })
        ));
    }

    #[test]
    fn multi_entry_chain_verifies() {
        let mut chain = AuditLedgerChain::new();

        let first = sample_entry(0, &genesis_digest());
        let second = sample_entry(1, &first.entry_digest);

        chain.append(first).unwrap();
        chain.append(second).unwrap();

        assert_eq!(chain.len(), 2);
        assert_eq!(chain.head().unwrap().sequence, 1);
        assert!(chain.verify_chain().is_ok());
    }

    #[test]
    fn tampered_payload_breaks_verification() {
        let mut chain = AuditLedgerChain::new();

        let mut entry = sample_entry(0, &genesis_digest());

        entry.payload_digest = Digest("changed".into());

        chain.entries.push(entry);

        assert!(matches!(
            chain.verify_chain(),
            Err(AuditLedgerChainError::EntryIntegrityFailure { sequence: 0 })
        ));
    }
}
