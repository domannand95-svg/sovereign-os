use chrono::{DateTime, Duration, Utc};

// =====================================================================
// 1. CANONICAL IDENTITY DOMAIN TYPES & RESOLUTION
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum IdentityResolution {
    Verified,
    IdentityMismatch,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferPolicy {
    RejectOnTransfer,
    AllowWithSignedTransition,
}

#[derive(Debug, Clone)]
pub struct CanonicalRemoteIdentity {
    pub provider: String,
    pub immutable_repository_id: String,
    pub canonical_owner: String,
    pub canonical_name: String,
    pub canonical_endpoint: String,
    pub transfer_policy: TransferPolicy,
}

#[derive(Debug, Clone)]
pub struct RemoteIdentityObservation {
    pub provider: String,
    pub immutable_repository_id: String,
    pub observed_owner: String,
    pub observed_name: String,
    pub observed_endpoint: String,
    pub encountered_redirect: bool,
}

pub struct CanonicalIdentityResolver;

impl CanonicalIdentityResolver {
    pub fn resolve_identity(
        expected: &CanonicalRemoteIdentity,
        observed: &RemoteIdentityObservation,
    ) -> IdentityResolution {
        // T008-004: Endpoint redirect following is strictly forbidden
        if observed.encountered_redirect {
            return IdentityResolution::IdentityMismatch;
        }

        // Provider boundary check
        if expected.provider != observed.provider {
            return IdentityResolution::IdentityMismatch;
        }

        // Primary Immutable Check: Immutable Repository ID (handles renames securely)
        if expected.immutable_repository_id != observed.immutable_repository_id {
            return IdentityResolution::IdentityMismatch;
        }

        // Verified Endpoint Match (prevents pushurl / redirect substitution)
        if expected.canonical_endpoint != observed.observed_endpoint {
            return IdentityResolution::IdentityMismatch;
        }

        // Ownership / Transfer Check
        if expected.canonical_owner != observed.observed_owner {
            match expected.transfer_policy {
                TransferPolicy::RejectOnTransfer => return IdentityResolution::IdentityMismatch,
                TransferPolicy::AllowWithSignedTransition => {
                    // Future hook for signed transfer manifests; default reject in absence of proof
                    return IdentityResolution::IdentityMismatch;
                }
            }
        }

        IdentityResolution::Verified
    }
}

// =====================================================================
// 2. ORCHESTRATOR IDENTITY GATE INTEGRATION MOCK
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TerminalDisposition {
    Denied, CandidateInvalid, CredentialUnavailable, IdentityMismatch,
    PreconditionFailed, VerifiedNoEffect, VerifiedSuccess, Conflict, Ambiguous
}

pub struct IdentityBoundPublicationOrchestrator<'a> {
    pub canonical_identity: &'a CanonicalRemoteIdentity,
}

impl<'a> IdentityBoundPublicationOrchestrator<'a> {
    pub fn verify_identity_gate(
        &self,
        observation: &RemoteIdentityObservation,
    ) -> Result<(), TerminalDisposition> {
        match CanonicalIdentityResolver::resolve_identity(self.canonical_identity, observation) {
            IdentityResolution::Verified => Ok(()),
            IdentityResolution::IdentityMismatch => Err(TerminalDisposition::IdentityMismatch),
        }
    }
}

// =====================================================================
// 3. C-002 CANONICAL IDENTITY TEST SUITES (THREAT MATRIX T008)
// =====================================================================

#[cfg(test)]
mod c002_identity_tests {
    use super::*;

    fn get_base_canonical_identity() -> CanonicalRemoteIdentity {
        CanonicalRemoteIdentity {
            provider: "github.com".into(),
            immutable_repository_id: "repo_id_12345".into(),
            canonical_owner: "org-sovereign".into(),
            canonical_name: "core-os".into(),
            canonical_endpoint: "https://github.com/org-sovereign/core-os.git".into(),
            transfer_policy: TransferPolicy::RejectOnTransfer,
        }
    }

