use crate::{LedgerEntry, PersistenceEngine};

pub struct EventReplayService;

impl EventReplayService {
    pub fn replay<P: PersistenceEngine>(
        _engine: &P,
    ) -> Result<Vec<LedgerEntry>, P::Error> {
        todo!("event replay implementation");
    }
}
