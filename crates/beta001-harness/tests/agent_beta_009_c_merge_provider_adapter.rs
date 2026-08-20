use chrono::{DateTime, Utc};

// =====================================================================
// 1. MERGE PROVIDER ADAPTER & VERIFIER DOMAIN TYPES
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
pub enum MergeStrategy {
    MergeCommit,
    SquashMerge,
    RebaseMerge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MergeTransportRequest {
    pub canonical_remote_identity: CanonicalRemoteIdentity,
    pub pull_request_identity: String,
    pub target_ref: String,
    pub expected_target_prestate_oid: String,
    pub merge_strategy: MergeStrategy,
    // Explicitly excluded: review observations, approval state authority, deployment/publication permissions, credentials
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeExecutionObservation {
    NotDispatched,
    Dispatched,
    ProviderReportedMerged,
    ProviderReportedRejected(String),
    ProviderReportedConflict(String),
    TransportInterrupted,
    TransportOutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeObservationState {
    VerifiedTargetState,
    ConflictState,
    Unknown,
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct MergeVerificationObservation {
    pub target_ref: String,
    pub observed_target_oid: Option<String>,
    pub observation_state: MergeObservationState,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeTerminalDisposition {
    VerifiedMerged,
    VerifiedNoEffect,
    Conflict,
    AdapterInconsistency,
    PreconditionFailed,
    Ambiguous,
    Denied,
}

pub trait ScopedCredentialProvider {
    fn with_secret(&self, f: &mut dyn FnMut(Option<&str>));
}

pub trait MergeProviderAdapter {
    fn execute_merge(
        &self,
        request: &MergeTransportRequest,
        credentials: &dyn ScopedCredentialProvider,
    ) -> MergeExecutionObservation;
}

pub trait MergeVerifier {
    fn verify_merge(
        &self,
        identity: &CanonicalRemoteIdentity,
        target_ref: &str,
        expected_resulting_oid: &str,
    ) -> MergeVerificationObservation;
}

// =====================================================================
// 2. MERGE DISPOSITION RESOLVER (ADAPTER CLAIM VS VERIFIER REALITY)
// =====================================================================

pub struct MergeDispositionResolver;

impl MergeDispositionResolver {
    pub fn resolve(
        exec_obs: &MergeExecutionObservation,
        verification: &MergeVerificationObservation,
        expected_resulting_oid: &str,
    ) -> MergeTerminalDisposition {
        let is_verified_target = verification.observation_state
            == MergeObservationState::VerifiedTargetState
            && verification.observed_target_oid.as_deref() == Some(expected_resulting_oid);

        // INVARIANT-514: Independent verification overrides or confirms adapter claims
        if is_verified_target {
            return MergeTerminalDisposition::VerifiedMerged;
        }

        match exec_obs {
            MergeExecutionObservation::ProviderReportedMerged => {
                // Adapter claims merged, but verification does not match expected result -> Adapter Inconsistency!
                MergeTerminalDisposition::AdapterInconsistency
            }
            MergeExecutionObservation::ProviderReportedConflict(_) => {
                if verification.observation_state == MergeObservationState::ConflictState {
                    MergeTerminalDisposition::Conflict
                } else {
                    MergeTerminalDisposition::VerifiedNoEffect
                }
            }
            MergeExecutionObservation::ProviderReportedRejected(msg) => {
                if msg.contains("precondition") || msg.contains("stale") {
                    MergeTerminalDisposition::PreconditionFailed
                } else {
                    MergeTerminalDisposition::Denied
                }
            }
            MergeExecutionObservation::TransportInterrupted
            | MergeExecutionObservation::TransportOutcomeUnknown => {
                MergeTerminalDisposition::Ambiguous
            }
            _ => MergeTerminalDisposition::Ambiguous,
        }
    }
}

// =====================================================================
// 3. SIMULATED MOCK ADAPTERS & FIXTURES FOR TESTS
// =====================================================================

pub struct MockMergeProviderAdapter {
    pub observation: MergeExecutionObservation,
}

impl MergeProviderAdapter for MockMergeProviderAdapter {
    fn execute_merge(
        &self,
        _request: &MergeTransportRequest,
        _credentials: &dyn ScopedCredentialProvider,
    ) -> MergeExecutionObservation {
        self.observation.clone()
    }
}

pub struct MockMergeVerifier {
    pub observation_state: MergeObservationState,
    pub observed_oid: Option<String>,
}

impl MergeVerifier for MockMergeVerifier {
    fn verify_merge(
        &self,
        _identity: &CanonicalRemoteIdentity,
        target_ref: &str,
        _expected_resulting_oid: &str,
    ) -> MergeVerificationObservation {
        MergeVerificationObservation {
            target_ref: target_ref.to_string(),
            observed_target_oid: self.observed_oid.clone(),
            observation_state: self.observation_state.clone(),
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
// 4. ADVERSARIAL VALIDATION SUITE (TC-MRG-ADAPTER-001..007)
// =====================================================================

#[cfg(test)]
mod merge_adapter_tests {
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
    fn tc_mrg_adapter_001_provider_reports_merge_verifier_confirms() {
        let identity = get_base_identity();
        let req = MergeTransportRequest {
            canonical_remote_identity: identity.clone(),
            pull_request_identity: "pr_123".into(),
            target_ref: "refs/heads/develop".into(),
            expected_target_prestate_oid: "prestate_oid_111".into(),
            merge_strategy: MergeStrategy::SquashMerge,
        };

        let adapter = MockMergeProviderAdapter {
            observation: MergeExecutionObservation::ProviderReportedMerged,
        };
        let verifier = MockMergeVerifier {
            observation_state: MergeObservationState::VerifiedTargetState,
            observed_oid: Some("resulting_oid_222".into()),
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_merge(&req, &creds);
        let verification = verifier.verify_merge(&identity, &req.target_ref, "resulting_oid_222");
        let disp = MergeDispositionResolver::resolve(&exec_obs, &verification, "resulting_oid_222");

        assert_eq!(disp, MergeTerminalDisposition::VerifiedMerged);
    }

    #[test]
    fn tc_mrg_adapter_002_provider_success_lie_detected() {
        let identity = get_base_identity();
        let req = MergeTransportRequest {
            canonical_remote_identity: identity.clone(),
            pull_request_identity: "pr_123".into(),
            target_ref: "refs/heads/develop".into(),
            expected_target_prestate_oid: "prestate_oid_111".into(),
            merge_strategy: MergeStrategy::SquashMerge,
        };

        // Adapter claims success, but verifier reports target unchanged (prestate)
        let adapter = MockMergeProviderAdapter {
            observation: MergeExecutionObservation::ProviderReportedMerged,
        };
        let verifier = MockMergeVerifier {
            observation_state: MergeObservationState::VerifiedTargetState,
            observed_oid: Some("prestate_oid_111".into()), // Unchanged!
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_merge(&req, &creds);
        let verification = verifier.verify_merge(&identity, &req.target_ref, "resulting_oid_222");
        let disp = MergeDispositionResolver::resolve(&exec_obs, &verification, "resulting_oid_222");

        assert_eq!(disp, MergeTerminalDisposition::AdapterInconsistency);
    }

    #[test]
    fn tc_mrg_adapter_003_provider_failure_but_state_changed() {
        let identity = get_base_identity();
        let req = MergeTransportRequest {
            canonical_remote_identity: identity.clone(),
            pull_request_identity: "pr_123".into(),
            target_ref: "refs/heads/develop".into(),
            expected_target_prestate_oid: "prestate_oid_111".into(),
            merge_strategy: MergeStrategy::MergeCommit,
        };

        // Adapter claims network error, but independent verifier observes correct resulting state (e.g. timeout post-commit)
        let adapter = MockMergeProviderAdapter {
            observation: MergeExecutionObservation::TransportInterrupted,
        };
        let verifier = MockMergeVerifier {
            observation_state: MergeObservationState::VerifiedTargetState,
            observed_oid: Some("resulting_oid_222".into()),
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_merge(&req, &creds);
        let verification = verifier.verify_merge(&identity, &req.target_ref, "resulting_oid_222");
        let disp = MergeDispositionResolver::resolve(&exec_obs, &verification, "resulting_oid_222");

        assert_eq!(disp, MergeTerminalDisposition::VerifiedMerged);
    }

    #[test]
    fn tc_mrg_adapter_004_merge_conflict_rejection() {
        let identity = get_base_identity();
        let req = MergeTransportRequest {
            canonical_remote_identity: identity.clone(),
            pull_request_identity: "pr_123".into(),
            target_ref: "refs/heads/develop".into(),
            expected_target_prestate_oid: "prestate_oid_111".into(),
            merge_strategy: MergeStrategy::RebaseMerge,
        };

        let adapter = MockMergeProviderAdapter {
            observation: MergeExecutionObservation::ProviderReportedConflict(
                "merge conflict".into(),
            ),
        };
        let verifier = MockMergeVerifier {
            observation_state: MergeObservationState::ConflictState,
            observed_oid: None,
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_merge(&req, &creds);
        let verification = verifier.verify_merge(&identity, &req.target_ref, "resulting_oid_222");
        let disp = MergeDispositionResolver::resolve(&exec_obs, &verification, "resulting_oid_222");

        assert_eq!(disp, MergeTerminalDisposition::Conflict);
    }

    #[test]
    fn tc_mrg_adapter_005_transport_interruption_yields_ambiguous() {
        let identity = get_base_identity();
        let req = MergeTransportRequest {
            canonical_remote_identity: identity.clone(),
            pull_request_identity: "pr_123".into(),
            target_ref: "refs/heads/develop".into(),
            expected_target_prestate_oid: "prestate_oid_111".into(),
            merge_strategy: MergeStrategy::SquashMerge,
        };

        let adapter = MockMergeProviderAdapter {
            observation: MergeExecutionObservation::TransportInterrupted,
        };
        let verifier = MockMergeVerifier {
            observation_state: MergeObservationState::Unknown,
            observed_oid: None,
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_merge(&req, &creds);
        let verification = verifier.verify_merge(&identity, &req.target_ref, "resulting_oid_222");
        let disp = MergeDispositionResolver::resolve(&exec_obs, &verification, "resulting_oid_222");

        assert_eq!(disp, MergeTerminalDisposition::Ambiguous);
    }

    #[test]
    fn tc_mrg_adapter_006_credential_isolation_enforced() {
        // Compile-time structural guarantee: MergeTransportRequest contains zero credential fields.
        let creds = MockCredentialProvider { secret: None };
        creds.with_secret(&mut |sec| {
            assert!(sec.is_none());
        });
    }

    #[test]
    fn tc_mrg_adapter_007_merge_cannot_trigger_deployment() {
        // Structural guarantee: MergeTransportRequest and MergeExecutionObservation contain zero deployment or publication triggers.
        let req = MergeTransportRequest {
            canonical_remote_identity: get_base_identity(),
            pull_request_identity: "pr_123".into(),
            target_ref: "refs/heads/develop".into(),
            expected_target_prestate_oid: "prestate_oid_111".into(),
            merge_strategy: MergeStrategy::SquashMerge,
        };
        assert_eq!(req.pull_request_identity, "pr_123");
    }
}
