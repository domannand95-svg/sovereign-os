use std::time::{Duration, Instant};

const MIN_TIMEOUT_MS: u64 = 150;
const MAX_TIMEOUT_MS: u64 = 300;

#[derive(Debug, Clone)]
pub struct ElectionTimer {
    started_at: Instant,
    timeout: Duration,
}

impl ElectionTimer {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            timeout: randomized_timeout(),
        }
    }

    pub fn reset(&mut self) {
        self.started_at = Instant::now();
        self.timeout = randomized_timeout();
    }

    pub fn has_expired(&self) -> bool {
        self.started_at.elapsed() >= self.timeout
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl Default for ElectionTimer {
    fn default() -> Self {
        Self::new()
    }
}

fn randomized_timeout() -> Duration {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;

    let span = MAX_TIMEOUT_MS - MIN_TIMEOUT_MS;
    let selected_ms = MIN_TIMEOUT_MS + (now % (span + 1));

    Duration::from_millis(selected_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_election_timer_timeout_bounds() {
        for _ in 0..32 {
            let timer = ElectionTimer::new();
            assert!(timer.timeout() >= Duration::from_millis(MIN_TIMEOUT_MS));
            assert!(timer.timeout() <= Duration::from_millis(MAX_TIMEOUT_MS));
        }
    }

    #[test]
    fn test_election_timer_reset_refreshes_start_time() {
        let mut timer = ElectionTimer::new();
        std::thread::sleep(Duration::from_millis(5));

        let before_reset_elapsed = timer.started_at.elapsed();
        timer.reset();
        let after_reset_elapsed = timer.started_at.elapsed();

        assert!(before_reset_elapsed > after_reset_elapsed);
    }
}
