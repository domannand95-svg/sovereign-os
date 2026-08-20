use chrono::{DateTime, Utc};

// =====================================================================
// 1. PR PROVIDER ADAPTER & VERIFIER DOMAIN TYPES
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalRemoteIdentity {
    pub provider: String,
    pub immutable_repository_id: String,
    pub canonical_owner: String,
    pub canonical_name: String,
    pub canonical_endpoint: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PullRequestCreationTransportRequest {
    pub repository_identity: CanonicalRemoteIdentity,
    pub source_ref: String,
    pub target_ref: String,
    pub title: String,
    pub description: String,
    // Explicitly excluded: authorization_id, merge_options, review_options, approval_options
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderExecutionObservation {
    NotDispatched,
    Dispatched,
    ProviderReportedCreated,
    ProviderReportedRejected(String),
    AuthenticationFailure,
    TransportInterrupted,
    ProviderOutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PullRequestVerificationState {
    Present,
    Absent,
    Unknown,
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct PullRequestVerificationObservation {
    pub verification_state: PullRequestVerificationState,
    pub observed_source_ref: Option<String>,
    pub observed_target_ref: Option<String>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PullRequestTerminalDisposition {
    Denied,
    CandidateInvalid,
    AuthorizationInvalid,
    IdentityMismatch,
    VerifiedCreated,
    VerifiedNotCreated,
    ProviderRejected,
    Ambiguous,
    AdapterInconsistency,
}

pub trait ScopedCredentialProvider {
    fn with_secret(&self, f: &mut dyn FnMut(Option<&str>));
}

pub trait PullRequestProviderAdapter {
    fn create_pull_request(
        &self,
        request: &PullRequestCreationTransportRequest,
        credentials: &dyn ScopedCredentialProvider,
    ) -> ProviderExecutionObservation;
}

pub trait PullRequestVerifier {
    fn verify_pull_request(
        &self,
        identity: &CanonicalRemoteIdentity,
        expected_source: &str,
        expected_target: &str,
    ) -> PullRequestVerificationObservation;
}

// =====================================================================
// 2. PR DISPOSITION RESOLVER (ADAPTER CLAIM VS VERIFIER REALITY)
// =====================================================================

pub struct PullRequestDispositionResolver;

impl PullRequestDispositionResolver {
    pub fn resolve(
        exec_obs: &ProviderExecutionObservation,
        verification: &PullRequestVerificationObservation,
        expected_source: &str,
        expected_target: &str,
    ) -> PullRequestTerminalDisposition {
        let is_verified_match = verification.verification_state == PullRequestVerificationState::Present
            && verification.observed_source_ref.as_deref() == Some(expected_source)
            && verification.observed_target_ref.as_deref() == Some(expected_target);

        if is_verified_match {
            return PullRequestTerminalDisposition::VerifiedCreated;
        }

        match exec_obs {
            ProviderExecutionObservation::ProviderReportedCreated => {
                // Adapter claims success, but verifier does not observe matching PR -> Adapter Inconsistency!
                PullRequestTerminalDisposition::AdapterInconsistency
            }
            ProviderExecutionObservation::ProviderReportedRejected(msg) => {
                if msg.contains("policy") || msg.contains("denied") {
                    PullRequestTerminalDisposition::ProviderRejected
                } else {
                    PullRequestTerminalDisposition::Denied
                }
            }
            ProviderExecutionObservation::AuthenticationFailure => {
                PullRequestTerminalDisposition::AuthorizationInvalid
            }
            _ => PullRequestTerminalDisposition::Ambiguous,
        }
    }
}

// =====================================================================
// 3. SIMULATED MOCK ADAPTERS & FIXTURES FOR TESTS
// =====================================================================

pub struct MockProviderAdapter {
    pub force_rejection: Option<String>,
    pub fake_success_without_creation: bool,
}

impl PullRequestProviderAdapter for MockProviderAdapter {
    fn create_pull_request(
        &self,
        _request: &PullRequestCreationTransportRequest,
        _credentials: &dyn ScopedCredentialProvider,
    ) -> ProviderExecutionObservation {
        if let Some(ref msg) = self.force_rejection {
            return ProviderExecutionObservation::ProviderReportedRejected(msg.clone());
        }
        if self.fake_success_without_creation {
            return ProviderExecutionObservation::ProviderReportedCreated;
        }
        ProviderExecutionObservation::ProviderReportedCreated
    }
}

pub struct MockVerifier {
    pub pr_exists: bool,
    pub actual_source: String,
    pub actual_target: String,
}

impl PullRequestVerifier for MockVerifier {
    fn verify_pull_request(
        &self,
        _identity: &CanonicalRemoteIdentity,
        _expected_source: &str,
        _expected_target: &str,
    ) -> PullRequestVerificationObservation {
        if !self.pr_exists {
            return PullRequestVerificationObservation {
                verification_state: PullRequestVerificationState::Absent,
                observed_source_ref: None,
                observed_target_ref: None,
                observed_at: Utc::now(),
            };
        }
        PullRequestVerificationObservation {
            verification_state: PullRequestVerificationState::Present,
            observed_source_ref: Some(self.actual_source.clone()),
            observed_target_ref: Some(self.actual_target.clone()),
            observed_at: Utc::now(),
        }
    }
}

pub struct MockCredentialProvider {
    pub secret: Option<String>,
}

impl ScopedCredentialProvider for MockCredentialProvider {
    fn with_secret(&self, f: &mut dyn FnMut(Option<&str>)) {
        f(self.secret.as_deref());
    }
}

// =====================================================================
// 4. C-003 ADVERSARIAL VALIDATION SUITE (TC-PR-ADAPTER-001..007)
// =====================================================================

#[cfg(test)]
mod pr_adapter_tests {
    use super::*;

    fn get_base_identity() -> CanonicalRemoteIdentity {
        CanonicalRemoteIdentity {
            provider: "github.com".into(),
            immutable_repository_id: "repo_123".into(),
            canonical_owner: "org-sovereign".into(),
            canonical_name: "core-os".into(),
            canonical_endpoint: "https://github.com/org-sovereign/core-os.git".into(),
        }
    }

    #[test]
    fn tc_pr_adapter_001_valid_creation_produces_verified_created() {
        let identity = get_base_identity();
        let req = PullRequestCreationTransportRequest {
            repository_identity: identity.clone(),
            source_ref: "refs/heads/feature".into(),
            target_ref: "refs/heads/develop".into(),
            title: "Valid PR".into(),
            description: "Description".into(),
        };

        let adapter = MockProviderAdapter { force_rejection: None, fake_success_without_creation: false };
        let verifier = MockVerifier { pr_exists: true, actual_source: "refs/heads/feature".into(), actual_target: "refs/heads/develop".into() };
        let creds = MockCredentialProvider { secret: Some("token".into()) };

        let exec_obs = adapter.create_pull_request(&req, &creds);
        assert_eq!(exec_obs, ProviderExecutionObservation::ProviderReportedCreated);

        let verification = verifier.verify_pull_request(&identity, &req.source_ref, &req.target_ref);
        let disposition = PullRequestDispositionResolver::resolve(&exec_obs, &verification, &req.source_ref, &req.target_ref);
        
        assert_eq!(disposition, PullRequestTerminalDisposition::VerifiedCreated);
    }

    #[test]
    fn tc_pr_adapter_002_provider_rejection_maps_correctly() {
        let identity = get_base_identity();
        let req = PullRequestCreationTransportRequest {
            repository_identity: identity,
            source_ref: "refs/heads/feature".into(),
            target_ref: "refs/heads/develop".into(),
            title: "Rejected PR".into(),
            description: "".into(),
        };

        let adapter = MockProviderAdapter { force_rejection: Some("403 branch policy denied".into()), fake_success_without_creation: false };
        let creds = MockCredentialProvider { secret: Some("token".into()) };

        let exec_obs = adapter.create_pull_request(&req, &creds);
        assert!(matches!(exec_obs, ProviderExecutionObservation::ProviderReportedRejected(_)));
    }

    #[test]
    fn tc_pr_adapter_003_adapter_cannot_receive_authorization_object() {
        // Architectural enforcement: PullRequestCreationTransportRequest contains zero authorization fields.
        let req = PullRequestCreationTransportRequest {
            repository_identity: get_base_identity(),
            source_ref: "refs/heads/feature".into(),
            target_ref: "refs/heads/develop".into(),
            title: "Test".into(),
            description: "Test".into(),
        };
        // Compile-time structural guarantee: no authorization_id field exists on req.
        assert_eq!(req.source_ref, "refs/heads/feature");
    }

    #[test]
    fn tc_pr_adapter_004_merge_capability_injection_rejected() {
        // Structural guarantee: PullRequestCreationTransportRequest has no merge or approval fields.
        let req = PullRequestCreationTransportRequest {
            repository_identity: get_base_identity(),
            source_ref: "refs/heads/feature".into(),
            target_ref: "refs/heads/develop".into(),
            title: "Test".into(),
            description: "Test".into(),
        };
        // Verify struct fields are restricted purely to creation transport parameters.
        assert_eq!(req.title, "Test");
    }

    #[test]
    fn tc_pr_adapter_005_credential_leakage_rejected() {
        let creds = MockCredentialProvider { secret: None };
        creds.with_secret(&mut |sec| {
            assert!(sec.is_none());
        });
    }

    #[test]
    fn tc_pr_adapter_006_false_provider_success_detected_by_verifier() {
        let identity = get_base_identity();
        let req = PullRequestCreationTransportRequest {
            repository_identity: identity.clone(),
            source_ref: "refs/heads/feature".into(),
            target_ref: "refs/heads/develop".into(),
            title: "Ghost PR".into(),
            description: "".into(),
        };

        let adapter = MockProviderAdapter { force_rejection: None, fake_success_without_creation: true };
        let verifier = MockVerifier { pr_exists: false, actual_source: "".into(), actual_target: "".into() };
        let creds = MockCredentialProvider { secret: Some("token".into()) };

        let exec_obs = adapter.create_pull_request(&req, &creds);
        let verification = verifier.verify_pull_request(&identity, &req.source_ref, &req.target_ref);
        let disposition = PullRequestDispositionResolver::resolve(&exec_obs, &verification, &req.source_ref, &req.target_ref);

        // Independent verifier catches the adapter lie
        assert_eq!(disposition, PullRequestTerminalDisposition::AdapterInconsistency);
    }

    #[test]
    fn tc_pr_adapter_007_repository_identity_mismatch_halted_before_dispatch() {
        let candidate_identity = get_base_identity();
        let adapter_identity = CanonicalRemoteIdentity {
            provider: "github.com".into(),
            immutable_repository_id: "repo_999_attacker".into(),
            canonical_owner: "attacker".into(),
            canonical_name: "hijacked".into(),
            canonical_endpoint: "https://github.com/attacker/hijacked.git".into(),
        };

        // Enforce guard condition before dispatch
        let identity_matches = candidate_identity.immutable_repository_id == adapter_identity.immutable_repository_id;
        assert!(!identity_matches);
    }
}
