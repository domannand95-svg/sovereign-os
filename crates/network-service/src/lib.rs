pub mod consensus;
pub mod discovery;
pub mod heartbeat;
pub mod messaging;

pub use consensus::{ConsensusState, NodeRole};
pub use discovery::{DiscoveryError, DiscoveryService, PeerAnnouncement};
pub use heartbeat::{HeartbeatRecord, HeartbeatService, PeerState};
pub use messaging::{MessageTransport, MessagingError, NetworkMessage};
