use chrono::{DateTime, Utc};

// =====================================================================
// 1. CORE IDENTITY & DOMAIN MODELS
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalRemoteIdentity {
    pub provider: String,
    pub provider_repository_id: String,
    pub owner: String,
    pub name: String,
    pub endpoint: String,
}

// Placeholder schemas for the Orchestrator signature
pub struct RepositoryPublicationCandidate;
pub struct RepositoryPublicationAuthorization;
pub struct RepositoryCredentialLease;
pub struct RepositoryPublicationReceipt;

// =====================================================================
// 2. ADAPTER BOUNDARY (Transport Execution)
// =====================================================================

/// Contains exact transport parameters. STRICTLY EXCLUDES credential metadata,
/// scopes, or authorization fields to preserve Invariant 417 and 421.
pub struct RemotePublicationTransportRequest {
    pub canonical_remote: CanonicalRemoteIdentity,
    pub destination_ref: String,
    pub expected_prestate_oid: String,
    pub candidate_oid: String,
}

pub trait CredentialBroker {
    /// Adapter requests the secret using the opaque broker reference.
    /// The adapter does not know the credential's capabilities or scopes.
    fn retrieve_secret(&self, broker_reference: &str) -> Option<String>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionObservation {
    NotDispatched,
    Dispatched,
    AdapterReportedSuccess,
    AdapterReportedFailure,
    RemoteReportedRejection(String),
    TransportInterrupted,
    TransportOutcomeUnknown,
}

/// Carries execution identity for audit and replay resistance.
pub struct AdapterExecutionObservation {
    pub execution_id: String,
    pub observation: ExecutionObservation,
    pub observed_at: DateTime<Utc>,
}

pub trait RemotePublicationAdapter {
    /// Attempts the remote transition. Inherently untrusted.
    /// Does not return Result<T, E> to prevent bypassing the epistemic model.
    fn dispatch_exact(
        &self,
        request: &RemotePublicationTransportRequest,
        broker: &dyn CredentialBroker,
    ) -> AdapterExecutionObservation;
}

// =====================================================================
// 3. INDEPENDENT VERIFIER BOUNDARY (Host Reality)
// =====================================================================

/// Verifier binds to the canonical identity to prevent URL substitution attacks.
pub struct IndependentVerificationRequest {
    pub canonical_remote: CanonicalRemoteIdentity,
    pub destination_ref: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObservationState {
    Present,
    Absent,
    Unknown,
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct IndependentPostObservation {
    pub observation_state: ObservationState,
    pub observed_oid: Option<String>,
    pub observed_at: DateTime<Utc>,
}

pub trait IndependentRemoteVerifier {
    /// Queries the remote endpoint to establish ground truth independently.
    fn observe_remote_state(
        &self,
        request: &IndependentVerificationRequest,
    ) -> IndependentPostObservation;
}

// =====================================================================
// 4. ORCHESTRATOR (The Gatekeeper & Intersector)
// =====================================================================

pub trait PublicationOrchestrator {
    /// The sole component authorized to intersect boundaries.
    /// 
    /// MUST validate:
    /// Effective Authority = Candidate Scope ∩ Authorization Scope ∩ Credential Capability ∩ Active Lease
    /// 
    /// ENFORCES T008-051 (Gate 5 Pre-Dispatch CAS Check):
    /// 1. Verifier observes X.
    /// 2. If remote != expected_prestate_oid, DENY (PRECONDITION_FAILED).
    /// 3. Adapter dispatches X -> Y.
    /// 4. Verifier observes post-state.
    fn execute_governed_publication(
        &self,
        candidate: &RepositoryPublicationCandidate,
        authorization: &RepositoryPublicationAuthorization,
        lease: &RepositoryCredentialLease,
    ) -> RepositoryPublicationReceipt;
}

// =====================================================================
// 5. EPISTEMIC DISPOSITION RESOLVER
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TerminalDisposition {
    Denied,
    CandidateInvalid,
    CredentialUnavailable,
    IdentityMismatch,
    PreconditionFailed,
    VerifiedNoEffect,
    VerifiedSuccess,
    Conflict,
    Ambiguous,
    AdapterInconsistency,
}

pub struct DispositionResolver;

impl DispositionResolver {
    /// Maps the intersection of ExecutionObservation and IndependentPostObservation
    /// into a definitive TerminalDisposition.
    pub fn resolve(
        exec_obs: &ExecutionObservation,
        post_obs: &IndependentPostObservation,
        expected_y: &str,
        expected_x: &str,
    ) -> TerminalDisposition {
        let is_y = post_obs.observation_state == ObservationState::Present 
            && post_obs.observed_oid.as_deref() == Some(expected_y);
        
        let is_x = post_obs.observation_state == ObservationState::Present 
            && post_obs.observed_oid.as_deref() == Some(expected_x);
            
        let is_unknown = post_obs.observation_state == ObservationState::Unknown 
            || post_obs.observation_state == ObservationState::Unreachable;
            
        // If it's present or absent but matches neither X nor Y, it diverged.
        let is_z = !is_y && !is_x && !is_unknown;

        // INVARIANT-416: Independent verification of Y ALWAYS yields VerifiedSuccess
        if is_y {
            return TerminalDisposition::VerifiedSuccess;
        }

        // Divergence to unexpected state Z
        if is_z {
            return TerminalDisposition::Conflict;
        }

        match exec_obs {
            ExecutionObservation::AdapterReportedSuccess => {
                if is_x {
                    TerminalDisposition::AdapterInconsistency
                } else {
                    TerminalDisposition::Ambiguous
                }
            }
            _ => {
                // For failures, interruptions, rejections, or unknown dispatches:
                // If remote is verified unchanged (X), it is safely VerifiedNoEffect.
                if is_x {
                    TerminalDisposition::VerifiedNoEffect
                } else {
                    TerminalDisposition::Ambiguous
                }
            }
        }
    }
}

// =====================================================================
// UNIT TESTS (Proving the Epistemic Resolver logic)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_obs(state: ObservationState, oid: Option<&str>) -> IndependentPostObservation {
        IndependentPostObservation {
            observation_state: state,
            observed_oid: oid.map(|s| s.to_string()),
            observed_at: Utc::now(),
        }
    }

