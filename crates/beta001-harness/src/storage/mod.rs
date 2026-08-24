//! ADAM-013 Storage, Persistence, and Crash-Recovery Subsystem

pub mod frame;
pub mod log;
pub mod recovery;

pub use frame::{
    CommitLogFrame, CommitRecordPayload, FrameError, COMMIT_LOG_FORMAT_VERSION_V1,
    COMMIT_LOG_FRAME_DOMAIN_TAG, COMMIT_LOG_MAGIC,
};
pub use log::{
    CommitLogWriter, DurabilityAcknowledgement, SyncPolicy, DEFAULT_MAX_FRAME_PAYLOAD_BYTES,
};
pub use recovery::{CommitLogRecovery, RecoveryError, RecoveryReport};
