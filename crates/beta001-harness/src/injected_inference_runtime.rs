use crate::raw_output_adapter::{RawOutputAdapter, RawOutputAdmissionResult};
use crate::runtime_profile::T5RuntimeProfile;
use crate::telemetry::ContainmentTelemetry;

/// Deterministic result of evaluating already-produced, untrusted inference bytes.
///
/// This first EXP-BETA-003 slice performs no provider invocation and grants no
/// filesystem, shell, network, tool, or governed-state authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InjectedInferenceRuntimeResult {
    pub adapter_result: RawOutputAdmissionResult,
    pub telemetry: ContainmentTelemetry,
}

/// Evaluates injected inference bytes through the frozen EXP-BETA-002 admission
/// boundary without invoking an inference provider or performing host effects.
///
/// `T5RuntimeProfile` is accepted as the governing runtime constraint object.
/// This initial injected-byte slice performs no effectful operation for the
/// profile to authorize; model-output admission remains exclusively delegated
/// to `RawOutputAdapter::admit`.
pub fn evaluate_injected_inference(
    _profile: &T5RuntimeProfile,
    raw_output: &[u8],
) -> InjectedInferenceRuntimeResult {
    let adapter_result = RawOutputAdapter::admit(raw_output);

    InjectedInferenceRuntimeResult {
        adapter_result,
        telemetry: ContainmentTelemetry::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceBoundaryBlocker {
    RawBytesNotRepresentableAsCandidateString,
    MissingIntegrityObservation,
    MissingContainmentObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceBoundaryDisposition {
    NotSealEligible {
        blockers: Vec<EvidenceBoundaryBlocker>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InjectedInferenceEvidenceBoundaryResult {
    pub runtime_result: InjectedInferenceRuntimeResult,
    pub disposition: EvidenceBoundaryDisposition,
}

/// Evaluates the first-slice evidence boundary without fabricating evidence
/// observations that have not occurred.
///
/// UTF-8 validation here is solely a representability check for the existing
/// string-backed candidate evidence plane. It does not alter, normalize, or
/// replace the preserved raw inference bytes and does not affect admission.
pub fn evaluate_injected_inference_evidence_boundary(
    profile: &T5RuntimeProfile,
    raw_output: &[u8],
) -> InjectedInferenceEvidenceBoundaryResult {
    let runtime_result = evaluate_injected_inference(profile, raw_output);
    let mut blockers = Vec::new();

    if std::str::from_utf8(raw_output).is_err() {
        blockers.push(EvidenceBoundaryBlocker::RawBytesNotRepresentableAsCandidateString);
    }

    blockers.push(EvidenceBoundaryBlocker::MissingIntegrityObservation);
    blockers.push(EvidenceBoundaryBlocker::MissingContainmentObservation);

    InjectedInferenceEvidenceBoundaryResult {
        runtime_result,
        disposition: EvidenceBoundaryDisposition::NotSealEligible { blockers },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationContractBlocker {
    MissingExecutionBinding,
    MissingIntegrityObservation,
    MissingContainmentObservation,
    ExecutionBindingMismatch,
    IntegrityCoverageIncomplete,
    IntegrityReportInconsistent,
    IntegrityChanged,
    ContainmentCoverageIncomplete,
    ContainmentBreached,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationContractDisposition {
    Satisfied,
    NotSatisfied {
        blockers: Vec<ObservationContractBlocker>,
    },
}

/// Evaluates the EXP-BETA-003 governed observation boundary.
///
/// Legacy containment and integrity values do not themselves prove execution
/// binding, integrity observation, or containment observation. Until those
/// observation capabilities are represented explicitly, this boundary fails
/// closed and cannot report a satisfied observation contract.
pub fn evaluate_governed_observation_contract(
    _telemetry: &ContainmentTelemetry,
    _integrity: &crate::integrity::IntegrityReport,
) -> ObservationContractDisposition {
    ObservationContractDisposition::NotSatisfied {
        blockers: vec![
            ObservationContractBlocker::MissingExecutionBinding,
            ObservationContractBlocker::MissingIntegrityObservation,
            ObservationContractBlocker::MissingContainmentObservation,
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionId(String);

impl ExecutionId {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();

        if value.is_empty() {
            return Err("execution identity must not be empty");
        }

        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainmentCoverageStatus {
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernedContainmentRecord {
    execution_id: ExecutionId,
    coverage: ContainmentCoverageStatus,
    telemetry: ContainmentTelemetry,
}

impl GovernedContainmentRecord {
    pub fn new(
        execution_id: ExecutionId,
        coverage: ContainmentCoverageStatus,
        telemetry: ContainmentTelemetry,
    ) -> Self {
        Self {
            execution_id,
            coverage,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernedIntegrityRecord {
    execution_id: ExecutionId,
    report: crate::integrity::IntegrityReport,
}

impl GovernedIntegrityRecord {
    pub fn new(execution_id: ExecutionId, report: crate::integrity::IntegrityReport) -> Self {
        Self {
            execution_id,
            report,
        }
    }
}

fn classify_integrity_observation(
    profile: &T5RuntimeProfile,
    report: &crate::integrity::IntegrityReport,
) -> Option<ObservationContractBlocker> {
    if report.version != crate::integrity::INTEGRITY_PLANE_VERSION {
        return Some(ObservationContractBlocker::IntegrityReportInconsistent);
    }

    let mut required_paths = std::collections::BTreeSet::new();

    for path in profile.protected_state_surfaces() {
        required_paths.insert(path.as_str().to_owned());
    }

    for surface in profile.state_surface_inventory() {
        if surface.evidence_treatment()
            == crate::runtime_profile::EvidenceTreatment::PrePostIntegrity
        {
            let Some(path) = surface.path() else {
                return Some(ObservationContractBlocker::IntegrityCoverageIncomplete);
            };

            required_paths.insert(path.as_str().to_owned());
        }
    }

    if required_paths.is_empty() {
        return Some(ObservationContractBlocker::IntegrityCoverageIncomplete);
    }

    for path in &required_paths {
        if !report.pre_snapshots.contains_key(path) || !report.post_snapshots.contains_key(path) {
            return Some(ObservationContractBlocker::IntegrityCoverageIncomplete);
        }
    }

    let recomputed = crate::integrity::evaluate_integrity(
        report.pre_snapshots.clone(),
        report.post_snapshots.clone(),
    );

    if report.is_intact != recomputed.is_intact
        || report.mutated_surfaces != recomputed.mutated_surfaces
    {
        return Some(ObservationContractBlocker::IntegrityReportInconsistent);
    }

    if !recomputed.is_intact {
        return Some(ObservationContractBlocker::IntegrityChanged);
    }

    None
}
/// Evaluates structurally bound EXP-BETA-003 observation records.
///
/// Execution identity is correlation only. It is not cryptographic provenance.
/// Satisfied denotes observation-contract satisfaction only; EligibleForSealing is not implemented.
pub fn evaluate_bound_governed_observation_contract(
    profile: &T5RuntimeProfile,
    execution_id: &ExecutionId,
    integrity: Option<&GovernedIntegrityRecord>,
    containment: Option<&GovernedContainmentRecord>,
) -> ObservationContractDisposition {
    let integrity_binding_mismatch = integrity
        .map(|record| &record.execution_id != execution_id)
        .unwrap_or(false);

    let containment_binding_mismatch = containment
        .map(|record| &record.execution_id != execution_id)
        .unwrap_or(false);

    if integrity_binding_mismatch || containment_binding_mismatch {
        return ObservationContractDisposition::NotSatisfied {
            blockers: vec![ObservationContractBlocker::ExecutionBindingMismatch],
        };
    }

    let mut blockers = Vec::new();

    match integrity {
        None => {
            blockers.push(ObservationContractBlocker::MissingIntegrityObservation);
        }
        Some(record) => {
            if let Some(blocker) = classify_integrity_observation(profile, &record.report) {
                blockers.push(blocker);
            }
        }
    }

    match containment {
        None => {
            blockers.push(ObservationContractBlocker::MissingContainmentObservation);
        }
        Some(record) => {
            if record.coverage == ContainmentCoverageStatus::Incomplete {
                blockers.push(ObservationContractBlocker::ContainmentCoverageIncomplete);
            }

            if !record.telemetry.breaches.is_empty() {
                blockers.push(ObservationContractBlocker::ContainmentBreached);
            }
        }
    }

    if blockers.is_empty() {
        ObservationContractDisposition::Satisfied
    } else {
        ObservationContractDisposition::NotSatisfied { blockers }
    }
}