    #[test]
    fn test_epistemic_success_verified() {
        let disp = DispositionResolver::resolve(
            &ExecutionObservation::AdapterReportedSuccess,
            &create_obs(ObservationState::Present, Some("YYY")),
            "YYY", "XXX"
        );
        assert_eq!(disp, TerminalDisposition::VerifiedSuccess);
    }

    #[test]
    fn test_epistemic_adapter_inconsistency_success_lie() {
        let disp = DispositionResolver::resolve(
            &ExecutionObservation::AdapterReportedSuccess,
            &create_obs(ObservationState::Present, Some("XXX")),
            "YYY", "XXX"
        );
        assert_eq!(disp, TerminalDisposition::AdapterInconsistency);
    }

    #[test]
    fn test_epistemic_false_negative_adapter_failure_but_remote_y() {
        let disp = DispositionResolver::resolve(
            &ExecutionObservation::AdapterReportedFailure,
            &create_obs(ObservationState::Present, Some("YYY")),
            "YYY", "XXX"
        );
        assert_eq!(disp, TerminalDisposition::VerifiedSuccess);
    }

    #[test]
    fn test_epistemic_transport_interrupted_no_effect() {
        let disp = DispositionResolver::resolve(
            &ExecutionObservation::TransportInterrupted,
            &create_obs(ObservationState::Present, Some("XXX")),
            "YYY", "XXX"
        );
        assert_eq!(disp, TerminalDisposition::VerifiedNoEffect);
    }

    #[test]
    fn test_epistemic_transport_interrupted_ambiguous() {
        let disp = DispositionResolver::resolve(
            &ExecutionObservation::TransportInterrupted,
            &create_obs(ObservationState::Unknown, None),
            "YYY", "XXX"
        );
        assert_eq!(disp, TerminalDisposition::Ambiguous);
    }

    #[test]
    fn test_epistemic_conflict_diverged_to_z() {
        let disp = DispositionResolver::resolve(
            &ExecutionObservation::RemoteReportedRejection("rejected".into()),
            &create_obs(ObservationState::Present, Some("ZZZ")),
            "YYY", "XXX"
        );
        assert_eq!(disp, TerminalDisposition::Conflict);
    }
}
