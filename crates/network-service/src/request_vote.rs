#[derive(Debug, Clone)]
pub struct RequestVoteRequest {
    pub term: u64,
    pub candidate_id: String,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

#[derive(Debug, Clone)]
pub struct RequestVoteResponse {
    pub term: u64,
    pub vote_granted: bool,
}

pub struct RequestVoteHandler;

impl RequestVoteHandler {
    pub fn handle(
        current_term: u64,
        voted_for: Option<&str>,
        request: &RequestVoteRequest,
    ) -> RequestVoteResponse {
        if request.term < current_term {
            return RequestVoteResponse {
                term: current_term,
                vote_granted: false,
            };
        }

        if let Some(candidate) = voted_for {
            if candidate != request.candidate_id {
                return RequestVoteResponse {
                    term: request.term,
                    vote_granted: false,
                };
            }
        }

        RequestVoteResponse {
            term: request.term,
            vote_granted: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_old_term() {
        let req = RequestVoteRequest {
            term: 1,
            candidate_id: "node-a".into(),
            last_log_index: 0,
            last_log_term: 0,
        };

        let resp = RequestVoteHandler::handle(2, None, &req);

        assert!(!resp.vote_granted);
    }

    #[test]
    fn grants_vote_if_not_voted() {
        let req = RequestVoteRequest {
            term: 2,
            candidate_id: "node-a".into(),
            last_log_index: 0,
            last_log_term: 0,
        };

        let resp = RequestVoteHandler::handle(2, None, &req);

        assert!(resp.vote_granted);
    }

    #[test]
    fn rejects_second_candidate() {
        let req = RequestVoteRequest {
            term: 2,
            candidate_id: "node-b".into(),
            last_log_index: 0,
            last_log_term: 0,
        };

        let resp = RequestVoteHandler::handle(2, Some("node-a"), &req);

        assert!(!resp.vote_granted);
    }
}
