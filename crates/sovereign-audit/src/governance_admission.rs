/// Governance outcome produced after policy evaluation.
/// This is not an execution capability.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AdmissionOutcome {
    Permit,
    Deny,
    Quarantine,
    Escalate,
}

/// Immutable governance decision artifact.
/// This records a decision but does not itself execute authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionDecision {
    pub decision_reference: String,
    pub intent_reference: String,
    pub evaluation_reference: String,
    pub governance_context_reference: String,
    pub outcome: AdmissionOutcome,
    pub authorized_scope: String,
    pub reason_references: Vec<String>,
}