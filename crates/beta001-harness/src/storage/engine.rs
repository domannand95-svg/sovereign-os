//! ADAM-013-D: Unified Storage Engine, Cold-Start Hydration & Crash-Safe Compaction
//!
//! Coordinates state tree hydration from verified snapshots and commit logs,
//! manages append-only durability frontiers, and enforces the 9-step crash-safe compaction rule.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::frame::{CommitLogFrame, CommitRecordPayload, FrameError};
use super::log::{
    CommitLogWriter, DurabilityAcknowledgement, SyncPolicy, DEFAULT_MAX_FRAME_PAYLOAD_BYTES,
};
use super::recovery::{CommitLogRecovery, RecoveryError};
use super::snapshot::{SnapshotError, SnapshotManifest, StateSnapshot};
use crate::state::{compute_genesis_transition_root, StateTree};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    Snapshot(SnapshotError),
    Recovery(RecoveryError),
    Frame(FrameError),
    SequenceRegression { expected: u64, attempted: u64 },
    StateDivergence(String),
    IoError(String),
}

impl From<SnapshotError> for EngineError {
    fn from(err: SnapshotError) -> Self {
        Self::Snapshot(err)
    }
}

impl From<RecoveryError> for EngineError {
    fn from(err: RecoveryError) -> Self {
        Self::Recovery(err)
    }
}

impl From<FrameError> for EngineError {
    fn from(err: FrameError) -> Self {
        Self::Frame(err)
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot(e) => write!(f, "Snapshot error: {}", e),
            Self::Recovery(e) => write!(f, "Recovery error: {}", e),
            Self::Frame(e) => write!(f, "Frame error: {}", e),
            Self::SequenceRegression {
                expected,
                attempted,
            } => {
                write!(
                    f,
                    "Sequence regression: expected {}, attempted {}",
                    expected, attempted
                )
            }
            Self::StateDivergence(e) => write!(f, "State divergence: {}", e),
            Self::IoError(e) => write!(f, "Storage engine I/O error: {}", e),
        }
    }
}

impl std::error::Error for EngineError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionReport {
    pub snapshot_manifest: SnapshotManifest,
    pub pruned_records_count: usize,
    pub retained_records_count: usize,
}

pub struct StorageEngine {
    storage_dir: PathBuf,
    log_path: PathBuf,
    writer: Mutex<CommitLogWriter>,
    policy: SyncPolicy,
    tree: Mutex<StateTree>,
    sequence_tick: Mutex<u64>,
    transition_root: Mutex<String>,
    latest_snapshot: Mutex<Option<SnapshotManifest>>,
}

impl StorageEngine {
    /// Opens or recovers the storage engine from the designated directory.
    pub fn open(storage_dir: impl AsRef<Path>, policy: SyncPolicy) -> Result<Self, EngineError> {
        let dir_buf = storage_dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir_buf).map_err(|e| EngineError::IoError(e.to_string()))?;

        // 1. Discover and select the highest fully verified candidate snapshot
        let (mut tree, snapshot_manifest_opt) = Self::discover_latest_verified_snapshot(&dir_buf)?;
        let snapshot_tick = snapshot_manifest_opt
            .as_ref()
            .map(|m| m.sequence_tick)
            .unwrap_or(0);
        let snapshot_trans_root = snapshot_manifest_opt
            .as_ref()
            .map(|m| m.transition_root.clone())
            .unwrap_or_else(compute_genesis_transition_root);

        // 2. Replay commit log strictly beyond the snapshot sequence tick frontier
        let log_path = dir_buf.join("commit.log");
        let recovery_report = CommitLogRecovery::recover_from_log_with_frontier(
            &log_path,
            &mut tree,
            snapshot_tick,
            Some(snapshot_trans_root.clone()),
            true,
        )?;

        let final_tick = if recovery_report.recovered_records_count > 0 {
            recovery_report.last_sequence_tick
        } else {
            snapshot_tick
        };

        let final_trans_root = recovery_report.recovered_transition_root;

