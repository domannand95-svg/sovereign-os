//! ADAM-013-A: Append-Only Commit Log Writer & Durability Controller

use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::frame::{CommitLogFrame, CommitRecordPayload, FrameError};

pub const DEFAULT_MAX_FRAME_PAYLOAD_BYTES: u64 = 64 * 1024 * 1024; // 64 MB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPolicy {
    /// Issue fsync() on the file descriptor upon acknowledging every committed transaction.
    FsyncOnCommit,
    /// Issue fsync() on every low-level physical file write operation.
    FsyncAlways,
    /// In-memory buffering; requires explicit sync() barrier for durable confirmation.
    FsyncPeriodic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurabilityAcknowledgement {
    DurableCommitted {
        sequence_tick: u64,
        file_offset: u64,
    },
    CommittedInMemory {
        sequence_tick: u64,
        file_offset: u64,
    },
}

pub struct CommitLogWriter {
    path: PathBuf,
    file: Mutex<File>,
    policy: SyncPolicy,
    max_payload_bytes: u64,
    current_tick: Mutex<u64>,
}

impl CommitLogWriter {
    /// Opens or creates an append-only commit log at specified path.
    pub fn open(path: impl AsRef<Path>, policy: SyncPolicy) -> Result<Self, FrameError> {
        let path_buf = path.as_ref().to_path_buf();
        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent).map_err(|e| FrameError::IoError(e.to_string()))?;
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path_buf)
            .map_err(|e| FrameError::IoError(e.to_string()))?;

        // Position file pointer at current tail
        file.seek(SeekFrom::End(0))
            .map_err(|e| FrameError::IoError(e.to_string()))?;

        Ok(Self {
            path: path_buf,
            file: Mutex::new(file),
            policy,
            max_payload_bytes: DEFAULT_MAX_FRAME_PAYLOAD_BYTES,
            current_tick: Mutex::new(0),
        })
    }

    /// Appends a commit record payload and enforces the specified durability frontier.
    pub fn append(
        &self,
        sequence_tick: u64,
        payload: CommitRecordPayload,
    ) -> Result<DurabilityAcknowledgement, FrameError> {
        let mut file_guard = self.file.lock().unwrap();
        let mut tick_guard = self.current_tick.lock().unwrap();

        let payload_bytes = payload.encode_canonical();
        let checksum = CommitLogFrame::compute_checksum(sequence_tick, &payload_bytes);

        let frame = CommitLogFrame {
            sequence_tick,
            payload,
            checksum,
        };

        let start_offset = file_guard
            .seek(SeekFrom::End(0))
            .map_err(|e| FrameError::IoError(e.to_string()))?;
        frame.write_to(&mut *file_guard)?;

        *tick_guard = sequence_tick;

        match self.policy {
            SyncPolicy::FsyncOnCommit | SyncPolicy::FsyncAlways => {
                file_guard
                    .sync_all()
                    .map_err(|e| FrameError::IoError(e.to_string()))?;
                Ok(DurabilityAcknowledgement::DurableCommitted {
                    sequence_tick,
                    file_offset: start_offset,
                })
            }
            SyncPolicy::FsyncPeriodic => {
                file_guard
                    .flush()
                    .map_err(|e| FrameError::IoError(e.to_string()))?;
                Ok(DurabilityAcknowledgement::CommittedInMemory {
                    sequence_tick,
                    file_offset: start_offset,
                })
            }
        }
    }

    /// Explicitly flushes and syncs userspace buffers to underlying non-volatile storage.
    pub fn sync(&self) -> Result<(), FrameError> {
        let file_guard = self.file.lock().unwrap();
        file_guard
            .sync_all()
            .map_err(|e| FrameError::IoError(e.to_string()))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
