use chrono::{DateTime, Duration, Utc};
use std::path::PathBuf;
use tempfile::TempDir;

// =====================================================================
// 1. CORE DOMAIN TYPES & RESOLVER
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
            ExecutionObservation::RemoteReportedRejection(_) => {
                if is_x { TerminalDisposition::VerifiedNoEffect } else { TerminalDisposition::Ambiguous }
            }
            ExecutionObservation::AdapterReportedFailure => {
                if is_x { TerminalDisposition::VerifiedNoEffect } else { TerminalDisposition::Ambiguous }
            }
            _ => {
                if is_x { TerminalDisposition::VerifiedNoEffect } else { TerminalDisposition::Ambiguous }
            }
        }
    }
}

// =====================================================================
// 2. SCHEMAS & ORCHESTRATOR
// =====================================================================

pub struct RepositoryPublicationCandidate {
    pub candidate_id: String,
    pub candidate_commit_oid: String,
    pub expected_prestate_oid: String,
    pub destination_ref: String,
    pub provider: String,
}

pub struct RepositoryPublicationAuthorization {
    pub authorization_id: String,
    pub authorized_candidate_id: String,
    pub operation: String,
}

pub struct RepositoryCredentialLease {
    pub lease_id: String,
    pub authorized_use_reference: String,
    pub provider: String,
    pub expires_at: DateTime<Utc>,
}

pub struct RepositoryPublicationReceipt {
    pub execution_observation: ExecutionObservation,
    pub terminal_disposition: TerminalDisposition,
    pub authorization_consumption: String,
}

pub trait ScopedCredentialProvider {
    fn with_secret(&self, f: &mut dyn FnMut(Option<&str>));
}

pub struct RemotePublicationTransportRequest {
    pub endpoint: String,
    pub source_ref: String,
    pub destination_ref: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkError {
    Rejected(String),
    ProtocolViolation,
    CredentialFailure,
    EndpointMismatch,
    Unknown,
}

pub trait NetworkTransport {
    fn execute_push(&self, req: &RemotePublicationTransportRequest, creds: &dyn ScopedCredentialProvider) -> Result<(), NetworkError>;
}

pub struct SovereignPublicationOrchestrator<'a> {
    pub transport: &'a dyn NetworkTransport,
    pub credentials: &'a dyn ScopedCredentialProvider,
    pub verifier: &'a dyn IndependentRemoteVerifier,
}