    #[test]
    fn tc_c002_001_origin_alias_substitution_yields_identity_mismatch() {
        let expected = get_base_canonical_identity();
        
        // Attack: Local alias 'origin' points to attacker repo (T008-001, T008-003)
        let observed = RemoteIdentityObservation {
            provider: "github.com".into(),
            immutable_repository_id: "repo_id_attacker_99999".into(),
            observed_owner: "attacker-org".into(),
            observed_name: "malicious-repo".into(),
            observed_endpoint: "https://github.com/attacker-org/malicious-repo.git".into(),
            encountered_redirect: false,
        };

        let resolution = CanonicalIdentityResolver::resolve_identity(&expected, &observed);
        assert_eq!(resolution, IdentityResolution::IdentityMismatch);
    }

    #[test]
    fn tc_c002_002_push_url_override_yields_identity_mismatch() {
        let expected = get_base_canonical_identity();

        // Attack: remote.origin.pushurl override (T008-003)
        let observed = RemoteIdentityObservation {
            provider: "github.com".into(),
            immutable_repository_id: "repo_id_12345".into(),
            observed_owner: "org-sovereign".into(),
            observed_name: "core-os".into(),
            observed_endpoint: "https://github.com/org-sovereign/hijacked-pushurl.git".into(),
            encountered_redirect: false,
        };

        let resolution = CanonicalIdentityResolver::resolve_identity(&expected, &observed);
        assert_eq!(resolution, IdentityResolution::IdentityMismatch);
    }

    #[test]
    fn tc_c002_003_repository_rename_retains_verification() {
        let expected = get_base_canonical_identity();

        // Benign metadata change: Repository renamed from core-os to sovereign-core (T008-048)
        let observed = RemoteIdentityObservation {
            provider: "github.com".into(),
            immutable_repository_id: "repo_id_12345".into(), // Stable ID
            observed_owner: "org-sovereign".into(),
            observed_name: "sovereign-core".into(),
            observed_endpoint: "https://github.com/org-sovereign/core-os.git".into(),
            encountered_redirect: false,
        };

        let resolution = CanonicalIdentityResolver::resolve_identity(&expected, &observed);
        assert_eq!(resolution, IdentityResolution::Verified);
    }

    #[test]
    fn tc_c002_004_repository_transfer_triggers_mismatch() {
        let expected = get_base_canonical_identity();

        // Attack/Unauthorized Transfer: Ownership changed to third-party without manifest (T008-049)
        let observed = RemoteIdentityObservation {
            provider: "github.com".into(),
            immutable_repository_id: "repo_id_12345".into(),
            observed_owner: "external-party".into(),
            observed_name: "core-os".into(),
            observed_endpoint: "https://github.com/external-party/core-os.git".into(),
            encountered_redirect: false,
        };

        let resolution = CanonicalIdentityResolver::resolve_identity(&expected, &observed);
        assert_eq!(resolution, IdentityResolution::IdentityMismatch);
    }

    #[test]
    fn tc_c002_005_endpoint_redirect_triggers_mismatch() {
        let expected = get_base_canonical_identity();

        // Attack: HTTP 302 redirect redirection (T008-004)
        let observed = RemoteIdentityObservation {
            provider: "github.com".into(),
            immutable_repository_id: "repo_id_12345".into(),
            observed_owner: "org-sovereign".into(),
            observed_name: "core-os".into(),
            observed_endpoint: "https://github.com/org-sovereign/core-os.git".into(),
            encountered_redirect: true,
        };

        let resolution = CanonicalIdentityResolver::resolve_identity(&expected, &observed);
        assert_eq!(resolution, IdentityResolution::IdentityMismatch);
    }

    #[test]
    fn tc_c002_006_environment_and_config_injection_intercepted() {
        let expected = get_base_canonical_identity();
        let orchestrator = IdentityBoundPublicationOrchestrator { canonical_identity: &expected };

        // Attack: git config url.*.insteadOf manipulation injecting foreign endpoint (T008-040, T008-041)
        let observed = RemoteIdentityObservation {
            provider: "github.com".into(),
            immutable_repository_id: "repo_id_injected".into(),
            observed_owner: "attacker".into(),
            observed_name: "injected".into(),
            observed_endpoint: "https://evil-proxy.local/repo.git".into(),
            encountered_redirect: false,
        };

        let result = orchestrator.verify_identity_gate(&observed);
        assert_eq!(result, Err(TerminalDisposition::IdentityMismatch));
    }
}
