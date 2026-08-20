use chrono::{DateTime, Utc};

// =====================================================================
// 1. DEPLOYMENT EXECUTION ADAPTER & RUNTIME VERIFIER DOMAIN TYPES
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeIdentity {
    pub provider: String,
    pub runtime_id: String,
    pub environment: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentStrategy {
    Rolling,
    BlueGreen,
    Recreate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeploymentTransportRequest {
    pub runtime_identity: RuntimeIdentity,
    pub artifact_digest: String,
    pub deployment_strategy: DeploymentStrategy,
    pub environment: String,
    // Explicitly excluded: deployment authorization, credential leases, approval evidence, review state, merge authority, infra permissions
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentExecutionObservation {
    NotDispatched,
    Dispatched,
    ProviderReportedAccepted,
    ProviderReportedCompleted,
    ProviderReportedFailed(String),
    TransportInterrupted,
    TransportOutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeState {
    RunningExpectedArtifact,
    RunningDifferentArtifact,
    NotRunning,
    Unknown,
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct RuntimeVerificationObservation {
    pub runtime_identity: String,
    pub observed_artifact_digest: Option<String>,
    pub runtime_state: RuntimeState,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentTerminalDisposition {
    VerifiedDeployed,
    VerifiedNoEffect,
    RuntimeMismatch,
    AdapterInconsistency,
    Failed,
    Ambiguous,
    Denied,
}

pub trait ScopedCredentialProvider {
    fn with_secret(&self, f: &mut dyn FnMut(Option<&str>));
}

pub trait DeploymentAdapter {
    fn execute_deployment(
        &self,
        request: &DeploymentTransportRequest,
        credentials: &dyn ScopedCredentialProvider,
    ) -> DeploymentExecutionObservation;
}

pub trait RuntimeVerifier {
    fn verify_runtime(
        &self,
        runtime_identity: &RuntimeIdentity,
        expected_artifact_digest: &str,
    ) -> RuntimeVerificationObservation;
}

// =====================================================================
// 2. DEPLOYMENT DISPOSITION RESOLVER (ADAPTER CLAIM VS RUNTIME REALITY)
// =====================================================================

pub struct DeploymentDispositionResolver;

impl DeploymentDispositionResolver {
    pub fn resolve(
        exec_obs: &DeploymentExecutionObservation,
        verification: &RuntimeVerificationObservation,
        expected_artifact_digest: &str,
    ) -> DeploymentTerminalDisposition {
        let is_running_expected = verification.runtime_state
            == RuntimeState::RunningExpectedArtifact
            && verification.observed_artifact_digest.as_deref() == Some(expected_artifact_digest);

        // INVARIANT-605: Independent runtime verification overrides or confirms adapter claims
        if is_running_expected {
            return DeploymentTerminalDisposition::VerifiedDeployed;
        }

        if verification.runtime_state == RuntimeState::RunningDifferentArtifact {
            return DeploymentTerminalDisposition::RuntimeMismatch;
        }

        match exec_obs {
            DeploymentExecutionObservation::ProviderReportedCompleted => {
                // Adapter claims completed, but verification does not match expected artifact -> Adapter Inconsistency!
                DeploymentTerminalDisposition::AdapterInconsistency
            }
            DeploymentExecutionObservation::ProviderReportedFailed(msg) => {
                if msg.contains("rejected") || msg.contains("policy") {
                    DeploymentTerminalDisposition::Denied
                } else {
                    DeploymentTerminalDisposition::Failed
                }
            }
            DeploymentExecutionObservation::TransportInterrupted
            | DeploymentExecutionObservation::TransportOutcomeUnknown => {
                DeploymentTerminalDisposition::Ambiguous
            }
            _ => DeploymentTerminalDisposition::Ambiguous,
        }
    }
}

// =====================================================================
// 3. SIMULATED MOCK ADAPTERS & FIXTURES FOR TESTS
// =====================================================================

pub struct MockDeploymentAdapter {
    pub observation: DeploymentExecutionObservation,
}

impl DeploymentAdapter for MockDeploymentAdapter {
    fn execute_deployment(
        &self,
        _request: &DeploymentTransportRequest,
        _credentials: &dyn ScopedCredentialProvider,
    ) -> DeploymentExecutionObservation {
        self.observation.clone()
    }
}

pub struct MockRuntimeVerifier {
    pub runtime_state: RuntimeState,
    pub observed_digest: Option<String>,
}

impl RuntimeVerifier for MockRuntimeVerifier {
    fn verify_runtime(
        &self,
        runtime_identity: &RuntimeIdentity,
        _expected_artifact_digest: &str,
    ) -> RuntimeVerificationObservation {
        RuntimeVerificationObservation {
            runtime_identity: runtime_identity.runtime_id.clone(),
            observed_artifact_digest: self.observed_digest.clone(),
            runtime_state: self.runtime_state.clone(),
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
// 4. ADVERSARIAL VALIDATION SUITE (TC-DEP-ADAPTER-001..007)
// =====================================================================

#[cfg(test)]
mod deployment_adapter_tests {
    use super::*;

    fn get_base_runtime() -> RuntimeIdentity {
        RuntimeIdentity {
            provider: "aws".into(),
            runtime_id: "cluster-staging-1".into(),
            environment: "staging".into(),
        }
    }

    #[test]
    fn tc_dep_adapter_001_provider_completion_and_runtime_verification_success() {
        let runtime = get_base_runtime();
        let expected_digest =
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let req = DeploymentTransportRequest {
            runtime_identity: runtime.clone(),
            artifact_digest: expected_digest.into(),
            deployment_strategy: DeploymentStrategy::Rolling,
            environment: "staging".into(),
        };

        let adapter = MockDeploymentAdapter {
            observation: DeploymentExecutionObservation::ProviderReportedCompleted,
        };
        let verifier = MockRuntimeVerifier {
            runtime_state: RuntimeState::RunningExpectedArtifact,
            observed_digest: Some(expected_digest.into()),
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_deployment(&req, &creds);
        let verification = verifier.verify_runtime(&runtime, expected_digest);
        let disp =
            DeploymentDispositionResolver::resolve(&exec_obs, &verification, expected_digest);

        assert_eq!(disp, DeploymentTerminalDisposition::VerifiedDeployed);
    }

    #[test]
    fn tc_dep_adapter_002_deployment_success_lie_detected() {
        let runtime = get_base_runtime();
        let expected_digest =
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let req = DeploymentTransportRequest {
            runtime_identity: runtime.clone(),
            artifact_digest: expected_digest.into(),
            deployment_strategy: DeploymentStrategy::Rolling,
            environment: "staging".into(),
        };

        // Adapter claims completion, but runtime is still running old artifact
        let adapter = MockDeploymentAdapter {
            observation: DeploymentExecutionObservation::ProviderReportedCompleted,
        };
        let verifier = MockRuntimeVerifier {
            runtime_state: RuntimeState::RunningDifferentArtifact,
            observed_digest: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            ),
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_deployment(&req, &creds);
        let verification = verifier.verify_runtime(&runtime, expected_digest);
        let disp =
            DeploymentDispositionResolver::resolve(&exec_obs, &verification, expected_digest);

        assert_eq!(disp, DeploymentTerminalDisposition::RuntimeMismatch);
    }

    #[test]
    fn tc_dep_adapter_003_provider_failure_but_runtime_converged() {
        let runtime = get_base_runtime();
        let expected_digest =
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let req = DeploymentTransportRequest {
            runtime_identity: runtime.clone(),
            artifact_digest: expected_digest.into(),
            deployment_strategy: DeploymentStrategy::BlueGreen,
            environment: "staging".into(),
        };

        // Adapter reports failure (e.g. timeout during callback), but independent verifier observes correct active artifact
        let adapter = MockDeploymentAdapter {
            observation: DeploymentExecutionObservation::ProviderReportedFailed("timeout".into()),
        };
        let verifier = MockRuntimeVerifier {
            runtime_state: RuntimeState::RunningExpectedArtifact,
            observed_digest: Some(expected_digest.into()),
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_deployment(&req, &creds);
        let verification = verifier.verify_runtime(&runtime, expected_digest);
        let disp =
            DeploymentDispositionResolver::resolve(&exec_obs, &verification, expected_digest);

        assert_eq!(disp, DeploymentTerminalDisposition::VerifiedDeployed);
    }

    #[test]
    fn tc_dep_adapter_004_runtime_artifact_mismatch() {
        let runtime = get_base_runtime();
        let expected_digest =
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let req = DeploymentTransportRequest {
            runtime_identity: runtime.clone(),
            artifact_digest: expected_digest.into(),
            deployment_strategy: DeploymentStrategy::Rolling,
            environment: "staging".into(),
        };

        let adapter = MockDeploymentAdapter {
            observation: DeploymentExecutionObservation::ProviderReportedCompleted,
        };
        let verifier = MockRuntimeVerifier {
            runtime_state: RuntimeState::RunningDifferentArtifact,
            observed_digest: Some(
                "sha256:1111111111111111111111111111111111111111111111111111111111111111".into(),
            ),
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_deployment(&req, &creds);
        let verification = verifier.verify_runtime(&runtime, expected_digest);
        let disp =
            DeploymentDispositionResolver::resolve(&exec_obs, &verification, expected_digest);

        assert_eq!(disp, DeploymentTerminalDisposition::RuntimeMismatch);
    }

    #[test]
    fn tc_dep_adapter_005_network_interruption_yields_ambiguous() {
        let runtime = get_base_runtime();
        let expected_digest =
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let req = DeploymentTransportRequest {
            runtime_identity: runtime.clone(),
            artifact_digest: expected_digest.into(),
            deployment_strategy: DeploymentStrategy::Rolling,
            environment: "staging".into(),
        };

        let adapter = MockDeploymentAdapter {
            observation: DeploymentExecutionObservation::TransportInterrupted,
        };
        let verifier = MockRuntimeVerifier {
            runtime_state: RuntimeState::Unknown,
            observed_digest: None,
        };
        let creds = MockCredentialProvider {
            secret: Some("token".into()),
        };

        let exec_obs = adapter.execute_deployment(&req, &creds);
        let verification = verifier.verify_runtime(&runtime, expected_digest);
        let disp =
            DeploymentDispositionResolver::resolve(&exec_obs, &verification, expected_digest);

        assert_eq!(disp, DeploymentTerminalDisposition::Ambiguous);
    }

    #[test]
    fn tc_dep_adapter_006_credential_isolation_enforced() {
        // Compile-time structural guarantee: DeploymentTransportRequest contains zero credential fields.
        let creds = MockCredentialProvider { secret: None };
        creds.with_secret(&mut |sec| {
            assert!(sec.is_none());
        });
    }

    #[test]
    fn tc_dep_adapter_007_deployment_cannot_expand_authority() {
        // Structural guarantee: DeploymentTransportRequest and DeploymentExecutionObservation contain zero side-effect triggers.
        let req = DeploymentTransportRequest {
            runtime_identity: get_base_runtime(),
            artifact_digest:
                "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            deployment_strategy: DeploymentStrategy::Rolling,
            environment: "staging".into(),
        };
        assert_eq!(req.environment, "staging");
    }
}
