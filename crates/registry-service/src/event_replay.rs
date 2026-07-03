use crate::{LedgerEntry, PersistenceEngine};

pub struct EventReplayService;

impl EventReplayService {
    pub fn replay<P: PersistenceEngine>(
        engine: &P,
    ) -> Result<Vec<LedgerEntry>, P::Error> {
        engine.load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LedgerEvent, LedgerHeader, MemoryPersistence};

    fn entry(index: u64) -> LedgerEntry {
        LedgerEntry {
            header: LedgerHeader {
                index,
                term: 1,
                timestamp: index * 100,
                previous_hash: [0; 32],
                payload_hash: [index as u8; 32],
            },
            event: LedgerEvent::AgentRegistered {
                agent_id: [index as u8; 16],
            },
        }
    }

    #[test]
    fn replay_returns_persisted_entries_in_order() {
        let mut storage = MemoryPersistence::new();

        storage.append(&entry(1)).unwrap();
        storage.append(&entry(2)).unwrap();

        let replayed = EventReplayService::replay(&storage).unwrap();

        assert_eq!(replayed.len(), 2);
        assert_eq!(replayed[0].header.index, 1);
        assert_eq!(replayed[1].header.index, 2);
    }
}
