//! Provenance Integration — Trial 004-C
//!
//! This module binds verified evaluation evidence to candidate records
//! without granting any execution authority or capability.
//!
//! # Invariant
//!
//! Verification Evidence ≠ Execution Authority

use crate::evaluator::EvaluatedDisposition;
use serde::{Deserialize, Serialize};

/// Provenance manifest for Trial 004-C
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

impl From<&crate::evaluator::Finding> for FindingSummary {
    fn from(f: &crate::evaluator::Finding) -> Self {
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
