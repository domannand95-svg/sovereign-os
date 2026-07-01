use crate::log_replication::LogEntry;

#[derive(Debug, Clone)]
pub struct AppendEntriesRequest {
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

#[derive(Debug, Clone)]
pub struct AppendEntriesResponse {
    pub term: u64,
    pub success: bool,
}

pub struct AppendEntriesHandler;

impl AppendEntriesHandler {
    pub fn handle(current_term: u64, request: &AppendEntriesRequest) -> AppendEntriesResponse {
        if request.term < current_term {
            return AppendEntriesResponse {
                term: current_term,
                success: false,
            };
        }

        AppendEntriesResponse {
            term: request.term,
            success: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_stale_term() {
        let request = AppendEntriesRequest {
            term: 1,
            leader_id: "leader".into(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: Vec::new(),
            leader_commit: 0,
        };

        let response = AppendEntriesHandler::handle(2, &request);

        assert!(!response.success);
        assert_eq!(response.term, 2);
    }

    #[test]
    fn accepts_current_term() {
        let request = AppendEntriesRequest {
            term: 2,
            leader_id: "leader".into(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: Vec::new(),
            leader_commit: 0,
        };

        let response = AppendEntriesHandler::handle(2, &request);

        assert!(response.success);
        assert_eq!(response.term, 2);
    }
}
