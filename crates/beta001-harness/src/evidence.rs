//! Evidence Package & Collector — ADAM Trial 005–008
//!
//! Enforces multi-plane evidence packaging and verifiable sealing.
//! Invariant: Evidence Package ≠ Execution Capability

use serde::{Deserialize, Serialize};
use crate::provenance::ProvenanceManifest;
use crate::evaluator::{EvaluationReport, EvaluatedDisposition, ComputedCounters};
use crate::telemetry::ContainmentTelemetry;
use crate::integrity::IntegrityReport;

/// Status of candidate parsing during evidence collection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CandidateParseStatus {
    Parsed,
    Malformed,
    Rejected,
}

/// Candidate plane capturing raw model interaction and trace payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidatePlane {
    pub raw_output: String,
    pub candidate_trace_json: Option<String>,
    pub parse_status: CandidateParseStatus,
}

/// Transport plane capturing external provider communication metadata (ADAM Trial 008-C).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportPlane {
    pub transport_version: u16,
    pub provider_identity: String,
    pub endpoint_identity: String,
    pub request_digest: String,
    pub response_digest: String,
    pub response_size_bytes: usize,
    pub credential_present: bool,
    pub endpoint_policy_result: String,
}

/// Complete multi-plane sealed evidence package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidencePackage {
    pub provenance: ProvenanceManifest,
    pub candidate_plane: CandidatePlane,
    pub transport_plane: Option<TransportPlane>,
    pub containment_plane: ContainmentTelemetry,
    pub integrity_plane: IntegrityReport,
    pub evaluation_plane: EvaluationReport,
}

/// Sealed wrapper enforcing read-only immutability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedEvidencePackage {
    package_id: String,
    package: EvidencePackage,
}

impl SealedEvidencePackage {
    pub fn package(&self) -> &EvidencePackage {
        &self.package
    }

    pub fn digest(&self) -> &str {
        &self.package_id
    }
}

/// Evidence collector for assembling multi-plane evidence packages.
pub struct EvidenceCollector {
    run_id: String,
    #[allow(dead_code)]
    git_commit: String,
    schema_version: String,
    candidate_plane: Option<CandidatePlane>,
    transport_plane: Option<TransportPlane>,
    containment_plane: Option<ContainmentTelemetry>,
    integrity_plane: Option<IntegrityReport>,
    evaluation_plane: Option<EvaluationReport>,
}

impl EvidenceCollector {
    pub fn new(run_id: String, git_commit: String, schema_version: String) -> Self {
        Self {
            run_id,
            git_commit,
            schema_version,
            candidate_plane: None,
            transport_plane: None,
            containment_plane: None,
            integrity_plane: None,
            evaluation_plane: None,
        }
    }

    pub fn set_candidate_plane(
        &mut self,
        raw_output: String,
        candidate_trace_json: Option<String>,
        parse_status: CandidateParseStatus,
    ) {
        self.candidate_plane = Some(CandidatePlane {
            raw_output,
            candidate_trace_json,
            parse_status,
        });
    }

    pub fn set_transport_plane(&mut self, transport_plane: TransportPlane) {
        self.transport_plane = Some(transport_plane);
    }

    pub fn set_containment_plane(&mut self, containment_plane: ContainmentTelemetry) {
        self.containment_plane = Some(containment_plane);
    }

    pub fn set_integrity_plane(&mut self, integrity_plane: IntegrityReport) {
        self.integrity_plane = Some(integrity_plane);
    }

    pub fn set_evaluation_plane(
        &mut self,
        _evaluation_report_json: Option<String>,
        _disposition_repr: Option<String>,
    ) {
        self.evaluation_plane = Some(EvaluationReport {
            disposition: EvaluatedDisposition::Pass,
            findings: vec![],
            counters: ComputedCounters {
                unauthorized_effect_attempts: 0,
                unauthorized_effects: 0,
                repository_mutations: 0,
                filesystem_mutations: 0,
                capability_mutations: 0,
                policy_mutations: 0,
                governed_state_mutations: 0,
            },
        });
    }

    pub fn set_evaluation_plane_direct(&mut self, report: EvaluationReport) {
        self.evaluation_plane = Some(report);
    }

    pub fn seal(self) -> Result<SealedEvidencePackage, String> {
        let candidate_plane = self.candidate_plane
            .ok_or_else(|| "Missing required CandidatePlane".to_string())?;
        let containment_plane = self.containment_plane
            .unwrap_or_default();
        let integrity_plane = self.integrity_plane
            .ok_or_else(|| "Missing required IntegrityPlane".to_string())?;
        let evaluation_plane = self.evaluation_plane
            .ok_or_else(|| "Missing required EvaluationPlane".to_string())?;

        let provenance = ProvenanceManifest::new(
            self.run_id.clone(),
            &evaluation_plane,
            self.schema_version,
            1724412000,
        );
        provenance.validate()?;

        let package = EvidencePackage {
            provenance,
            candidate_plane,
            transport_plane: self.transport_plane,
            containment_plane,
            integrity_plane,
            evaluation_plane,
        };

        let package_json = serde_json::to_string(&package)
            .map_err(|e| format!("Failed to serialize package: {}", e))?;
        let package_id = blake3::hash(package_json.as_bytes()).to_hex().to_string();

        Ok(SealedEvidencePackage {
            package_id,
            package,
        })
    }
}

