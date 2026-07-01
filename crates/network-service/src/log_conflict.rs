use crate::log_replication::LogEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogConflictResolution {
    AlreadyConsistent,
    Truncated,
    Appended,
    ReplacedConflict,
}

#[derive(Debug, Default)]
pub struct LogConflictResolver;

impl LogConflictResolver {
    pub fn resolve(
        local_log: &mut Vec<LogEntry>,
        incoming_entries: Vec<LogEntry>,
    ) -> LogConflictResolution {
        if incoming_entries.is_empty() {
            return LogConflictResolution::AlreadyConsistent;
        }

        let mut result = LogConflictResolution::AlreadyConsistent;

        for incoming in incoming_entries {
            match local_log
                .iter()
                .position(|entry| entry.index == incoming.index)
            {
                Some(position) => {
                    if local_log[position].term != incoming.term {
                        local_log.truncate(position);
                        local_log.push(incoming);
                        result = LogConflictResolution::ReplacedConflict;
                    }
                }
                None => {
                    local_log.push(incoming);
                    result = LogConflictResolution::Appended;
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: u64, term: u64) -> LogEntry {
        LogEntry {
            index,
            term,
            command: String::new(),
        }
    }

    #[test]
    fn appends_new_entries() {
        let mut log = vec![entry(1, 1)];

        let result = LogConflictResolver::resolve(&mut log, vec![entry(2, 1), entry(3, 1)]);

        assert_eq!(result, LogConflictResolution::Appended);
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn replaces_conflicting_entries() {
        let mut log = vec![entry(1, 1), entry(2, 1), entry(3, 2)];

        let result = LogConflictResolver::resolve(&mut log, vec![entry(3, 3), entry(4, 3)]);

        assert_eq!(result, LogConflictResolution::Appended);
        assert_eq!(log.len(), 4);
        assert_eq!(log[2].term, 3);
        assert_eq!(log[3].index, 4);
    }

    #[test]
    fn leaves_identical_log_unchanged() {
        let mut log = vec![entry(1, 1), entry(2, 1)];

        let result = LogConflictResolver::resolve(&mut log, vec![entry(1, 1), entry(2, 1)]);

        assert_eq!(result, LogConflictResolution::AlreadyConsistent);
        assert_eq!(log.len(), 2);
    }
}
