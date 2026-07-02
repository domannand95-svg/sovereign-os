use registry_service::AllocationRequest;
use std::io::{Read, Write};
use std::net::TcpStream;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
    #[error("I/O transport failure: {0}")]
    TransportError(#[from] std::io::Error),

    #[error("Serialization or deserialization failure: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("Remote stream closed prematurely")]
    ConnectionClosed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum NetworkMessage {
    Handshake {
        node_id: Uuid,
        protocol_version: u32,
    },
    VoteRequest {
        term: u64,
        candidate_id: Uuid,
    },
    VoteResponse {
        term: u64,
        voter_id: Uuid,
        vote_granted: bool,
    },
    HeartbeatPing {
        node_id: Uuid,
        sequence: u64,
    },
    HeartbeatAck {
        node_id: Uuid,
        sequence: u64,
    },
    InstallSnapshotRequest {
        term: u64,
        leader_id: Uuid,
        last_included_index: u64,
        last_included_term: u64,
        offset: u64,
        data: Vec<u8>,
        done: bool,
    },
    InstallSnapshotResponse {
        term: u64,
        follower_id: Uuid,
        bytes_stored: u64,
        success: bool,
    },
    WorkloadProposal {
        transaction_id: Uuid,
        workload_id: Uuid,
        priority: u32,
        request: AllocationRequest,
    },
    WorkloadAccept {
        transaction_id: Uuid,
        assigned_node_id: Uuid,
    },
    WorkloadReject {
        transaction_id: Uuid,
        reason: String,
    },
}

pub struct MessageTransport;

impl MessageTransport {
    pub fn send(stream: &mut TcpStream, message: &NetworkMessage) -> Result<(), MessagingError> {
        let payload = serde_json::to_vec(message)?;
        let length = payload.len() as u32;

        stream.write_all(&length.to_be_bytes())?;
        stream.write_all(&payload)?;
        stream.flush()?;

        Ok(())
    }

    pub fn receive(stream: &mut TcpStream) -> Result<NetworkMessage, MessagingError> {
        let mut length_buffer = [0u8; 4];

        match stream.read_exact(&mut length_buffer) {
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(MessagingError::ConnectionClosed);
            }
            Err(e) => return Err(MessagingError::TransportError(e)),
        }

        let length = u32::from_be_bytes(length_buffer) as usize;
        let mut payload_buffer = vec![0u8; length];

        stream.read_exact(&mut payload_buffer)?;

        Ok(serde_json::from_slice(&payload_buffer)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_length_prefixed_tcp_round_trip() {
        let server_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let bound_addr = server_listener.local_addr().unwrap();

        let test_tx_id = Uuid::new_v4();
        let test_wl_id = Uuid::new_v4();

        let outbound_message = NetworkMessage::WorkloadProposal {
            transaction_id: test_tx_id,
            workload_id: test_wl_id,
            priority: 500,
            request: AllocationRequest {
                required_compute_cores: 8,
                required_memory_bytes: 34_359_738_368,
                required_capabilities: vec!["high-throughput".to_string()],
            },
        };

        let message_clone = outbound_message.clone();

        let server_handle = thread::spawn(move || {
            let (mut inbound_stream, _) = server_listener.accept().unwrap();
            let incoming_payload = MessageTransport::receive(&mut inbound_stream).unwrap();

            assert_eq!(incoming_payload, message_clone);
        });

        let mut client_stream = TcpStream::connect(bound_addr).unwrap();
        MessageTransport::send(&mut client_stream, &outbound_message).unwrap();

        server_handle.join().unwrap();
    }

    #[test]
    fn test_vote_response_tcp_round_trip() {
        let server_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let bound_addr = server_listener.local_addr().unwrap();

        let voter_id = Uuid::new_v4();

        let outbound_message = NetworkMessage::VoteResponse {
            term: 7,
            voter_id,
            vote_granted: true,
        };

        let message_clone = outbound_message.clone();

        let server_handle = thread::spawn(move || {
            let (mut inbound_stream, _) = server_listener.accept().unwrap();
            let incoming_payload = MessageTransport::receive(&mut inbound_stream).unwrap();

            assert_eq!(incoming_payload, message_clone);
        });

        let mut client_stream = TcpStream::connect(bound_addr).unwrap();
        MessageTransport::send(&mut client_stream, &outbound_message).unwrap();

        server_handle.join().unwrap();
    
}
}
