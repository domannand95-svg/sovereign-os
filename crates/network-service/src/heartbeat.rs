use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    Healthy,
    Degraded,
    Dead,
}

#[derive(Debug, Clone)]
pub struct HeartbeatRecord {
    pub node_id: Uuid,
    pub last_seen_ms: u64,
}

#[derive(Debug, Default)]
pub struct HeartbeatService {
    peers: HashMap<Uuid, HeartbeatRecord>,
}

impl HeartbeatService {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    pub fn record_heartbeat(&mut self, node_id: Uuid) {
        self.peers.insert(
            node_id,
            HeartbeatRecord {
                node_id,
                last_seen_ms: current_time_ms(),
            },
        );
    }

    pub fn peer_state(&self, node_id: &Uuid, now_ms: u64) -> Option<PeerState> {
        let record = self.peers.get(node_id)?;
        let age = now_ms.saturating_sub(record.last_seen_ms);

        if age <= 2_500 {
            Some(PeerState::Healthy)
        } else if age <= 5_000 {
            Some(PeerState::Degraded)
        } else {
            Some(PeerState::Dead)
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_heartbeat() {
        let node_id = Uuid::new_v4();
        let mut service = HeartbeatService::new();

        service.record_heartbeat(node_id);

        assert_eq!(service.peer_count(), 1);
    }

    #[test]
    fn classifies_peer_state() {
        let node_id = Uuid::new_v4();
        let mut service = HeartbeatService::new();

        service.peers.insert(
            node_id,
            HeartbeatRecord {
                node_id,
                last_seen_ms: 1_000,
            },
        );

        assert_eq!(
            service.peer_state(&node_id, 3_000),
            Some(PeerState::Healthy)
        );
        assert_eq!(
            service.peer_state(&node_id, 5_000),
            Some(PeerState::Degraded)
        );
        assert_eq!(service.peer_state(&node_id, 7_000), Some(PeerState::Dead));
    }
}
