use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("socket error: {0}")]
    Socket(#[from] std::io::Error),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PeerAnnouncement {
    pub node_id: Uuid,
    pub listen_addr: SocketAddr,
    pub timestamp_ms: u64,
}

pub struct DiscoveryService {
    node_id: Uuid,
    broadcast_addr: SocketAddr,
    listen_addr: SocketAddr,
    running: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl DiscoveryService {
    pub fn new(node_id: Uuid, broadcast_port: u16, listen_addr: SocketAddr) -> Self {
        Self {
            node_id,
            broadcast_addr: SocketAddr::from(([255, 255, 255, 255], broadcast_port)),
            listen_addr,
            running: Arc::new(AtomicBool::new(false)),
            threads: Vec::new(),
        }
    }

    pub fn start<F>(&mut self, on_peer_discovered: F) -> Result<(), DiscoveryError>
    where
        F: Fn(PeerAnnouncement) + Send + Sync + 'static,
    {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);
        let on_peer = Arc::new(on_peer_discovered);

        let listener_socket =
            UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], self.broadcast_addr.port())))?;
        listener_socket.set_read_timeout(Some(Duration::from_millis(200)))?;

        let broadcaster_socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0)))?;
        broadcaster_socket.set_broadcast(true)?;

        let running_listener = self.running.clone();
        let local_node_id = self.node_id;

        let listener_handle = thread::spawn(move || {
            let mut buffer = [0u8; 1024];

            while running_listener.load(Ordering::SeqCst) {
                match listener_socket.recv_from(&mut buffer) {
                    Ok((bytes_read, _remote_sender)) => {
                        if let Ok(announcement) =
                            serde_json::from_slice::<PeerAnnouncement>(&buffer[..bytes_read])
                        {
                            if announcement.node_id != local_node_id {
                                on_peer(announcement);
                            }
                        }
                    }
                    Err(ref e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        continue;
                    }
                    Err(_) => break,
                }
            }
        });

        let running_broadcaster = self.running.clone();
        let broadcast_target = self.broadcast_addr;
        let payload_prototype = PeerAnnouncement {
            node_id: self.node_id,
            listen_addr: self.listen_addr,
            timestamp_ms: 0,
        };

        let broadcaster_handle = thread::spawn(move || {
            while running_broadcaster.load(Ordering::SeqCst) {
                let mut payload = payload_prototype.clone();
                payload.timestamp_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                if let Ok(serialized) = serde_json::to_vec(&payload) {
                    let _ = broadcaster_socket.send_to(&serialized, broadcast_target);
                }

                thread::sleep(Duration::from_secs(1));
            }
        });

        self.threads.push(listener_handle);
        self.threads.push(broadcaster_handle);

        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}

impl Drop for DiscoveryService {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_udp_broadcast_and_foreign_peer_extraction() {
        let node_a_id = Uuid::new_v4();
        let node_b_id = Uuid::new_v4();

        let addr_a: SocketAddr = "127.0.0.1:8081".parse().unwrap();
        let addr_b: SocketAddr = "127.0.0.1:8082".parse().unwrap();

        let shared_registry = Arc::new(Mutex::new(Vec::new()));
        let registry_clone = shared_registry.clone();

        let mut service_a = DiscoveryService::new(node_a_id, 9991, addr_a);
        service_a
            .start(move |announcement| {
                let mut list = registry_clone.lock().unwrap();
                list.push(announcement);
            })
            .unwrap();

        let outbound_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        outbound_socket.set_broadcast(true).unwrap();

        let mock_announcement = PeerAnnouncement {
            node_id: node_b_id,
            listen_addr: addr_b,
            timestamp_ms: 1000,
        };

        let packet = serde_json::to_vec(&mock_announcement).unwrap();
        let broadcast_target: SocketAddr = "255.255.255.255:9991".parse().unwrap();

        for _ in 0..3 {
            outbound_socket.send_to(&packet, broadcast_target).unwrap();
            thread::sleep(Duration::from_millis(50));
        }

        thread::sleep(Duration::from_millis(300));
        service_a.stop();

        let discovered_peers = shared_registry.lock().unwrap();
        assert!(!discovered_peers.is_empty());
        assert_eq!(discovered_peers[0].node_id, node_b_id);
        assert_eq!(discovered_peers[0].listen_addr, addr_b);
    }
}
