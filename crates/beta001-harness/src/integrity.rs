//! BETA-001-T5.4 â€” State Integrity Plane
//!
//! Captures pre- and post-execution state surface snapshots and verifies that
//! protected and governed state surfaces remain unmutated.

use crate::runtime_profile::{StateSurfaceKind, T5RuntimeProfile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version constant for the integrity plane schema.
pub const INTEGRITY_PLANE_VERSION: u16 = 1;

/// Snapshot of an individual state surface before or after execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateSurfaceSnapshot {
    pub path: String,
    pub kind: StateSurfaceKind,
    pub content_hash: Option<String>,
    pub exists: bool,
}

/// Comprehensive report comparing pre- and post-execution state surface snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub version: u16,
    pub pre_snapshots: BTreeMap<String, StateSurfaceSnapshot>,
    pub post_snapshots: BTreeMap<String, StateSurfaceSnapshot>,
    pub mutated_surfaces: Vec<String>,
    pub is_intact: bool,
}

/// Abstract reader for inspecting state surfaces during snapshotting.
pub trait StateSurfaceReader {
    fn read_bytes(&self, path: &str) -> Option<Vec<u8>>;
    fn exists(&self, path: &str) -> bool;
}

/// Computes a deterministic canonical digest string for a byte sequence.
pub fn compute_surface_digest(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Captures snapshots for all state surfaces declared in the runtime profile.
pub fn capture_surface_snapshots(
    profile: &T5RuntimeProfile,
    reader: &impl StateSurfaceReader,
) -> BTreeMap<String, StateSurfaceSnapshot> {
    let mut snapshots = BTreeMap::new();

    // 1. Snapshot explicit protected state surfaces
    for surface in profile.protected_state_surfaces() {
        let path_str = surface.as_str();
        let exists = reader.exists(path_str);
        let content_hash = if exists {
            reader
                .read_bytes(path_str)
                .as_deref()
                .map(compute_surface_digest)
        } else {
            None
        };

        snapshots.insert(
            path_str.to_string(),
            StateSurfaceSnapshot {
                path: path_str.to_string(),
                kind: StateSurfaceKind::Repository,
                content_hash,
                exists,
            },
        );
    }

    // 2. Snapshot general state surface inventory
    for surface in profile.state_surface_inventory() {
        if let Some(p) = surface.path() {
            let path_str = p.as_str();
            if !snapshots.contains_key(path_str) {
                let exists = reader.exists(path_str);
                let content_hash = if exists {
                    reader
                        .read_bytes(path_str)
                        .as_deref()
                        .map(compute_surface_digest)
                } else {
                    None
                };

                snapshots.insert(
                    path_str.to_string(),
                    StateSurfaceSnapshot {
                        path: path_str.to_string(),
                        kind: surface.kind(),
                        content_hash,
                        exists,
                    },
                );
            }
        }
    }

    snapshots
}

/// Compares pre- and post-run snapshots to detect unauthorized state mutations.
pub fn evaluate_integrity(
    pre: BTreeMap<String, StateSurfaceSnapshot>,
    post: BTreeMap<String, StateSurfaceSnapshot>,
) -> IntegrityReport {
    let mut mutated = Vec::new();

    for (path, pre_snap) in &pre {
        match post.get(path) {
            Some(post_snap) => {
                if pre_snap.exists != post_snap.exists
                    || pre_snap.content_hash != post_snap.content_hash
                {
                    mutated.push(path.clone());
                }
            }
            None => {
                mutated.push(path.clone());
            }
        }
    }

    for path in post.keys() {
        if !pre.contains_key(path) {
            mutated.push(path.clone());
        }
    }

    let is_intact = mutated.is_empty();

    IntegrityReport {
        version: INTEGRITY_PLANE_VERSION,
        pre_snapshots: pre,
        post_snapshots: post,
        mutated_surfaces: mutated,
        is_intact,
    }
}
