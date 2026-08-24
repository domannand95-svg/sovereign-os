//! ADAM-013 Storage, Persistence, Snapshotting, and Crash-Recovery Subsystem

pub mod frame;
pub mod log;
pub mod recovery;
pub mod snapshot;

pub use frame::{
    CommitLogFrame, CommitRecordPayload, FrameError, COMMIT_LOG_FORMAT_VERSION_V1,
    COMMIT_LOG_FRAME_DOMAIN_TAG, COMMIT_LOG_MAGIC,
};
pub use log::{
    CommitLogWriter, DurabilityAcknowledgement, SyncPolicy, DEFAULT_MAX_FRAME_PAYLOAD_BYTES,
};
pub use recovery::{CommitLogRecovery, RecoveryError, RecoveryReport};
pub use snapshot::{
    SnapshotError, SnapshotManifest, StateSnapshot, SNAPSHOT_DOMAIN_TAG,
    SNAPSHOT_FORMAT_VERSION_V1, SNAPSHOT_MAGIC,
};
