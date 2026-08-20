use chrono::{DateTime, Duration, Utc};
use std::cell::Cell;

// =====================================================================
// DOMAIN MODELS & CORE BOUNDARIES
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TerminalDisposition {
    Denied, CandidateInvalid, CredentialUnavailable, IdentityMismatch,
    PreconditionFailed, VerifiedNoEffect, VerifiedSuccess, Conflict, Ambiguous, AdapterInconsistency
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionObservation {
    NotDispatched, Dispatched, AdapterReportedSuccess, AdapterReportedFailure,
    RemoteReportedRejection(String), TransportInterrupted, TransportOutcomeUnknown
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObservationState { Present, Absent, Unknown, Unreachable }

#[derive(Debug, Clone)]
pub struct IndependentPostObservation {
    pub observation_state: ObservationState,
    pub observed_oid: Option<String>,
    pub observed_at: DateTime<Utc>,
}

pub struct DispositionResolver;
impl DispositionResolver {
    pub fn resolve(exec_obs: &ExecutionObservation, post_obs: &IndependentPostObservation, expected_y: &str, expected_x: &str) -> TerminalDisposition {
        let is_y = post_obs.observation_state == ObservationState::Present && post_obs.observed_oid.as_deref() == Some(expected_y);
        let is_x = post_obs.observation_state == ObservationState::Present && post_obs.observed_oid.as_deref() == Some(expected_x);
        let is_unknown = post_obs.observation_state == ObservationState::Unknown || post_obs.observation_state == ObservationState::Unreachable;
        let is_z = !is_y && !is_x && !is_unknown;

        if is_y { return TerminalDisposition::VerifiedSuccess; }
        if is_z { return TerminalDisposition::Conflict; }

        match exec_obs {
            ExecutionObservation::AdapterReportedSuccess => {
                if is_x { TerminalDisposition::AdapterInconsistency } else { TerminalDisposition::Ambiguous }
            }
            ExecutionObservation::NotDispatched => TerminalDisposition::PreconditionFailed,
            _ => {
                if is_x { TerminalDisposition::VerifiedNoEffect } else { TerminalDisposition::Ambiguous }
            }
        }
    }
}

// =====================================================================
// SCHEMAS
// =====================================================================

#[derive(Clone)]
pub struct RepositoryPublicationCandidate {
    pub candidate_id: String,
    pub candidate_commit_oid: String,
    pub expected_prestate_oid: String,
    pub proposed_from_oid: String,
    pub proposed_to_oid: String,
    pub destination_ref: String,
    pub provider: String,
}

#[derive(Clone)]
pub struct RepositoryPublicationAuthorization {
    pub authorization_id: String,
    pub authorized_candidate_id: String,
    pub operation: String,
}

#[derive(Clone)]
pub struct RepositoryCredentialLease {
    pub lease_id: String,
    pub authorized_use_reference: String,
    pub provider: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RepositoryPublicationReceipt {
    pub execution_observation: ExecutionObservation,
    pub terminal_disposition: TerminalDisposition,
    pub authorization_consumption: String,
}

// =====================================================================
// ADAPTER & TRANSPORT
// =====================================================================

pub trait ScopedCredentialProvider {
    fn with_secret(&self, f: &mut dyn FnMut(Option<&str>));
}

pub struct RemotePublicationTransportRequest {
    pub expected_prestate_oid: String,
    pub candidate_oid: String,
}

#[derive(Debug, Clone)]
pub enum NetworkError { Timeout, ProtocolViolation, CredentialFailure, Rejected(String) }

pub trait NetworkTransport {
    fn execute_push(&self, req: &RemotePublicationTransportRequest, creds: &dyn ScopedCredentialProvider) -> Result<(), NetworkError>;
}

pub struct SovereignGitAdapter<'a> {
    pub network_client: &'a dyn NetworkTransport,
}

impl<'a> SovereignGitAdapter<'a> {
    pub fn dispatch_exact(&self, request: &RemotePublicationTransportRequest, credentials: &dyn ScopedCredentialProvider) -> ExecutionObservation {
        match self.network_client.execute_push(request, credentials) {
            Ok(_) => ExecutionObservation::AdapterReportedSuccess,
            Err(NetworkError::Timeout) => ExecutionObservation::TransportInterrupted,
            Err(NetworkError::ProtocolViolation) => ExecutionObservation::AdapterReportedFailure,
            Err(NetworkError::CredentialFailure) => ExecutionObservation::NotDispatched,
            Err(NetworkError::Rejected(msg)) => ExecutionObservation::RemoteReportedRejection(msg),
        }
    }
}

pub trait IndependentRemoteVerifier {
    fn observe_remote_state(&self, dest_ref: &str) -> IndependentPostObservation;
}

// =====================================================================
// ORCHESTRATOR
// =====================================================================

pub struct SovereignPublicationOrchestrator<'a> {
    pub adapter: &'a SovereignGitAdapter<'a>,
    pub verifier: &'a dyn IndependentRemoteVerifier,
    pub credentials: &'a dyn ScopedCredentialProvider,
}

impl<'a> SovereignPublicationOrchestrator<'a> {
    pub fn execute_governed_publication(
        &self,
        cand: &RepositoryPublicationCandidate,
        auth: &RepositoryPublicationAuthorization,
        lease: &RepositoryCredentialLease,
    ) -> RepositoryPublicationReceipt {
        // Gate 2: Integrity & Escalation Resistance
        if auth.authorized_candidate_id != cand.candidate_id { return self.deny(TerminalDisposition::CandidateInvalid); }
        if cand.candidate_commit_oid != cand.proposed_to_oid { return self.deny(TerminalDisposition::CandidateInvalid); }
        if auth.operation != "repository.remote.publish_exact" { return self.deny(TerminalDisposition::Denied); }

        // Gate 3 & 4: Identity & Credential Boundary
        if cand.provider != lease.provider { return self.deny(TerminalDisposition::IdentityMismatch); }
        if lease.authorized_use_reference != auth.authorization_id { return self.deny(TerminalDisposition::CredentialUnavailable); }
        if lease.expires_at < Utc::now() { return self.deny(TerminalDisposition::CredentialUnavailable); }

        // Gate 5: CAS Race Prevention (T008-051)
        let pre_obs = self.verifier.observe_remote_state(&cand.destination_ref);
        if pre_obs.observation_state != ObservationState::Present || pre_obs.observed_oid.as_deref() != Some(cand.expected_prestate_oid.as_str()) {
            return self.deny(TerminalDisposition::PreconditionFailed);
        }

        // Dispatch
        let transport_req = RemotePublicationTransportRequest {
            expected_prestate_oid: cand.expected_prestate_oid.clone(),
            candidate_oid: cand.candidate_commit_oid.clone(),
        };

        let exec_obs = self.adapter.dispatch_exact(&transport_req, self.credentials);

        // Verification & Disposition
        let post_obs = self.verifier.observe_remote_state(&cand.destination_ref);
        let terminal_disp = DispositionResolver::resolve(&exec_obs, &post_obs, &cand.candidate_commit_oid, &cand.expected_prestate_oid);

        RepositoryPublicationReceipt {
            execution_observation: exec_obs,
            terminal_disposition: terminal_disp,
            authorization_consumption: "CONSUMED".into(),
        }
    }

    fn deny(&self, disp: TerminalDisposition) -> RepositoryPublicationReceipt {
        RepositoryPublicationReceipt {
            execution_observation: ExecutionObservation::NotDispatched,
            terminal_disposition: disp,
            authorization_consumption: "NOT_CONSUMED".into(),
        }
    }
}

// =====================================================================
// INTEGRATION FIXTURES & TEST SUITES
// =====================================================================

#[cfg(test)]
mod e2e_tests {
    use super::*;

    struct E2EFixture {
        network_result: Result<(), NetworkError>,
        verifier_pre_oid: String,
        verifier_post_oid: String,
        secret: Option<String>,
        calls: Cell<usize>,
    }

    impl NetworkTransport for E2EFixture {
        fn execute_push(&self, _req: &RemotePublicationTransportRequest, creds: &dyn ScopedCredentialProvider) -> Result<(), NetworkError> {
            let mut captured = false;
            creds.with_secret(&mut |s| { if s.is_some() { captured = true; } });
            if !captured { return Err(NetworkError::CredentialFailure); }
            self.network_result.clone()
        }
    }

    impl IndependentRemoteVerifier for E2EFixture {
        fn observe_remote_state(&self, _dest_ref: &str) -> IndependentPostObservation {
            let count = self.calls.get();
            self.calls.set(count + 1);
            let oid = if count == 0 { &self.verifier_pre_oid } else { &self.verifier_post_oid };
            IndependentPostObservation { observation_state: ObservationState::Present, observed_oid: Some(oid.clone()), observed_at: Utc::now() }
        }
    }

    impl ScopedCredentialProvider for E2EFixture {
        fn with_secret(&self, f: &mut dyn FnMut(Option<&str>)) { f(self.secret.as_deref()); }
    }

    fn default_run(fixture: &E2EFixture, cand_override: Option<RepositoryPublicationCandidate>, auth_override: Option<RepositoryPublicationAuthorization>, lease_override: Option<RepositoryCredentialLease>) -> RepositoryPublicationReceipt {
        let cand = cand_override.unwrap_or(RepositoryPublicationCandidate {
            candidate_id: "cand_01".into(), candidate_commit_oid: "Y".into(), expected_prestate_oid: "X".into(),
            proposed_from_oid: "X".into(), proposed_to_oid: "Y".into(), destination_ref: "refs/heads/main".into(), provider: "github.com".into()
        });
        let auth = auth_override.unwrap_or(RepositoryPublicationAuthorization {
            authorization_id: "auth_01".into(), authorized_candidate_id: "cand_01".into(), operation: "repository.remote.publish_exact".into()
        });
        let lease = lease_override.unwrap_or(RepositoryCredentialLease {
            lease_id: "lease_01".into(), authorized_use_reference: "auth_01".into(), provider: "github.com".into(), expires_at: Utc::now() + Duration::days(1)
        });

        let adapter = SovereignGitAdapter { network_client: fixture };
        let orchestrator = SovereignPublicationOrchestrator { adapter: &adapter, verifier: fixture, credentials: fixture };
        orchestrator.execute_governed_publication(&cand, &auth, &lease)
    }

    // Suite 1: Happy Path
    #[test]
    fn suite_1_happy_path() {
        let fixture = E2EFixture { network_result: Ok(()), verifier_pre_oid: "X".into(), verifier_post_oid: "Y".into(), secret: Some("token".into()), calls: Cell::new(0) };
        let receipt = default_run(&fixture, None, None, None);
        assert_eq!(receipt.terminal_disposition, TerminalDisposition::VerifiedSuccess);
    }

    // Suite 2: CAS Race Suite (T008-051)
    #[test]
    fn suite_2_cas_race_prevents_dispatch() {
        // Pre-state is Z, but candidate expects X
        let fixture = E2EFixture { network_result: Ok(()), verifier_pre_oid: "Z".into(), verifier_post_oid: "Z".into(), secret: Some("token".into()), calls: Cell::new(0) };
        let receipt = default_run(&fixture, None, None, None);
        assert_eq!(receipt.execution_observation, ExecutionObservation::NotDispatched);
        assert_eq!(receipt.terminal_disposition, TerminalDisposition::PreconditionFailed);
    }

    // Suite 3: Adapter Lie Suite
    #[test]
    fn suite_3_adapter_success_lie_detected() {
        // Adapter reports success, but verifier still sees X
        let fixture = E2EFixture { network_result: Ok(()), verifier_pre_oid: "X".into(), verifier_post_oid: "X".into(), secret: Some("token".into()), calls: Cell::new(0) };
        let receipt = default_run(&fixture, None, None, None);
        assert_eq!(receipt.execution_observation, ExecutionObservation::AdapterReportedSuccess);
        assert_eq!(receipt.terminal_disposition, TerminalDisposition::AdapterInconsistency);
    }

    // Suite 4: False Negative Suite
    #[test]
    fn suite_4_false_negative_resolved_to_success() {
        // Adapter reports protocol failure, but verifier confirms Y
        let fixture = E2EFixture { network_result: Err(NetworkError::ProtocolViolation), verifier_pre_oid: "X".into(), verifier_post_oid: "Y".into(), secret: Some("token".into()), calls: Cell::new(0) };
        let receipt = default_run(&fixture, None, None, None);
        assert_eq!(receipt.execution_observation, ExecutionObservation::AdapterReportedFailure);
        assert_eq!(receipt.terminal_disposition, TerminalDisposition::VerifiedSuccess);
    }

    // Suite 5: Credential Boundary Suite
    #[test]
    fn suite_5_expired_lease_denied() {
        let fixture = E2EFixture { network_result: Ok(()), verifier_pre_oid: "X".into(), verifier_post_oid: "Y".into(), secret: Some("token".into()), calls: Cell::new(0) };
        let expired_lease = RepositoryCredentialLease {
            lease_id: "lease_01".into(), authorized_use_reference: "auth_01".into(), provider: "github.com".into(), expires_at: Utc::now() - Duration::days(1)
        };
        let receipt = default_run(&fixture, None, None, Some(expired_lease));
        assert_eq!(receipt.terminal_disposition, TerminalDisposition::CredentialUnavailable);
        assert_eq!(receipt.execution_observation, ExecutionObservation::NotDispatched);
    }

    #[test]
    fn suite_5_provider_mismatch_denied() {
        let fixture = E2EFixture { network_result: Ok(()), verifier_pre_oid: "X".into(), verifier_post_oid: "Y".into(), secret: Some("token".into()), calls: Cell::new(0) };
        let mismatch_lease = RepositoryCredentialLease {
            lease_id: "lease_01".into(), authorized_use_reference: "auth_01".into(), provider: "gitlab.com".into(), expires_at: Utc::now() + Duration::days(1)
        };
        let receipt = default_run(&fixture, None, None, Some(mismatch_lease));
        assert_eq!(receipt.terminal_disposition, TerminalDisposition::IdentityMismatch);
    }

    // Suite 6: Escalation Resistance Suite
    #[test]
    fn suite_6_escalation_force_update_denied() {
        let fixture = E2EFixture { network_result: Ok(()), verifier_pre_oid: "X".into(), verifier_post_oid: "Y".into(), secret: Some("token".into()), calls: Cell::new(0) };
        let bad_auth = RepositoryPublicationAuthorization {
            authorization_id: "auth_01".into(), authorized_candidate_id: "cand_01".into(), operation: "repository.remote.force_update".into()
        };
        let receipt = default_run(&fixture, None, Some(bad_auth), None);
        assert_eq!(receipt.terminal_disposition, TerminalDisposition::Denied);
    }
}