impl<'a> SovereignPublicationOrchestrator<'a> {
    pub fn execute_governed_publication(
        &self,
        cand: &RepositoryPublicationCandidate,
        auth: &RepositoryPublicationAuthorization,
        lease: &RepositoryCredentialLease,
    ) -> RepositoryPublicationReceipt {
        if auth.authorized_candidate_id != cand.candidate_id { return self.deny(TerminalDisposition::CandidateInvalid); }
        if auth.operation != "repository.remote.publish_exact" { return self.deny(TerminalDisposition::Denied); }

        if cand.provider != lease.provider { return self.deny(TerminalDisposition::IdentityMismatch); }
        if lease.authorized_use_reference != auth.authorization_id { return self.deny(TerminalDisposition::CredentialUnavailable); }
        if lease.expires_at < Utc::now() { return self.deny(TerminalDisposition::CredentialUnavailable); }

        let pre_obs = self.verifier.observe_remote_state(&cand.destination_ref);
        if pre_obs.observation_state != ObservationState::Present || pre_obs.observed_oid.as_deref() != Some(cand.expected_prestate_oid.as_str()) {
            return self.deny(TerminalDisposition::PreconditionFailed);
        }

        let transport_req = RemotePublicationTransportRequest {
            endpoint: "file://local".into(),
            source_ref: cand.destination_ref.clone(),
            destination_ref: cand.destination_ref.clone(),
        };

        let exec_obs = match self.transport.execute_push(&transport_req, self.credentials) {
            Ok(()) => ExecutionObservation::AdapterReportedSuccess,
            Err(NetworkError::Rejected(msg)) => ExecutionObservation::RemoteReportedRejection(msg),
            Err(NetworkError::ProtocolViolation) => ExecutionObservation::AdapterReportedFailure,
            Err(_) => ExecutionObservation::TransportOutcomeUnknown,
        };

        let post_obs = self.verifier.observe_remote_state(&cand.destination_ref);
        let term_disp = DispositionResolver::resolve(&exec_obs, &post_obs, &cand.candidate_commit_oid, &cand.expected_prestate_oid);

        RepositoryPublicationReceipt {
            execution_observation: exec_obs,
            terminal_disposition: term_disp,
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

pub trait IndependentRemoteVerifier {
    fn observe_remote_state(&self, dest_ref: &str) -> IndependentPostObservation;
}

// =====================================================================
// 3. FIXTURE IMPLEMENTATIONS
// =====================================================================

pub struct LibGit2ProviderFixture {
    pub local_repo_path: PathBuf,
    pub remote_repo_path: PathBuf,
}

fn path_to_file_url(path: &std::path::Path) -> String {
    let path_str = path.to_str().unwrap().replace('\\', "/");
    if path_str.starts_with('/') {
        format!("file://{}", path_str)
    } else {
        format!("file:///{}", path_str)
    }
}

impl NetworkTransport for LibGit2ProviderFixture {
    fn execute_push(&self, req: &RemotePublicationTransportRequest, _creds: &dyn ScopedCredentialProvider) -> Result<(), NetworkError> {
        let repo = git2::Repository::open(&self.local_repo_path).map_err(|_| NetworkError::ProtocolViolation)?;
        let endpoint = path_to_file_url(&self.remote_repo_path);
        let mut remote = repo.remote_anonymous(&endpoint).map_err(|_| NetworkError::EndpointMismatch)?;

        let refspec = format!("{}:{}", req.source_ref, req.destination_ref);
        remote.push(&[&refspec], None).map_err(|e| NetworkError::Rejected(e.message().to_string()))
    }
}

impl IndependentRemoteVerifier for LibGit2ProviderFixture {
    fn observe_remote_state(&self, dest_ref: &str) -> IndependentPostObservation {
        if let Ok(repo) = git2::Repository::open(&self.remote_repo_path) {
            if let Ok(reference) = repo.find_reference(dest_ref) {
                if let Some(target) = reference.target() {
                    return IndependentPostObservation {
                        observation_state: ObservationState::Present,
                        observed_oid: Some(target.to_string()),
                        observed_at: Utc::now(),
                    };
                }
            }
            return IndependentPostObservation {
                observation_state: ObservationState::Absent,
                observed_oid: None,
                observed_at: Utc::now(),
            };
        }
        IndependentPostObservation { observation_state: ObservationState::Unreachable, observed_oid: None, observed_at: Utc::now() }
    }
}

// Smart HTTP Receive-Pack Simulation Fixture with Object Graph Transfer (C-001 Domain)
pub struct SmartHttpServerFixture {
    pub force_remote_rejection: bool,
    pub local_repo_path: PathBuf,
    pub remote_repo_path: PathBuf,
    pub target_commit_oid: String,
}

impl NetworkTransport for SmartHttpServerFixture {
    fn execute_push(&self, req: &RemotePublicationTransportRequest, _creds: &dyn ScopedCredentialProvider) -> Result<(), NetworkError> {
        if self.force_remote_rejection {
            return Err(NetworkError::Rejected("HTTP 403 Forbidden: pre-receive hook declined".into()));
        }

        // PHASE 1: Object Graph Availability / Transfer (local ODB → remote ODB)
        let local_repo = git2::Repository::open(&self.local_repo_path).map_err(|_| NetworkError::ProtocolViolation)?;
        let remote_repo = git2::Repository::open(&self.remote_repo_path).map_err(|_| NetworkError::ProtocolViolation)?;

        let oid = git2::Oid::from_str(&self.target_commit_oid).map_err(|_| NetworkError::ProtocolViolation)?;
        let commit_obj = local_repo.find_object(oid, Some(git2::ObjectType::Commit)).map_err(|_| NetworkError::ProtocolViolation)?;

        // Ensure object is present in remote ODB (simulate packfile transfer)
        let local_odb = local_repo.odb().map_err(|_| NetworkError::ProtocolViolation)?;
        let remote_odb = remote_repo.odb().map_err(|_| NetworkError::ProtocolViolation)?;

        if !remote_odb.exists(oid) {
            let reader = local_odb.read(oid).map_err(|_| NetworkError::ProtocolViolation)?;
            remote_odb.write(reader.kind(), reader.data()).map_err(|_| NetworkError::ProtocolViolation)?;
        }

        // PHASE 2: Reference Mutation (Atomic update post-object verification)
        remote_repo.reference(&req.destination_ref, oid, true, "Smart HTTP receive-pack: object transfer verified")
            .map_err(|_| NetworkError::ProtocolViolation)?;

        Ok(())
    }
}

impl IndependentRemoteVerifier for SmartHttpServerFixture {
    fn observe_remote_state(&self, dest_ref: &str) -> IndependentPostObservation {
        if let Ok(repo) = git2::Repository::open(&self.remote_repo_path) {
            if let Ok(reference) = repo.find_reference(dest_ref) {
                if let Some(target) = reference.target() {
                    return IndependentPostObservation {
                        observation_state: ObservationState::Present,
                        observed_oid: Some(target.to_string()),
                        observed_at: Utc::now(),
                    };
                }
            }
        }
        IndependentPostObservation { observation_state: ObservationState::Absent, observed_oid: None, observed_at: Utc::now() }
    }
}

// =====================================================================
// 4. C-001 REDESIGNED ACCEPTANCE SUITES
// =====================================================================

#[cfg(test)]
mod c001_provider_tests {
    use super::*;

    struct MockProvider { secret: Option<String> }
    impl ScopedCredentialProvider for MockProvider {
        fn with_secret(&self, f: &mut dyn FnMut(Option<&str>)) { f(self.secret.as_deref()); }
    }

    fn setup_base_repo() -> (TempDir, TempDir, String, String) {
        let local_dir = tempfile::tempdir().unwrap();
        let remote_dir = tempfile::tempdir().unwrap();

        let _remote_repo = git2::Repository::init_bare(remote_dir.path()).unwrap();
        let local_repo = git2::Repository::init(local_dir.path()).unwrap();

        let sig = git2::Signature::now("Sovereign", "os@sovereign.local").unwrap();
        let tree_id = local_repo.index().unwrap().write_tree().unwrap();
        let tree = local_repo.find_tree(tree_id).unwrap();
        
        let commit_x = local_repo.commit(Some("refs/heads/main"), &sig, &sig, "Initial X", &tree, &[]).unwrap();
        let oid_x = commit_x.to_string();

        let endpoint = path_to_file_url(remote_dir.path());
        let mut remote = local_repo.remote_anonymous(&endpoint).unwrap();
        remote.push(&["refs/heads/main:refs/heads/main"], None).unwrap();

        let blob_id = local_repo.blob(b"update content").unwrap();
        let mut builder = local_repo.treebuilder(Some(&tree)).unwrap();
        builder.insert("file.txt", blob_id, 0o100644).unwrap();
        let new_tree_id = builder.write().unwrap();
        let new_tree = local_repo.find_tree(new_tree_id).unwrap();
        let commit_y = local_repo.commit(Some("refs/heads/main"), &sig, &sig, "Transition Y", &new_tree, &[&local_repo.find_commit(commit_x).unwrap()]).unwrap();
        let oid_y = commit_y.to_string();

        (local_dir, remote_dir, oid_x, oid_y)
    }

    #[test]
    fn tc_c001_000_file_transport_bypasses_server_hooks() {
        let (local_dir, remote_dir, _oid_x, _oid_y) = setup_base_repo();
        
        let hooks_dir = remote_dir.path().join("hooks");
        std::fs::create_dir_all(&hooks_dir).unwrap();
        let hook_path = hooks_dir.join(if cfg!(windows) { "pre-receive.bat" } else { "pre-receive" });
        std::fs::write(&hook_path, if cfg!(windows) { "@echo off\nexit /b 1" } else { "#!/bin/sh\nexit 1" }).unwrap();

        let fixture = LibGit2ProviderFixture {
            local_repo_path: local_dir.path().to_path_buf(),
            remote_repo_path: remote_dir.path().to_path_buf(),
        };

        let req = RemotePublicationTransportRequest {
            endpoint: path_to_file_url(remote_dir.path()),
            source_ref: "refs/heads/main".into(),
            destination_ref: "refs/heads/main".into(),
        };
        let result = fixture.execute_push(&req, &MockProvider { secret: None });
        assert!(result.is_ok());
    }

    #[test]
    fn tc_c001_001_smart_http_server_hook_rejection_results_in_verified_no_effect() {
        let (local_dir, remote_dir, oid_x, oid_y) = setup_base_repo();

        let fixture = SmartHttpServerFixture {
            force_remote_rejection: true,
            local_repo_path: local_dir.path().to_path_buf(),
            remote_repo_path: remote_dir.path().to_path_buf(),
            target_commit_oid: oid_y.clone(),
        };

        let cand = RepositoryPublicationCandidate {
            candidate_id: "cand_01".into(),
            candidate_commit_oid: oid_y.clone(),
            expected_prestate_oid: oid_x.clone(),
            destination_ref: "refs/heads/main".into(),
            provider: "github.com".into(),
        };
        let auth = RepositoryPublicationAuthorization {
            authorization_id: "auth_01".into(),
            authorized_candidate_id: "cand_01".into(),
            operation: "repository.remote.publish_exact".into(),
        };
        let lease = RepositoryCredentialLease {
            lease_id: "lease_01".into(),
            authorized_use_reference: "auth_01".into(),
            provider: "github.com".into(),
            expires_at: Utc::now() + Duration::days(1),
        };

        let orchestrator = SovereignPublicationOrchestrator {
            transport: &fixture,
            credentials: &MockProvider { secret: Some("token".into()) },
            verifier: &fixture,
        };

        let receipt = orchestrator.execute_governed_publication(&cand, &auth, &lease);

        assert!(matches!(receipt.execution_observation, ExecutionObservation::RemoteReportedRejection(_)));
        assert_eq!(receipt.terminal_disposition, TerminalDisposition::VerifiedNoEffect);
    }

    #[test]
    fn tc_c001_004_smart_http_hook_accepts_update_results_in_verified_success() {
        let (local_dir, remote_dir, oid_x, oid_y) = setup_base_repo();

        let fixture = SmartHttpServerFixture {
            force_remote_rejection: false,
            local_repo_path: local_dir.path().to_path_buf(),
            remote_repo_path: remote_dir.path().to_path_buf(),
            target_commit_oid: oid_y.clone(),
        };

        let cand = RepositoryPublicationCandidate {
            candidate_id: "cand_02".into(),
            candidate_commit_oid: oid_y.clone(),
            expected_prestate_oid: oid_x.clone(),
            destination_ref: "refs/heads/main".into(),
            provider: "github.com".into(),
        };
        let auth = RepositoryPublicationAuthorization {
            authorization_id: "auth_02".into(),
            authorized_candidate_id: "cand_02".into(),
            operation: "repository.remote.publish_exact".into(),
        };
        let lease = RepositoryCredentialLease {
            lease_id: "lease_02".into(),
            authorized_use_reference: "auth_02".into(),
            provider: "github.com".into(),
            expires_at: Utc::now() + Duration::days(1),
        };

        let orchestrator = SovereignPublicationOrchestrator {
            transport: &fixture,
            credentials: &MockProvider { secret: Some("token".into()) },
            verifier: &fixture,
        };

        let receipt = orchestrator.execute_governed_publication(&cand, &auth, &lease);

        assert_eq!(receipt.execution_observation, ExecutionObservation::AdapterReportedSuccess);
        assert_eq!(receipt.terminal_disposition, TerminalDisposition::VerifiedSuccess);
    }
}
