//! ADAM-013-E: Snapshot Attestation & Portable Cryptographic Verification
//!
//! Enables independent cryptographic verification of snapshot integrity,
//! state roots, transition lineage, and portability across compatible nodes.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::snapshot::{SnapshotError, SnapshotManifest, StateSnapshot};

pub const SNAPSHOT_BUNDLE_DOMAIN_TAG: &str = "SOVEREIGN_OS:SNAPSHOT_BUNDLE:v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    Snapshot(SnapshotError),
    BundleDigestMismatch { expected: String, actual: String },
    ManifestMismatch(String),
    IoError(String),
}

impl From<SnapshotError> for ExportError {
    fn from(err: SnapshotError) -> Self {
        Self::Snapshot(err)
    }
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot(e) => write!(f, "Snapshot verification error: {}", e),
            Self::BundleDigestMismatch { expected, actual } => {
                write!(
                    f,
                    "Bundle digest mismatch: expected {}, actual {}",
                    expected, actual
                )
            }
            Self::ManifestMismatch(e) => write!(f, "Manifest mismatch: {}", e),
            Self::IoError(e) => write!(f, "Export I/O error: {}", e),
        }
    }
}

impl std::error::Error for ExportError {}

/// Portable, self-attesting snapshot bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportableSnapshotBundle {
    pub manifest_sequence_tick: u64,
    pub manifest_revision: u64,
    pub manifest_state_root: String,
    pub manifest_transition_root: String,
    pub manifest_entry_count: u64,
    pub manifest_checksum: [u8; 32],
    pub raw_snapshot_bytes: Vec<u8>,
    pub bundle_digest: String,
}

/// Standalone cryptographic attestation verified independently of runtime storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSnapshotAttestation {
    pub sequence_tick: u64,
    pub revision: u64,
    pub state_root: String,
    pub transition_root: String,
    pub entry_count: u64,
    pub bundle_digest: String,
}

pub struct SnapshotExporter;

impl SnapshotExporter {
    /// Computes the domain-separated BLAKE3 bundle attestation digest.
    pub fn compute_bundle_digest(manifest: &SnapshotManifest, raw_bytes: &[u8]) -> String {
        let mut hasher = blake3::Hasher::new();

        // 1. Domain tag
        hasher.update(&(SNAPSHOT_BUNDLE_DOMAIN_TAG.len() as u64).to_be_bytes());
        hasher.update(SNAPSHOT_BUNDLE_DOMAIN_TAG.as_bytes());

        // 2. Fixed manifest fields
        hasher.update(&manifest.sequence_tick.to_be_bytes());
        hasher.update(&manifest.revision.to_be_bytes());

        hasher.update(&(manifest.state_root.len() as u32).to_be_bytes());
        hasher.update(manifest.state_root.as_bytes());

        hasher.update(&(manifest.transition_root.len() as u32).to_be_bytes());
        hasher.update(manifest.transition_root.as_bytes());

        hasher.update(&manifest.entry_count.to_be_bytes());
        hasher.update(&manifest.checksum);

        // 3. Length-prefixed raw snapshot payload
        hasher.update(&(raw_bytes.len() as u64).to_be_bytes());
        hasher.update(raw_bytes);

        hasher.finalize().to_hex().to_string()
    }

    /// Exports a verified snapshot file into a portable attestation bundle.
    pub fn export_bundle(
        snapshot_path: impl AsRef<Path>,
    ) -> Result<ExportableSnapshotBundle, ExportError> {
        let path = snapshot_path.as_ref();
        let (_, manifest) = StateSnapshot::load_and_verify(path)?;

        let mut file = File::open(path).map_err(|e| ExportError::IoError(e.to_string()))?;
        let mut raw_bytes = Vec::new();
        file.read_to_end(&mut raw_bytes)
            .map_err(|e| ExportError::IoError(e.to_string()))?;

        let bundle_digest = Self::compute_bundle_digest(&manifest, &raw_bytes);

        Ok(ExportableSnapshotBundle {
            manifest_sequence_tick: manifest.sequence_tick,
            manifest_revision: manifest.revision,
            manifest_state_root: manifest.state_root,
            manifest_transition_root: manifest.transition_root,
            manifest_entry_count: manifest.entry_count,
            manifest_checksum: manifest.checksum,
            raw_snapshot_bytes: raw_bytes,
            bundle_digest,
        })
    }

    /// Independently verifies an attestation bundle without initializing storage or filesystem state.
    pub fn verify_bundle(
        bundle: &ExportableSnapshotBundle,
    ) -> Result<VerifiedSnapshotAttestation, ExportError> {
        let manifest = SnapshotManifest {
            sequence_tick: bundle.manifest_sequence_tick,
            revision: bundle.manifest_revision,
            state_root: bundle.manifest_state_root.clone(),
            transition_root: bundle.manifest_transition_root.clone(),
            entry_count: bundle.manifest_entry_count,
            checksum: bundle.manifest_checksum,
        };

        // 1. Verify bundle digest
        let expected_digest = Self::compute_bundle_digest(&manifest, &bundle.raw_snapshot_bytes);
        if bundle.bundle_digest != expected_digest {
            return Err(ExportError::BundleDigestMismatch {
                expected: expected_digest,
                actual: bundle.bundle_digest.clone(),
            });
        }

        // 2. Decode raw snapshot bytes, verify internal BLAKE3 checksum, and recompute StateRoot
        let temp_dir = tempfile::tempdir().map_err(|e| ExportError::IoError(e.to_string()))?;
        let temp_snap_path = temp_dir.path().join("verify.snap");
        std::fs::write(&temp_snap_path, &bundle.raw_snapshot_bytes)
            .map_err(|e| ExportError::IoError(e.to_string()))?;

        let (tree, verified_manifest) = StateSnapshot::load_and_verify(&temp_snap_path)?;

        // 3. Assert manifest concordance
        if verified_manifest != manifest {
            return Err(ExportError::ManifestMismatch(format!(
                "Manifest mismatch: bundle {:?}, decoded {:?}",
                manifest, verified_manifest
            )));
        }

        if tree.len() as u64 != manifest.entry_count {
            return Err(ExportError::ManifestMismatch(format!(
                "Entry count mismatch: manifest {}, tree {}",
                manifest.entry_count,
                tree.len()
            )));
        }

        Ok(VerifiedSnapshotAttestation {
            sequence_tick: manifest.sequence_tick,
            revision: manifest.revision,
            state_root: manifest.state_root,
            transition_root: manifest.transition_root,
            entry_count: manifest.entry_count,
            bundle_digest: bundle.bundle_digest.clone(),
        })
    }
}
