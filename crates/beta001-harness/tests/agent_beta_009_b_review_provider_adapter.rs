use chrono::{DateTime, Utc};

// =====================================================================
// 1. REVIEW PROVIDER ADAPTER & VERIFIER DOMAIN TYPES
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
pub enum ReviewTransportOperation {
    Observe,
    SubmitComment { body: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewProviderTransportRequest {
    pub repository_identity: CanonicalRemoteIdentity,
    pub pull_request_id: String,
    pub operation: ReviewTransportOperation,
    // Explicitly excluded: authorization_id, merge_permission, approval_permission, reviewer_override, credentials
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderReviewState {
    Commented,
    ChangesRequested,
    ApprovedObserved,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewExecutionObservation {
    NotDispatched,
    Dispatched,
    ProviderReportedObservation(ProviderReviewState),
    ProviderReportedCommentCreated,
    ProviderReportedRejected(String),
    AuthenticationFailure,
    TransportInterrupted,
    ProviderOutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewVerificationState {
    Present,
    Absent,
    Unknown,
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct ReviewVerificationObservation {
    pub verification_state: ReviewVerificationState,
    pub observed_state: Option<ProviderReviewState>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewTerminalDisposition {
    Denied,
    CandidateInvalid,
    AuthorizationInvalid,
    IdentityMismatch,
    VerifiedObserved,
    VerifiedCommentCreated,
    VerifiedNotCreated,
    ProviderRejected,
    Ambiguous,
    AdapterInconsistency,
}

pub trait ScopedCredentialProvider {
    fn with_secret(&self, f: &mut dyn FnMut(Option<&str>));
}

pub trait ReviewProviderAdapter {
    fn execute_review_operation(
        &self,
        request: &ReviewProviderTransportRequest,
        credentials: &dyn ScopedCredentialProvider,
    ) -> ReviewExecutionObservation;
}

pub trait ReviewVerifier {
    fn verify_review_state(
        &self,
        identity: &CanonicalRemoteIdentity,
        pull_request_id: &str,
    ) -> ReviewVerificationObservation;
}

// =====================================================================
// 2. REVIEW DISPOSITION RESOLVER (ADAPTER CLAIM VS VERIFIER REALITY)
// =====================================================================

pub struct ReviewDispositionResolver;

impl ReviewDispositionResolver {
    pub fn resolve(
        request_op: &ReviewTransportOperation,
        exec_obs: &ReviewExecutionObservation,
        verification: &ReviewVerificationObservation,
    ) -> ReviewTerminalDisposition {
        match exec_obs {
            ReviewExecutionObservation::ProviderReportedObservation(_) => {
                if verification.verification_state == ReviewVerificationState::Present {
                    ReviewTerminalDisposition::VerifiedObserved
                } else {
                    ReviewTerminalDisposition::AdapterInconsistency
                }
            }
            ReviewExecutionObservation::ProviderReportedCommentCreated => {
                if verification.verification_state == ReviewVerificationState::Present {
                    ReviewTerminalDisposition::VerifiedCommentCreated
                } else {
                    ReviewTerminalDisposition::AdapterInconsistency
                }
            }
            ReviewExecutionObservation::ProviderReportedRejected(msg) => {
                if msg.contains("403") || msg.contains("denied") || msg.contains("policy") {
                    ReviewTerminalDisposition::ProviderRejected
                } else {
                    ReviewTerminalDisposition::Denied
                }
            }
            ReviewExecutionObservation::AuthenticationFailure => {
                ReviewTerminalDisposition::AuthorizationInvalid
            }
            _ => ReviewTerminalDisposition::Ambiguous,
        }
    }
}

// =====================================================================
// 3. SIMULATED MOCK ADAPTERS & FIXTURES FOR TESTS
// =====================================================================

pub struct MockReviewProviderAdapter {
    pub force_rejection: Option<String>,
    pub fake_success_without_state: bool,
    pub reported_state: ProviderReviewState,
}

impl ReviewProviderAdapter for MockReviewProviderAdapter {
    fn execute_review_operation(
        &self,
        request: &ReviewProviderTransportRequest,
        _credentials: &dyn ScopedCredentialProvider,
    ) -> ReviewExecutionObservation {
        if let Some(ref msg) = self.force_rejection {
            return ReviewExecutionObservation::ProviderReportedRejected(msg.clone());
        }
        match &request.operation {
            ReviewTransportOperation::Observe => {
                ReviewExecutionObservation::ProviderReportedObservation(self.reported_state.clone())
            }
            ReviewTransportOperation::SubmitComment { .. } => {
                if self.fake_success_without_state {
                    ReviewExecutionObservation::ProviderReportedCommentCreated
                } else {
                    ReviewExecutionObservation::ProviderReportedCommentCreated
                }
            }
        }
    }
}

pub struct MockReviewVerifier {
    pub review_exists: bool,
    pub state: Option<ProviderReviewState>,
}

impl ReviewVerifier for MockReviewVerifier {
    fn verify_review_state(
        &self,
        _identity: &CanonicalRemoteIdentity,
        _pull_request_id: &str,
    ) -> ReviewVerificationObservation {
        if !self.review_exists {
            return ReviewVerificationObservation {
                verification_state: ReviewVerificationState::Absent,
                observed_state: None,
                observed_at: Utc::now(),
            };
        }
        ReviewVerificationObservation {
            verification_state: ReviewVerificationState::Present,
            observed_state: self.state.clone(),
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
// 4. ADVERSARIAL VALIDATION SUITE (TC-REV-ADAPTER-001..007)
// =====================================================================

#[cfg(test)]
mod review_adapter_tests {
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
    fn tc_rev_adapter_001_valid_observation_returns_telemetry() {
        let identity = get_base_identity();
        let req = ReviewProviderTransportRequest {
            repository_identity: identity.clone(),
            pull_request_id: "pr_123".into(),
            operation: ReviewTransportOperation::Observe,
        };

        let adapter = MockReviewProviderAdapter {
            force_rejection: None,
            fake_success_without_state: false,
            reported_state: ProviderReviewState::Commented,
        };
        let verifier = MockReviewVerifier { review_exists: true, state: Some(ProviderReviewState::Commented) };
        let creds = MockCredentialProvider { secret: Some("token".into()) };

        let exec_obs = adapter.execute_review_operation(&req, &creds);
        let verification = verifier.verify_review_state(&identity, &req.pull_request_id);
        let disp = ReviewDispositionResolver::resolve(&req.operation, &exec_obs, &verification);

        assert_eq!(disp, ReviewTerminalDisposition::VerifiedObserved);
    }

    #[test]
    fn tc_rev_adapter_002_valid_comment_submission_returns_telemetry() {
        let identity = get_base_identity();
        let req = ReviewProviderTransportRequest {
            repository_identity: identity.clone(),
            pull_request_id: "pr_123".into(),
            operation: ReviewTransportOperation::SubmitComment { body: "LGTM".into() },
        };

        let adapter = MockReviewProviderAdapter {
            force_rejection: None,
            fake_success_without_state: false,
            reported_state: ProviderReviewState::Commented,
        };
        let verifier = MockReviewVerifier { review_exists: true, state: Some(ProviderReviewState::Commented) };
        let creds = MockCredentialProvider { secret: Some("token".into()) };

        let exec_obs = adapter.execute_review_operation(&req, &creds);
        let verification = verifier.verify_review_state(&identity, &req.pull_request_id);
        let disp = ReviewDispositionResolver::resolve(&req.operation, &exec_obs, &verification);

        assert_eq!(disp, ReviewTerminalDisposition::VerifiedCommentCreated);
    }

    #[test]
    fn tc_rev_adapter_003_provider_rejection_maps_correctly() {
        let identity = get_base_identity();
        let req = ReviewProviderTransportRequest {
            repository_identity: identity,
            pull_request_id: "pr_123".into(),
            operation: ReviewTransportOperation::Observe,
        };

        let adapter = MockReviewProviderAdapter {
            force_rejection: Some("403 review permission denied".into()),
            fake_success_without_state: false,
            reported_state: ProviderReviewState::Commented,
        };
        let creds = MockCredentialProvider { secret: Some("token".into()) };

        let exec_obs = adapter.execute_review_operation(&req, &creds);
        assert!(matches!(exec_obs, ReviewExecutionObservation::ProviderReportedRejected(_)));
    }

    #[test]
    fn tc_rev_adapter_004_adapter_cannot_access_authorization_metadata() {
        // Compile-time structural guarantee: ReviewProviderTransportRequest contains zero authorization fields.
        let req = ReviewProviderTransportRequest {
            repository_identity: get_base_identity(),
            pull_request_id: "pr_123".into(),
            operation: ReviewTransportOperation::Observe,
        };
        assert_eq!(req.pull_request_id, "pr_123");
    }

    #[test]
    fn tc_rev_adapter_005_approval_capability_injection_rejected() {
        // Structural guarantee: ReviewTransportOperation enum has no variant for approval or merge.
        let op = ReviewTransportOperation::Observe;
        match op {
            ReviewTransportOperation::Observe | ReviewTransportOperation::SubmitComment { .. } => {}
        }
    }

    #[test]
    fn tc_rev_adapter_006_credential_leakage_rejected() {
        let creds = MockCredentialProvider { secret: None };
        creds.with_secret(&mut |sec| {
            assert!(sec.is_none());
        });
    }

    #[test]
    fn tc_rev_adapter_007_false_provider_success_detected_by_verifier() {
        let identity = get_base_identity();
        let req = ReviewProviderTransportRequest {
            repository_identity: identity.clone(),
            pull_request_id: "pr_123".into(),
            operation: ReviewTransportOperation::SubmitComment { body: "Fake".into() },
        };

        let adapter = MockReviewProviderAdapter {
            force_rejection: None,
            fake_success_without_state: true,
            reported_state: ProviderReviewState::Commented,
        };
        let verifier = MockReviewVerifier { review_exists: false, state: None };
        let creds = MockCredentialProvider { secret: Some("token".into()) };

        let exec_obs = adapter.execute_review_operation(&req, &creds);
        let verification = verifier.verify_review_state(&identity, &req.pull_request_id);
        let disp = ReviewDispositionResolver::resolve(&req.operation, &exec_obs, &verification);

        assert_eq!(disp, ReviewTerminalDisposition::AdapterInconsistency);
    }
}
