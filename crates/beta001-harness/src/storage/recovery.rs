//! ADAM-013-B: Crash Recovery, Lineage Replay & Corrupted Tail Truncation
//!
//! Scans append-only commit logs, verifies semantic state and causal lineage transitions,
//! safely truncates incomplete terminal writes (TornTail), and fails closed on interior corruptions.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::frame::{CommitLogFrame, FrameError, COMMIT_LOG_MAGIC};
use super::log::DEFAULT_MAX_FRAME_PAYLOAD_BYTES;
use crate::state::{
    compute_delta_digest, compute_genesis_transition_root, compute_transition_root, StateTree,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    InteriorCorruption {
        offset: u64,
        reason: String,
    },
    SequenceGap {
        expected: u64,
        actual: u64,
        offset: u64,
    },
    LineageMismatch {
        reason: String,
        offset: u64,
    },
    StateRootMismatch {
        expected: String,
        computed: String,
        offset: u64,
    },
    TransitionRootMismatch {
        expected: String,
        computed: String,
        offset: u64,
    },
    IoError(String),
}

impl std::fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InteriorCorruption { offset, reason } => {
                write!(
                    f,
                    "Interior log corruption at offset {}: {}",
                    offset, reason
                )
            }
            Self::SequenceGap {
                expected,
                actual,
                offset,
            } => {
                write!(
                    f,
                    "Sequence gap at offset {}: expected {}, found {}",
                    offset, expected, actual
                )
            }
            Self::LineageMismatch { reason, offset } => {
                write!(f, "Lineage mismatch at offset {}: {}", offset, reason)
            }
            Self::StateRootMismatch {
                expected,
                computed,
                offset,
            } => {
                write!(
                    f,
                    "State root mismatch at offset {}: expected {}, computed {}",
                    offset, expected, computed
                )
            }
            Self::TransitionRootMismatch {
                expected,
                computed,
                offset,
            } => {
                write!(
                    f,
                    "Transition root mismatch at offset {}: expected {}, computed {}",
                    offset, expected, computed
                )
            }
            Self::IoError(e) => write!(f, "Recovery I/O error: {}", e),
        }
    }
}

impl std::error::Error for RecoveryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    pub recovered_records_count: usize,
    pub last_sequence_tick: u64,
    pub last_verified_offset: u64,
    pub torn_tail_truncated_bytes: u64,
    pub recovered_state_root: String,
    pub recovered_transition_root: String,
    pub recovered_revision: u64,
}

struct TailTruncationContext<'a> {
    file: &'a File,
    file_len: u64,
    last_verified_offset: u64,
    records_count: usize,
    last_sequence_tick: u64,
    tree: &'a StateTree,
    current_transition_root: String,
    auto_truncate: bool,
}

pub struct CommitLogRecovery;

impl CommitLogRecovery {
    /// Recovers and verifies state and transition lineage from an append-only commit log.
    pub fn recover_from_log(
        log_path: impl AsRef<Path>,
        tree: &mut StateTree,
        initial_transition_root: Option<String>,
        auto_truncate_torn_tail: bool,
    ) -> Result<RecoveryReport, RecoveryError> {
        let path: PathBuf = log_path.as_ref().to_path_buf();
        if !path.exists() {
            return Ok(RecoveryReport {
                recovered_records_count: 0,
                last_sequence_tick: 0,
                last_verified_offset: 0,
                torn_tail_truncated_bytes: 0,
                recovered_state_root: tree.compute_state_root(),
                recovered_transition_root: initial_transition_root
                    .unwrap_or_else(compute_genesis_transition_root),
                recovered_revision: tree.revision(),
            });
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| RecoveryError::IoError(e.to_string()))?;

        let file_len = file
            .seek(SeekFrom::End(0))
            .map_err(|e| RecoveryError::IoError(e.to_string()))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| RecoveryError::IoError(e.to_string()))?;

        let mut current_offset = 0u64;
        let mut last_verified_offset = 0u64;
        let mut last_sequence_tick = 0u64;
        let mut records_count = 0usize;
        let mut current_transition_root =
            initial_transition_root.unwrap_or_else(compute_genesis_transition_root);

