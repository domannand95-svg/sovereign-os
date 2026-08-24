//! ADAM-014-C / 014-D: One-Way Verified State Replication & Replay Shield
//!
//! Enables primary-to-replica state replication over SOVWIRE1 framing with
//! strict sequence tick monotonicity, lineage verification, and replay rejection.

use super::wire::{WireError, WireFrame, WireMessageType};
use crate::storage::{
    CommitRecordPayload, DurabilityAcknowledgement, EngineError, FrameError, StorageEngine,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicationError {
    ReplayDetected {
        current_tick: u64,
        attempted_tick: u64,
    },
    SequenceGapDetected {
        expected_tick: u64,
        received_tick: u64,
    },
    InvalidMessageType(WireMessageType),
    Engine(EngineError),
    Frame(FrameError),
    Wire(WireError),
}

impl From<EngineError> for ReplicationError {
    fn from(err: EngineError) -> Self {
        Self::Engine(err)
    }
}

impl From<FrameError> for ReplicationError {
    fn from(err: FrameError) -> Self {
        Self::Frame(err)
    }
}

impl From<WireError> for ReplicationError {
    fn from(err: WireError) -> Self {
        Self::Wire(err)
    }
}

impl std::fmt::Display for ReplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReplayDetected {
                current_tick,
                attempted_tick,
            } => {
                write!(
                    f,
                    "Replay rejected: current sequence tick is {}, attempted tick was {}",
                    current_tick, attempted_tick
                )
            }
            Self::SequenceGapDetected {
                expected_tick,
                received_tick,
            } => {
                write!(
                    f,
                    "Sequence gap rejected: expected tick {}, received tick {}",
                    expected_tick, received_tick
                )
            }
            Self::InvalidMessageType(t) => {
                write!(f, "Invalid message type for state replication: {:?}", t)
            }
            Self::Engine(e) => write!(f, "Replication storage engine error: {}", e),
            Self::Frame(e) => write!(f, "Replication frame error: {}", e),
            Self::Wire(e) => write!(f, "Replication wire error: {}", e),
        }
    }
}

impl std::error::Error for ReplicationError {}

pub struct StateReplicator;

impl StateReplicator {
    /// Encapsulates a committed transaction record into a canonical WireFrame for transmission.
    pub fn create_commit_wire_frame(
        sequence_tick: u64,
        payload: &CommitRecordPayload,
    ) -> WireFrame {
        let payload_bytes = payload.encode_canonical();
        WireFrame::new(WireMessageType::CommitFrame, sequence_tick, payload_bytes)
    }

    /// Ingests an incoming WireFrame into the local replica StorageEngine with strict replay protection.
    pub fn ingest_replicated_frame(
        engine: &StorageEngine,
        frame: &WireFrame,
    ) -> Result<DurabilityAcknowledgement, ReplicationError> {
        if frame.msg_type != WireMessageType::CommitFrame {
            return Err(ReplicationError::InvalidMessageType(frame.msg_type));
        }

        let current_tick = engine.current_sequence_tick();

        // 1. Replay Shield: Reject past or duplicate sequence ticks
        if frame.sequence_tick <= current_tick {
            return Err(ReplicationError::ReplayDetected {
                current_tick,
                attempted_tick: frame.sequence_tick,
            });
        }

        // 2. Sequence Gap Shield: Must be strictly sequential
        if frame.sequence_tick != current_tick + 1 {
            return Err(ReplicationError::SequenceGapDetected {
                expected_tick: current_tick + 1,
                received_tick: frame.sequence_tick,
            });
        }

        // 3. Decode payload & commit to replica storage engine
        let payload = CommitRecordPayload::decode_canonical(frame.payload.as_slice())?;
        let ack = engine.commit_record(frame.sequence_tick, payload)?;

        Ok(ack)
    }
}
