use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionState {
    Follower,
    Candidate,
    Leader,
}

pub struct LeaderElection {
    state: ElectionState,
    last_heartbeat: Instant,
    timeout: Duration,
}

impl LeaderElection {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            state: ElectionState::Follower,
            last_heartbeat: Instant::now(),
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    pub fn tick(&mut self) {
        if self.state == ElectionState::Follower && self.last_heartbeat.elapsed() >= self.timeout {
            self.state = ElectionState::Candidate;
        }
    }

    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
        self.state = ElectionState::Follower;
    }

    pub fn state(&self) -> ElectionState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn follower_times_out() {
        let mut election = LeaderElection::new(5);
        sleep(Duration::from_millis(10));
        election.tick();
        assert_eq!(election.state(), ElectionState::Candidate);
    }

    #[test]
    fn heartbeat_resets_state() {
        let mut election = LeaderElection::new(100);
        election.heartbeat();
        assert_eq!(election.state(), ElectionState::Follower);
    }
}
