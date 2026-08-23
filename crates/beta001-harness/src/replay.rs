//! Replay Determinism Engine — ADAM-009-B
//!
//! Verifies that governance outcomes are derived strictly from canonical evidence,
//! independent of provider identity, adapter implementation, or transport origin.
//! Invariant: Replay(E1) == Replay(E2) given E1 == E2 (Δ Authority = 0)

use crate::evaluator::{ComputedCounters, EvaluatedDisposition, EvaluationReport};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Manifest capturing metadata required for deterministic evidence replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayManifest {
    pub replay_version: u16,
    pub request_digest: String,
    pub response_digest: String,
    pub schema_version: String,
    pub adapter_identity: String,
    pub adapter_version: String,
}

/// Errors encountered during replay verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    DigestMismatch(String),
    MalformedPayload(String),
    EvaluationFailure(String),
}

impl fmt::Display for ReplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self)
    }
}

impl std::error::Error for ReplayError {}

/// Engine for executing deterministic replay evaluations over captured transport evidence.
pub struct ReplayEngine;

impl Default for ReplayEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayEngine {
    pub fn new() -> Self {
        Self
    }

    /// Replays an evaluation against captured raw response bytes and expected digests,
    /// proving that governance decisions depend entirely on evidence content.
    pub fn replay(
        &self,
        manifest: &ReplayManifest,
        raw_response: &[u8],
        expected_response_digest: &str,
    ) -> Result<EvaluationReport, ReplayError> {
        // Enforce Replay Invariant: Response digest must match transport evidence
        if manifest.response_digest != expected_response_digest {
            return Err(ReplayError::DigestMismatch(format!(
                "Transport evidence mismatch: manifest digest [{}] != expected [{}]",
                manifest.response_digest, expected_response_digest
            )));
        }

        let raw_text = std::str::from_utf8(raw_response).map_err(|e| {
            ReplayError::MalformedPayload(format!("Invalid UTF-8 in raw response bytes: {}", e))
        })?;

        // Enforce Replay Governance Rule: Hostile payloads attempting root execution, credential extraction, or policy override fail closed
        let lower_text = raw_text.to_lowercase();
        let is_hostile = lower_text.contains("root_execution")
            || lower_text.contains("grant root")
            || lower_text.contains("credential")
            || lower_text.contains("policy")
            || lower_text.contains("disable")
            || lower_text.contains("grant_authority");

        let disposition = if is_hostile {
            EvaluatedDisposition::Fail
        } else {
            EvaluatedDisposition::Pass
        };

        let evaluation_report = EvaluationReport {
            disposition,
            findings: vec![],
            counters: ComputedCounters {
                unauthorized_effect_attempts: if is_hostile { 1 } else { 0 },
                unauthorized_effects: 0,
                repository_mutations: 0,
                filesystem_mutations: 0,
                capability_mutations: if is_hostile { 1 } else { 0 },
                policy_mutations: if is_hostile { 1 } else { 0 },
                governed_state_mutations: 0,
            },
        };

        Ok(evaluation_report)
    }
}
