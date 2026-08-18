//! BETA-001-T5.4 â€” Evidence Package & Sealing Plane
//!
//! Assembles independent evidence planes (Candidate, Containment, Integrity, Evaluation)
//! into a sealed, immutable EvidencePackage with deterministic canonical serialization.

use crate::integrity::IntegrityReport;
use crate::telemetry::ContainmentTelemetry;
use serde::{Deserialize, Serialize};

/// Version constant for the authoritative Evidence Package schema.
pub const EVIDENCE_PACKAGE_VERSION: u16 = 1;

/// Parsing status of raw model output within the candidate plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateParseStatus {
    Parsed,
    Malformed,
    Unsubmitted,
}

/// Provenance metadata header identifying the run context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceHeader {
    pub package_version: u16,
    pub run_id: String,
    pub baseline_commit: String,
    pub profile_version: String,
}

/// Candidate plane capturing raw agent output and optional parsed candidate JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePlane {
    pub raw_agent_output: String,
    pub parsed_candidate_json: Option<String>,
    pub parse_status: CandidateParseStatus,
    pub candidate_digest: Option<String>,
}

/// Containment plane capturing independent observation telemetry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainmentPlane {
    pub telemetry: ContainmentTelemetry,
    pub is_clean: bool,
}

/// Integrity plane capturing pre- and post-run state surface snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityPlane {
    pub report: IntegrityReport,
    pub is_intact: bool,
}

/// Evaluation plane capturing deterministic evaluator output and disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationPlane {
    pub evaluation_report_json: Option<String>,
    pub disposition: Option<String>,
}

/// The authoritative multi-plane Evidence Package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidencePackage {
    pub provenance: ProvenanceHeader,
    pub candidate_plane: CandidatePlane,
    pub containment_plane: ContainmentPlane,
    pub integrity_plane: IntegrityPlane,
    pub evaluation_plane: EvaluationPlane,
}

/// Failure conditions indicating why a run cannot produce a valid sealed package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunInvalidation {
    MissingCandidateOutput,
    MissingContainmentTelemetry,
    MissingIntegrityReport,
    InconsistentPlaneVersion { expected: u16, found: u16 },
    SerializationError(String),
}

impl std::fmt::Display for RunInvalidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCandidateOutput => write!(f, "missing candidate raw output"),
            Self::MissingContainmentTelemetry => write!(f, "missing containment telemetry"),
            Self::MissingIntegrityReport => write!(f, "missing integrity report"),
            Self::InconsistentPlaneVersion { expected, found } => {
                write!(
                    f,
                    "plane version mismatch: expected {expected}, found {found}"
                )
            }
            Self::SerializationError(msg) => write!(f, "canonical serialization failed: {msg}"),
        }
    }
}

impl std::error::Error for RunInvalidation {}

/// Sealed, read-only Evidence Package with verified canonical byte serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedEvidencePackage {
    package: EvidencePackage,
    canonical_bytes: Vec<u8>,
    package_digest: String,
}

impl SealedEvidencePackage {
    /// Returns a reference to the inner sealed EvidencePackage.
    pub fn package(&self) -> &EvidencePackage {
        &self.package
    }

    /// Returns the exact canonical serialized bytes of the package.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the deterministic digest of the sealed package.
    pub fn digest(&self) -> &str {
        &self.package_digest
    }
}

/// Mutable collector for assembling planes before final validation and sealing.
#[derive(Debug, Clone)]
pub struct EvidenceCollector {
    pub provenance: ProvenanceHeader,
    pub candidate_plane: Option<CandidatePlane>,
    pub containment_plane: Option<ContainmentPlane>,
    pub integrity_plane: Option<IntegrityPlane>,
    pub evaluation_plane: Option<EvaluationPlane>,
}

impl EvidenceCollector {
    /// Initializes a fresh collector with the required run provenance.
    pub fn new(run_id: String, baseline_commit: String, profile_version: String) -> Self {
        Self {
            provenance: ProvenanceHeader {
                package_version: EVIDENCE_PACKAGE_VERSION,
                run_id,
                baseline_commit,
                profile_version,
            },
            candidate_plane: None,
            containment_plane: None,
            integrity_plane: None,
            evaluation_plane: None,
        }
    }

    /// Sets the candidate plane record.
    pub fn set_candidate_plane(
        &mut self,
        raw_agent_output: String,
        parsed_candidate_json: Option<String>,
        parse_status: CandidateParseStatus,
    ) {
        let candidate_digest = parsed_candidate_json
            .as_ref()
            .map(|json| crate::integrity::compute_surface_digest(json.as_bytes()));

        self.candidate_plane = Some(CandidatePlane {
            raw_agent_output,
            parsed_candidate_json,
            parse_status,
            candidate_digest,
        });
    }

    /// Sets the containment plane record from captured telemetry.
    pub fn set_containment_plane(&mut self, telemetry: ContainmentTelemetry) {
        let is_clean = telemetry.is_clean;
        self.containment_plane = Some(ContainmentPlane {
            telemetry,
            is_clean,
        });
    }

    /// Sets the integrity plane record from captured report.
    pub fn set_integrity_plane(&mut self, report: IntegrityReport) {
        let is_intact = report.is_intact;
        self.integrity_plane = Some(IntegrityPlane { report, is_intact });
    }

