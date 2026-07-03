use serde::{Deserialize, Serialize};
use crate::CapabilityTier;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerEvent {
    TaskStateChanged { task_id: [u8; 16], new_status: String },
    AgentRegistered { agent_id: [u8; 16] },
    AgentSlashingRecorded { agent_id: [u8; 16], penalty: u32 },
    ProposalCommitted { proposal_id: [u8; 16], status: String },
    CapabilityTierUpdated { agent_id: [u8; 16], new_tier: CapabilityTier },
    AgentIsolationChanged { agent_id: [u8; 16], is_isolated: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerHeader {
    pub index: u64,
    pub term: u64,
    pub timestamp: u64,
    pub previous_hash: [u8; 32],
    pub payload_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub header: LedgerHeader,
    pub event: LedgerEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerError {
    InvalidIndex { expected: u64, found: u64 },
    TermRegression { current_term: u64, entry_term: u64 },
    HashChainBroken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventLedger {
    pub entries: Vec<LedgerEntry>,
    pub current_hash: [u8; 32],
}

impl EventLedger {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            current_hash: [0; 32],
        }
    }

    pub fn append_entry(&mut self, entry: LedgerEntry) -> Result<(), LedgerError> {
        let expected_index = self.current_height() + 1;

        if entry.header.index != expected_index {
            return Err(LedgerError::InvalidIndex {
                expected: expected_index,
                found: entry.header.index,
            });
        }

        if entry.header.previous_hash != self.current_hash {
            return Err(LedgerError::HashChainBroken);
        }

        if let Some(latest) = self.latest_entry() {
            if entry.header.term < latest.header.term {
                return Err(LedgerError::TermRegression {
                    current_term: latest.header.term,
                    entry_term: entry.header.term,
                });
            }
        }

        self.current_hash = compute_entry_hash(&entry);
        self.entries.push(entry);

        Ok(())
    }

    pub fn validate_chain(&self) -> Result<(), LedgerError> {
        let mut expected_previous_hash = [0; 32];

        for (position, entry) in self.entries.iter().enumerate() {
            let expected_index = position as u64 + 1;

            if entry.header.index != expected_index {
                return Err(LedgerError::InvalidIndex {
                    expected: expected_index,
                    found: entry.header.index,
                });
            }

            if entry.header.previous_hash != expected_previous_hash {
                return Err(LedgerError::HashChainBroken);
            }

            expected_previous_hash = compute_entry_hash(entry);
        }

        if expected_previous_hash != self.current_hash {
            return Err(LedgerError::HashChainBroken);
        }

        Ok(())
    }

    pub fn latest_entry(&self) -> Option<&LedgerEntry> {
        self.entries.last()
    }

    pub fn current_height(&self) -> u64 {
        self.entries.len() as u64
    }
}

impl Default for EventLedger {
    fn default() -> Self {
        Self::new()
    }
}

pub fn compute_entry_hash(entry: &LedgerEntry) -> [u8; 32] {
    let mut hash = [0u8; 32];

    mix_u64(&mut hash, entry.header.index);
    mix_u64(&mut hash, entry.header.term);
    mix_u64(&mut hash, entry.header.timestamp);

    for (i, byte) in entry.header.previous_hash.iter().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(*byte).rotate_left(1);
    }

    for (i, byte) in entry.header.payload_hash.iter().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(*byte).rotate_left(1);
    }

    mix_event(&mut hash, &entry.event);

    hash
}

fn mix_u64(hash: &mut [u8; 32], value: u64) {
    for (i, byte) in value.to_le_bytes().iter().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(*byte).rotate_left(1);
    }
}

fn mix_event(hash: &mut [u8; 32], event: &LedgerEvent) {
    match event {
        LedgerEvent::TaskStateChanged { task_id, new_status } => {
            mix_bytes(hash, task_id);
            mix_bytes(hash, new_status.as_bytes());
        }
        LedgerEvent::AgentRegistered { agent_id } => {
            mix_bytes(hash, agent_id);
        }
        LedgerEvent::AgentSlashingRecorded { agent_id, penalty } => {
            mix_bytes(hash, agent_id);
            mix_u64(hash, *penalty as u64);
        }
        LedgerEvent::ProposalCommitted { proposal_id, status } => {
            mix_bytes(hash, proposal_id);
            mix_bytes(hash, status.as_bytes());
        }
        LedgerEvent::CapabilityTierUpdated { agent_id, new_tier } => {
            mix_bytes(hash, agent_id);
            mix_u64(hash, *new_tier as u64);
        }
        LedgerEvent::AgentIsolationChanged { agent_id, is_isolated } => {
            mix_bytes(hash, agent_id);
            mix_u64(hash, *is_isolated as u64);
        }
    }
}

fn mix_bytes(hash: &mut [u8; 32], bytes: &[u8]) {
    for (i, byte) in bytes.iter().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(*byte).rotate_left(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: u64, previous_hash: [u8; 32]) -> LedgerEntry {
        LedgerEntry {
            header: LedgerHeader {
                index,
                term: 1,
                timestamp: index * 100,
                previous_hash,
                payload_hash: [index as u8; 32],
            },
            event: LedgerEvent::AgentRegistered {
                agent_id: [index as u8; 16],
            },
        }
    }

    #[test]
    fn new_ledger_starts_empty() {
        let ledger = EventLedger::new();

        assert_eq!(ledger.current_height(), 0);
        assert_eq!(ledger.current_hash, [0; 32]);
        assert!(ledger.latest_entry().is_none());
    }

    #[test]
    fn append_entry_advances_height_and_hash() {
        let mut ledger = EventLedger::new();
        let first = entry(1, [0; 32]);

        ledger.append_entry(first).unwrap();

        assert_eq!(ledger.current_height(), 1);
        assert_ne!(ledger.current_hash, [0; 32]);
        assert!(ledger.latest_entry().is_some());
    }

    #[test]
    fn append_rejects_invalid_index() {
        let mut ledger = EventLedger::new();
        let bad = entry(2, [0; 32]);

        assert_eq!(
            ledger.append_entry(bad),
            Err(LedgerError::InvalidIndex {
                expected: 1,
                found: 2,
            })
        );
    }

    #[test]
    fn append_rejects_broken_hash_chain() {
        let mut ledger = EventLedger::new();
        let bad = entry(1, [9; 32]);

        assert_eq!(ledger.append_entry(bad), Err(LedgerError::HashChainBroken));
    }

    #[test]
    fn validate_chain_accepts_valid_entries() {
        let mut ledger = EventLedger::new();

        let first = entry(1, [0; 32]);
        ledger.append_entry(first).unwrap();

        let second = entry(2, ledger.current_hash);
        ledger.append_entry(second).unwrap();

        assert_eq!(ledger.validate_chain(), Ok(()));
    }

    #[test]
    fn validate_chain_rejects_tampering() {
        let mut ledger = EventLedger::new();

        let first = entry(1, [0; 32]);
        ledger.append_entry(first).unwrap();

        ledger.entries[0].header.payload_hash = [99; 32];

        assert_eq!(ledger.validate_chain(), Err(LedgerError::HashChainBroken));
    }

    #[test]
    fn append_rejects_term_regression() {
        let mut ledger = EventLedger::new();

        let first = entry(1, [0; 32]);
        ledger.append_entry(first).unwrap();

        let mut second = entry(2, ledger.current_hash);
        second.header.term = 0;

        assert_eq!(
            ledger.append_entry(second),
            Err(LedgerError::TermRegression {
                current_term: 1,
                entry_term: 0,
            })
        );
    }
}