        // 3. Open append-only commit log writer
        let writer = CommitLogWriter::open(&log_path, policy)?;

        Ok(Self {
            storage_dir: dir_buf,
            log_path,
            writer: Mutex::new(writer),
            policy,
            tree: Mutex::new(tree),
            sequence_tick: Mutex::new(final_tick),
            transition_root: Mutex::new(final_trans_root),
            latest_snapshot: Mutex::new(snapshot_manifest_opt),
        })
    }

    /// Appends a verified commit record, advances state and lineage, and satisfies durability frontier.
    pub fn commit_record(
        &self,
        sequence_tick: u64,
        payload: CommitRecordPayload,
    ) -> Result<DurabilityAcknowledgement, EngineError> {
        let mut tree_guard = self.tree.lock().unwrap();
        let mut tick_guard = self.sequence_tick.lock().unwrap();
        let mut trans_guard = self.transition_root.lock().unwrap();
        let writer_guard = self.writer.lock().unwrap();

        if sequence_tick != *tick_guard + 1 {
            return Err(EngineError::SequenceRegression {
                expected: *tick_guard + 1,
                attempted: sequence_tick,
            });
        }

        let pre_root = tree_guard.compute_state_root();
        if payload.pre_state_root != pre_root {
            return Err(EngineError::StateDivergence(format!(
                "Pre-state root mismatch: expected {}, tree {}",
                payload.pre_state_root, pre_root
            )));
        }

        if payload.prior_transition_root != *trans_guard {
            return Err(EngineError::StateDivergence(format!(
                "Prior transition root mismatch: expected {}, engine {}",
                payload.prior_transition_root, *trans_guard
            )));
        }

        // Apply mutations to in-memory state tree
        tree_guard.apply_raw_mutations(&payload.mutations);
        let post_root = tree_guard.compute_state_root();
        if payload.post_state_root != post_root {
            return Err(EngineError::StateDivergence(format!(
                "Post-state root mismatch: expected {}, tree {}",
                payload.post_state_root, post_root
            )));
        }

        // Persist frame with durability guarantee
        let ack = writer_guard.append(sequence_tick, payload.clone())?;

        *tick_guard = sequence_tick;
        *trans_guard = payload.transition_root;

        Ok(ack)
    }

    /// Creates and publishes a verified, durable state snapshot at the current sequence tick.
    pub fn create_snapshot(&self) -> Result<SnapshotManifest, EngineError> {
        let tree_guard = self.tree.lock().unwrap();
        let tick = *self.sequence_tick.lock().unwrap();
        let trans_root = self.transition_root.lock().unwrap().clone();

        let snap_filename = format!("state_seq_{:020}.snap", tick);
        let snap_path = self.storage_dir.join(snap_filename);

        let manifest = StateSnapshot::write_atomic(&snap_path, &tree_guard, tick, &trans_root)?;
        *self.latest_snapshot.lock().unwrap() = Some(manifest.clone());

        Ok(manifest)
    }

    /// Executes crash-safe compaction: creates snapshot, prunes subsumed commit log prefix, and syncs directory.
    pub fn compact(&self) -> Result<CompactionReport, EngineError> {
        let mut writer_guard = self.writer.lock().unwrap();
        let tree_guard = self.tree.lock().unwrap();
        let current_tick = *self.sequence_tick.lock().unwrap();
        let trans_root = self.transition_root.lock().unwrap().clone();

        // 1. Durably publish snapshot at current tick
        let snap_filename = format!("state_seq_{:020}.snap", current_tick);
        let snap_path = self.storage_dir.join(snap_filename);
        let manifest =
            StateSnapshot::write_atomic(&snap_path, &tree_guard, current_tick, &trans_root)?;

        // 2. Read existing commit log frames to identify suffix strictly beyond current_tick
        let mut retained_frames = Vec::new();
        let mut pruned_count = 0usize;

        if self.log_path.exists() {
            let mut file =
                File::open(&self.log_path).map_err(|e| EngineError::IoError(e.to_string()))?;
            let file_len = file
                .metadata()
                .map_err(|e| EngineError::IoError(e.to_string()))?
                .len();
            let mut offset = 0u64;

            while offset < file_len {
                let frame = CommitLogFrame::read_from(&mut file, DEFAULT_MAX_FRAME_PAYLOAD_BYTES)?;
                let total_len = CommitLogFrame::HEADER_SIZE as u64
                    + frame.payload.encode_canonical().len() as u64
                    + CommitLogFrame::CHECKSUM_SIZE as u64;

                if frame.sequence_tick <= current_tick {
                    pruned_count += 1;
                } else {
                    retained_frames.push(frame);
                }
                offset += total_len;
            }
        }

        // 3. Write retained frames to temp log file
        let tmp_log_path = self.storage_dir.join("commit.log.compact_tmp");
        {
            let mut tmp_file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_log_path)
                .map_err(|e| EngineError::IoError(e.to_string()))?;

            for frame in &retained_frames {
                frame.write_to(&mut tmp_file)?;
            }

            tmp_file
                .flush()
                .map_err(|e| EngineError::IoError(e.to_string()))?;
            tmp_file
                .sync_all()
                .map_err(|e| EngineError::IoError(e.to_string()))?;
        }

        // 4. Drop current writer and atomically replace commit.log
        drop(std::mem::replace(
            &mut *writer_guard,
            CommitLogWriter::open(&tmp_log_path, self.policy)?,
        ));

        fs::rename(&tmp_log_path, &self.log_path)
            .map_err(|e| EngineError::IoError(e.to_string()))?;

        // 5. Sync directory metadata and reopen writer
        Self::sync_directory(&self.storage_dir).map_err(|e| EngineError::IoError(e.to_string()))?;
        *writer_guard = CommitLogWriter::open(&self.log_path, self.policy)?;
        *self.latest_snapshot.lock().unwrap() = Some(manifest.clone());

        Ok(CompactionReport {
            snapshot_manifest: manifest,
            pruned_records_count: pruned_count,
            retained_records_count: retained_frames.len(),
        })
    }

    /// Discovers candidate `.snap` files, verifies their cryptographic integrity, and returns the highest valid state.
    fn discover_latest_verified_snapshot(
        dir: &Path,
    ) -> Result<(StateTree, Option<SnapshotManifest>), EngineError> {
        let mut candidates = Vec::new();

        if dir.exists() {
            let entries = fs::read_dir(dir).map_err(|e| EngineError::IoError(e.to_string()))?;
            for entry in entries {
                let entry = entry.map_err(|e| EngineError::IoError(e.to_string()))?;
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("snap") {
                    if let Ok((tree, manifest)) = StateSnapshot::load_and_verify(&path) {
                        candidates.push((
                            manifest.sequence_tick,
                            manifest.revision,
                            tree,
                            manifest,
                        ));
                    }
                }
            }
        }

        if let Some((_, _, best_tree, best_manifest)) = candidates
            .into_iter()
            .max_by_key(|(tick, rev, _, _)| (*tick, *rev))
        {
            Ok((best_tree, Some(best_manifest)))
        } else {
            Ok((StateTree::new(), None))
        }
    }

    fn sync_directory(dir: &Path) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            let f = File::open(dir)?;
            f.sync_all()?;
        }
        #[cfg(windows)]
        {
            let _ = dir;
        }
        Ok(())
    }

    pub fn current_state_root(&self) -> String {
        self.tree.lock().unwrap().compute_state_root()
    }

    pub fn current_transition_root(&self) -> String {
        self.transition_root.lock().unwrap().clone()
    }

    pub fn current_sequence_tick(&self) -> u64 {
        *self.sequence_tick.lock().unwrap()
    }

    pub fn current_revision(&self) -> u64 {
        self.tree.lock().unwrap().revision()
    }

    pub fn get_state_value(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.tree.lock().unwrap().get(key).map(|v| v.to_vec())
    }
}