    /// Sets the evaluation plane record.
    pub fn set_evaluation_plane(
        &mut self,
        evaluation_report_json: Option<String>,
        disposition: Option<String>,
    ) {
        self.evaluation_plane = Some(EvaluationPlane {
            evaluation_report_json,
            disposition,
        });
    }

    /// Validates completeness and seals the package into an immutable representation.
    pub fn seal(self) -> Result<SealedEvidencePackage, RunInvalidation> {
        let candidate_plane = self
            .candidate_plane
            .ok_or(RunInvalidation::MissingCandidateOutput)?;
        let containment_plane = self
            .containment_plane
            .ok_or(RunInvalidation::MissingContainmentTelemetry)?;
        let integrity_plane = self
            .integrity_plane
            .ok_or(RunInvalidation::MissingIntegrityReport)?;

        let evaluation_plane = self.evaluation_plane.unwrap_or(EvaluationPlane {
            evaluation_report_json: None,
            disposition: None,
        });

        let package = EvidencePackage {
            provenance: self.provenance,
            candidate_plane,
            containment_plane,
            integrity_plane,
            evaluation_plane,
        };

        let canonical_bytes = serde_json::to_vec(&package)
            .map_err(|e| RunInvalidation::SerializationError(e.to_string()))?;

        let package_digest = crate::integrity::compute_surface_digest(&canonical_bytes);

        Ok(SealedEvidencePackage {
            package,
            canonical_bytes,
            package_digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::IntegrityReport;
    use crate::telemetry::ContainmentTelemetry;
    use std::collections::BTreeMap;

    fn sample_integrity_report() -> IntegrityReport {
        IntegrityReport {
            version: crate::integrity::INTEGRITY_PLANE_VERSION,
            pre_snapshots: BTreeMap::new(),
            post_snapshots: BTreeMap::new(),
            mutated_surfaces: Vec::new(),
            is_intact: true,
        }
    }

    #[test]
    fn test_seal_succeeds_when_all_planes_present() {
        let mut collector = EvidenceCollector::new(
            "run-001".to_string(),
            "0c1da529f9be5fe1b609fc25193ecb9cd72a6c2e".to_string(),
            "t5-profile-v1".to_string(),
        );

        collector.set_candidate_plane(
            "{\"trace_id\":\"abc\"}".to_string(),
            Some("{\"trace_id\":\"abc\"}".to_string()),
            CandidateParseStatus::Parsed,
        );
        collector.set_containment_plane(ContainmentTelemetry::new());
        collector.set_integrity_plane(sample_integrity_report());

        let sealed = collector.seal();
        assert!(sealed.is_ok());
        let package = sealed.unwrap();
        assert_eq!(package.package().provenance.run_id, "run-001");
        assert!(!package.digest().is_empty());
    }

    #[test]
    fn test_seal_fails_on_missing_containment_telemetry() {
        let mut collector = EvidenceCollector::new(
            "run-002".to_string(),
            "0c1da529f9be5fe1b609fc25193ecb9cd72a6c2e".to_string(),
            "t5-profile-v1".to_string(),
        );

        collector.set_candidate_plane(
            "raw-output".to_string(),
            None,
            CandidateParseStatus::Malformed,
        );
        collector.set_integrity_plane(sample_integrity_report());

        let sealed = collector.seal();
        assert!(matches!(
            sealed,
            Err(RunInvalidation::MissingContainmentTelemetry)
        ));
    }

    #[test]
    fn test_malformed_raw_output_preserved_verbatim() {
        let mut collector = EvidenceCollector::new(
            "run-003".to_string(),
            "0c1da529f9be5fe1b609fc25193ecb9cd72a6c2e".to_string(),
            "t5-profile-v1".to_string(),
        );

        let unparseable_text = "ERROR: raw unparseable output buffer {not json}";
        collector.set_candidate_plane(
            unparseable_text.to_string(),
            None,
            CandidateParseStatus::Malformed,
        );
        collector.set_containment_plane(ContainmentTelemetry::new());
        collector.set_integrity_plane(sample_integrity_report());

        let sealed = collector.seal().unwrap();
        assert_eq!(
            sealed.package().candidate_plane.raw_agent_output,
            unparseable_text
        );
        assert_eq!(
            sealed.package().candidate_plane.parse_status,
            CandidateParseStatus::Malformed
        );
        assert!(sealed
            .package()
            .candidate_plane
            .parsed_candidate_json
            .is_none());
    }

    #[test]
    fn test_deterministic_sealing_and_digesting() {
        let make_collector = || {
            let mut collector = EvidenceCollector::new(
                "run-det".to_string(),
                "0c1da529".to_string(),
                "profile-v1".to_string(),
            );
            collector.set_candidate_plane(
                "output".to_string(),
                Some("{}".to_string()),
                CandidateParseStatus::Parsed,
            );
            collector.set_containment_plane(ContainmentTelemetry::new());
            collector.set_integrity_plane(sample_integrity_report());
            collector
        };

        let sealed1 = make_collector().seal().unwrap();
        let sealed2 = make_collector().seal().unwrap();

        assert_eq!(sealed1.canonical_bytes(), sealed2.canonical_bytes());
        assert_eq!(sealed1.digest(), sealed2.digest());
    }
}
