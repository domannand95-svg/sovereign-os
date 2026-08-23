//! ADAM Trial 006 — Evidence Package Boundary
//!
//! Aggregates:
//! - Raw evidence identity
//! - EvaluationReport
//! - ProvenanceManifest
//!
//! Invariant:
//!
//! Evidence Package != Execution Capability

use crate::evaluator::EvaluationReport;
use crate::provenance::ProvenanceManifest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidencePackage {
    pub package_id: String,

    pub trial_id: String,

    pub raw_evidence_hash: String,

    pub evaluation_report: EvaluationReport,

    pub provenance_manifest: ProvenanceManifest,

    pub sealed: bool,
}

impl EvidencePackage {
    pub fn validate(&self) -> Result<(), EvidencePackageError> {
        if self.trial_id.is_empty() {
            return Err(EvidencePackageError::MissingField(
                "trial_id".into(),
            ));
        }

        if self.raw_evidence_hash.len() != 64 {
            return Err(EvidencePackageError::InvalidHash(
                "raw_evidence_hash".into(),
            ));
        }

        self.provenance_manifest
            .validate()
            .map_err(|_| EvidencePackageError::InvalidManifest)?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidencePackageError {
    MissingField(String),
    InvalidHash(String),
    InvalidManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageStatus {
    Unsealed,
    Sealed,
}

impl EvidencePackage {
    pub fn new(
        provenance_manifest: ProvenanceManifest,
        evaluation_report: EvaluationReport,
        trace: serde_json::Value,
    ) -> Self {
        let raw_bytes = serde_json::to_vec(&trace)
            .expect("trace serialization must succeed");

        let raw_evidence_hash =
            blake3::hash(&raw_bytes).to_hex().to_string();

        Self {
            package_id: format!("adam_trial_006_{}", raw_evidence_hash),
            trial_id: "ADAM-TRIAL-006".to_string(),
            raw_evidence_hash,
            evaluation_report,
            provenance_manifest,
            sealed: false,
        }
    }

    pub fn seal(mut self) -> Result<Self, EvidencePackageError> {
        self.validate()?;
        self.sealed = true;
        Ok(self)
    }
}