        loop {
            if current_offset >= file_len {
                break;
            }

            let remaining = file_len - current_offset;
            if remaining < CommitLogFrame::HEADER_SIZE as u64 {
                let ctx = TailTruncationContext {
                    file: &file,
                    file_len,
                    last_verified_offset,
                    records_count,
                    last_sequence_tick,
                    tree,
                    current_transition_root,
                    auto_truncate: auto_truncate_torn_tail,
                };
                return Self::handle_tail_truncation(ctx);
            }

            let mut header_buf = [0u8; CommitLogFrame::HEADER_SIZE];
            if let Err(e) = file.read_exact(&mut header_buf) {
                return Err(RecoveryError::IoError(e.to_string()));
            }

            if &header_buf[0..8] != COMMIT_LOG_MAGIC {
                return Err(RecoveryError::InteriorCorruption {
                    offset: current_offset,
                    reason: "Invalid frame magic bytes".to_string(),
                });
            }

            let payload_len = u64::from_be_bytes(header_buf[18..26].try_into().unwrap());
            let total_frame_len = CommitLogFrame::HEADER_SIZE as u64
                + payload_len
                + CommitLogFrame::CHECKSUM_SIZE as u64;

            if remaining < total_frame_len {
                let ctx = TailTruncationContext {
                    file: &file,
                    file_len,
                    last_verified_offset,
                    records_count,
                    last_sequence_tick,
                    tree,
                    current_transition_root,
                    auto_truncate: auto_truncate_torn_tail,
                };
                return Self::handle_tail_truncation(ctx);
            }

            file.seek(SeekFrom::Start(current_offset))
                .map_err(|e| RecoveryError::IoError(e.to_string()))?;
            let frame = match CommitLogFrame::read_from(&mut file, DEFAULT_MAX_FRAME_PAYLOAD_BYTES)
            {
                Ok(f) => f,
                Err(FrameError::ChecksumMismatch { expected, actual }) => {
                    if current_offset + total_frame_len == file_len && auto_truncate_torn_tail {
                        let ctx = TailTruncationContext {
                            file: &file,
                            file_len,
                            last_verified_offset,
                            records_count,
                            last_sequence_tick,
                            tree,
                            current_transition_root,
                            auto_truncate: true,
                        };
                        return Self::handle_tail_truncation(ctx);
                    }
                    return Err(RecoveryError::InteriorCorruption {
                        offset: current_offset,
                        reason: format!(
                            "Checksum mismatch: expected {:?}, actual {:?}",
                            expected, actual
                        ),
                    });
                }
                Err(e) => {
                    return Err(RecoveryError::InteriorCorruption {
                        offset: current_offset,
                        reason: format!("Frame deserialization error: {:?}", e),
                    });
                }
            };

            // 1. Verify sequence tick strict monotonicity
            if records_count > 0 && frame.sequence_tick != last_sequence_tick + 1 {
                return Err(RecoveryError::SequenceGap {
                    expected: last_sequence_tick + 1,
                    actual: frame.sequence_tick,
                    offset: current_offset,
                });
            }

            // 2. Verify pre-state conditions and revision
            let current_root = tree.compute_state_root();
            if frame.payload.pre_revision != tree.revision() {
                return Err(RecoveryError::LineageMismatch {
                    reason: format!(
                        "Pre-revision mismatch: expected {}, recorded {}",
                        tree.revision(),
                        frame.payload.pre_revision
                    ),
                    offset: current_offset,
                });
            }

            if frame.payload.pre_state_root != current_root {
                return Err(RecoveryError::StateRootMismatch {
                    expected: frame.payload.pre_state_root,
                    computed: current_root,
                    offset: current_offset,
                });
            }

            if frame.payload.prior_transition_root != current_transition_root {
                return Err(RecoveryError::TransitionRootMismatch {
                    expected: frame.payload.prior_transition_root,
                    computed: current_transition_root,
                    offset: current_offset,
                });
            }

            // 3. Replay mutations against StateTree
            let changed = tree.apply_raw_mutations(&frame.payload.mutations);
            let post_root = tree.compute_state_root();

            if changed && tree.revision() != frame.payload.post_revision {
                return Err(RecoveryError::LineageMismatch {
                    reason: format!(
                        "Post-revision mismatch: expected {}, recorded {}",
                        tree.revision(),
                        frame.payload.post_revision
                    ),
                    offset: current_offset,
                });
            }

            if post_root != frame.payload.post_state_root {
                return Err(RecoveryError::StateRootMismatch {
                    expected: frame.payload.post_state_root,
                    computed: post_root,
                    offset: current_offset,
                });
            }

            // 4. Verify cryptographic transition root derivation
            let delta_digest = compute_delta_digest(&frame.payload.mutations);
            let computed_trans_root = compute_transition_root(
                &current_transition_root,
                &frame.payload.pre_state_root,
                &frame.payload.post_state_root,
                frame.payload.package_digest.as_str(),
                frame.payload.execution_id.as_str(),
                &delta_digest,
            );

            if computed_trans_root != frame.payload.transition_root {
                return Err(RecoveryError::TransitionRootMismatch {
                    expected: frame.payload.transition_root,
                    computed: computed_trans_root,
                    offset: current_offset,
                });
            }

            current_transition_root = computed_trans_root;
            last_sequence_tick = frame.sequence_tick;
            last_verified_offset += total_frame_len;
            current_offset += total_frame_len;
            records_count += 1;
        }

        Ok(RecoveryReport {
            recovered_records_count: records_count,
            last_sequence_tick,
            last_verified_offset,
            torn_tail_truncated_bytes: 0,
            recovered_state_root: tree.compute_state_root(),
            recovered_transition_root: current_transition_root,
            recovered_revision: tree.revision(),
        })
    }

    fn handle_tail_truncation(
        ctx: TailTruncationContext<'_>,
    ) -> Result<RecoveryReport, RecoveryError> {
        let torn_bytes = ctx.file_len - ctx.last_verified_offset;
        if ctx.auto_truncate && torn_bytes > 0 {
            ctx.file
                .set_len(ctx.last_verified_offset)
                .map_err(|e| RecoveryError::IoError(e.to_string()))?;
            ctx.file
                .sync_all()
                .map_err(|e| RecoveryError::IoError(e.to_string()))?;
        }

        Ok(RecoveryReport {
            recovered_records_count: ctx.records_count,
            last_sequence_tick: ctx.last_sequence_tick,
            last_verified_offset: ctx.last_verified_offset,
            torn_tail_truncated_bytes: if ctx.auto_truncate { torn_bytes } else { 0 },
            recovered_state_root: ctx.tree.compute_state_root(),
            recovered_transition_root: ctx.current_transition_root,
            recovered_revision: ctx.tree.revision(),
        })
    }
}
