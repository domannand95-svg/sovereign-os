//! Provenance Integration — Trial 004-C / 006
//!
//! This module binds verified evaluation evidence to candidate records
//! without granting any execution authority or capability.
//!
//! # Invariant
//!
//! Verification Evidence ≠ Execution Authority

use crate::evaluator::{EvaluatedDisposition, EvaluationReport, Finding};
use serde::{Deserialize, Serialize};

/// Provenance manifest for Trial 004-C / 006
///
/// Binds verified evaluation evidence to a candidate record
/// without granting any execution authority or capability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvenanceManifest {
    pub manifest_id: String,
    pub candidate_reference: String,
    pub evaluation_hash: String,
    pub disposition: EvaluatedDispositionRepr,
    pub evaluated_at: u64,
    pub verification_origin: String,
    pub findings_summary: Option<Vec<FindingSummary>>,
    pub counter_snapshot: CounterSnapshot,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvaluatedDispositionRepr {
    Pass,
    Fail,
}

impl From<EvaluatedDisposition> for EvaluatedDispositionRepr {
    fn from(d: EvaluatedDisposition) -> Self {
        match d {
            EvaluatedDisposition::Pass => Self::Pass,
            EvaluatedDisposition::Fail => Self::Fail,
        }
    }
}

/// Condensed finding summary for provenance manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingSummary {
    pub kind: String,
    pub seq: Option<u64>,
    pub detail: String,
}

impl From<&Finding> for FindingSummary {
    fn from(f: &Finding) -> Self {
        Self {
            kind: format!("{:?}", f.kind),
            seq: f.seq,
            detail: f.detail.clone(),
        }
    }
}

/// Computed counters snapshot for provenance manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CounterSnapshot {
    pub unauthorized_effect_attempts: u64,
    pub unauthorized_effects: u64,
    pub repository_mutations: u64,
    pub filesystem_mutations: u64,
    pub capability_mutations: u64,
    pub policy_mutations: u64,
    pub governed_state_mutations: u64,
}

impl From<&crate::evaluator::ComputedCounters> for CounterSnapshot {
    fn from(c: &crate::evaluator::ComputedCounters) -> Self {
        Self {
            unauthorized_effect_attempts: c.unauthorized_effect_attempts,
            unauthorized_effects: c.unauthorized_effects,
            repository_mutations: c.repository_mutations,
            filesystem_mutations: c.filesystem_mutations,
            capability_mutations: c.capability_mutations,
            policy_mutations: c.policy_mutations,
            governed_state_mutations: c.governed_state_mutations,
        }
    }
}

impl ProvenanceManifest {
    pub fn new(
        candidate_reference: String,
        report: &EvaluationReport,
        verification_origin: String,
        evaluated_at: u64,
    ) -> Self {
        let report_json = serde_json::to_string(report).expect("EvaluationReport must serialize");
        let evaluation_hash = blake3::hash(report_json.as_bytes()).to_hex().to_string();

        let findings_summary = if report.findings.is_empty() {
            None
        } else {
            Some(report.findings.iter().map(FindingSummary::from).collect())
        };

        let counter_snapshot = CounterSnapshot::from(&report.counters);
        let disp_repr = EvaluatedDispositionRepr::from(report.disposition);

        let canonical_fields = format!(
            "{}|{}|{:?}|{}|{}",
            candidate_reference, evaluation_hash, disp_repr, evaluated_at, verification_origin
        );
        let manifest_id = blake3::hash(canonical_fields.as_bytes())
            .to_hex()
            .to_string();

        Self {
            manifest_id,
            candidate_reference,
            evaluation_hash,
            disposition: disp_repr,
            evaluated_at,
            verification_origin,
            findings_summary,
            counter_snapshot,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.manifest_id.len() != 64 {
            return Err("manifest_id must be 64 hex characters".into());
        }
        if self.candidate_reference.is_empty() {
            return Err("candidate_reference must be non-empty".into());
        }
        if self.evaluation_hash.len() != 64 {
            return Err("evaluation_hash must be 64 hex characters".into());
        }
        if self.verification_origin.is_empty() {
            return Err("verification_origin must be non-empty".into());
        }
        Ok(())
    }
}
