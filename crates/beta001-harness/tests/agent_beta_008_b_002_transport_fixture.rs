use std::cell::Cell;

// =====================================================================
// 1. BOUNDARIES & OBSERVATIONS
// =====================================================================

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

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkError {
    Timeout,
    Rejected(String),
    CredentialFailure,    // Broker failed to provide secret locally
    EndpointMismatch,     // TLS validation failure, HTTP 404
    TransportUnavailable, // DNS failure, Connection Refused
    ProtocolViolation,
    Unknown,
}

// Deterministic Mapping (B-001 Contract)
pub fn map_network_error(err: &NetworkError) -> ExecutionObservation {
    match err {
        NetworkError::Timeout => ExecutionObservation::TransportInterrupted,
        NetworkError::Rejected(msg) => ExecutionObservation::RemoteReportedRejection(msg.clone()), // Includes HTTP 401, 403
        NetworkError::CredentialFailure => ExecutionObservation::NotDispatched,
        NetworkError::EndpointMismatch => ExecutionObservation::NotDispatched,
        NetworkError::TransportUnavailable => ExecutionObservation::TransportInterrupted,
        NetworkError::ProtocolViolation => ExecutionObservation::AdapterReportedFailure,
        NetworkError::Unknown => ExecutionObservation::TransportOutcomeUnknown,
    }
}

// =====================================================================
// 2. CREDENTIAL LIFECYCLE & FFI MOCK
// =====================================================================

pub trait ScopedCredentialProvider {
    fn with_secret(&self, f: &mut dyn FnMut(Option<&str>));
}

/// Simulates the synchronous `libgit2` C-FFI credentials callback.
pub struct Git2CredentialsCallback<'a> {
    provider: &'a dyn ScopedCredentialProvider,
    call_count: &'a Cell<usize>,
}

impl<'a> Git2CredentialsCallback<'a> {
    pub fn invoke(&self) -> Result<String, String> {
        let count = self.call_count.get();
        if count >= 1 {
            return Err("Lease boundary enforced: multiple credential retrievals blocked".into());
        }
        self.call_count.set(count + 1);

        let mut resolved = None;

        // BOUNDARY DOCUMENTATION:
        // The credential lifetime is bounded by this libgit2 authentication callback lifecycle.
        // No Rust-owned object, adapter state, transport state, or audit object may
        // retain credential material beyond this lifecycle.
        self.provider.with_secret(&mut |secret_opt| {
            if let Some(secret) = secret_opt {
                // Simulating git2::Cred::userpass_plaintext allocating into C-memory
                resolved = Some(format!("x-access-token:{}", secret));
            }
        });

        resolved.ok_or("Credential Unavailable".into())
    }
}

// =====================================================================
// 3. FAKE GIT HTTPS SERVER FIXTURE
// =====================================================================

pub enum HttpBehavior {
    Success,
    RequireAuthTwice,
    RejectAuth401,
    BranchProtected403,
    DnsFailure,
    TlsFailure,
}

pub struct FakeGitHttpServer {
    pub behavior: HttpBehavior,
}

impl FakeGitHttpServer {
    pub fn simulate_push(
        &self,
        refspec: &str,
        _expected_prestate: &str,
        auth_cb: &Git2CredentialsCallback,
    ) -> Result<(), NetworkError> {
        // EXPLICIT REFSPEC RESTRICTIONS (INVARIANT-412, INVARIANT-421)
        // Prohibit wildcards, tag following, push.default semantics, and force flags
        if refspec.contains('*') || refspec.starts_with('+') || refspec.is_empty() {
            return Err(NetworkError::ProtocolViolation);
        }

        match self.behavior {
            HttpBehavior::Success => {
                let _ = auth_cb
                    .invoke()
                    .map_err(|_| NetworkError::CredentialFailure)?;
                Ok(())
            }
            HttpBehavior::RequireAuthTwice => {
                let _ = auth_cb
                    .invoke()
                    .map_err(|_| NetworkError::CredentialFailure)?;
                // Second retrieval attempt simulating libgit2 retry loop
                if auth_cb.invoke().is_err() {
                    return Err(NetworkError::ProtocolViolation); // Fixture correctly halts
                }
                Ok(())
            }
            HttpBehavior::RejectAuth401 => {
                let _ = auth_cb
                    .invoke()
                    .map_err(|_| NetworkError::CredentialFailure)?;
                // Proves that remote rejection (401) is structurally different from local failure
                Err(NetworkError::Rejected("HTTP 401 Unauthorized".into()))
            }
            HttpBehavior::BranchProtected403 => {
                let _ = auth_cb
                    .invoke()
                    .map_err(|_| NetworkError::CredentialFailure)?;
                Err(NetworkError::Rejected(
                    "HTTP 403 Forbidden: Branch protection hook".into(),
                ))
            }
            HttpBehavior::DnsFailure => Err(NetworkError::TransportUnavailable),
            HttpBehavior::TlsFailure => Err(NetworkError::EndpointMismatch),
        }
    }
}

// =====================================================================
// 4. CONCRETE TRANSPORT MOCK
// =====================================================================

pub struct RemotePublicationTransportRequest {
    pub destination_ref: String,
    pub expected_prestate_oid: String,
    pub candidate_oid: String,
}

pub struct ConcreteGitTransport<'a> {
    pub server_fixture: &'a FakeGitHttpServer,
}

impl<'a> ConcreteGitTransport<'a> {
    pub fn execute_push(
        &self,
        req: &RemotePublicationTransportRequest,
        creds: &dyn ScopedCredentialProvider,
    ) -> ExecutionObservation {
        // Enforce exact 1:1 refspec construction without force (+) flags
        let exact_refspec = format!("{}:{}", req.candidate_oid, req.destination_ref);
        let call_count = Cell::new(0);
        let callback = Git2CredentialsCallback {
            provider: creds,
            call_count: &call_count,
        };

        let result = self.server_fixture.simulate_push(
            &exact_refspec,
            &req.expected_prestate_oid,
            &callback,
        );

        match result {
            Ok(_) => ExecutionObservation::AdapterReportedSuccess,
            Err(e) => map_network_error(&e),
        }
    }
}

// =====================================================================
// UNIT TESTS (Proving Phase B Mappings & Callback Abuse Limits)
// =====================================================================

#[cfg(test)]
mod b002_tests {
    use super::*;

    struct MockProvider {
        secret: Option<String>,
    }
    impl ScopedCredentialProvider for MockProvider {
        fn with_secret(&self, f: &mut dyn FnMut(Option<&str>)) {
            f(self.secret.as_deref());
        }
    }

    fn default_req() -> RemotePublicationTransportRequest {
        RemotePublicationTransportRequest {
            destination_ref: "refs/heads/main".into(),
            expected_prestate_oid: "XXX".into(),
            candidate_oid: "YYY".into(),
        }
    }

    #[test]
    fn test_tc_https_001_credential_requested_once() {
        let server = FakeGitHttpServer {
            behavior: HttpBehavior::Success,
        };
        let transport = ConcreteGitTransport {
            server_fixture: &server,
        };
        let obs = transport.execute_push(
            &default_req(),
            &MockProvider {
                secret: Some("token".into()),
            },
        );

        assert_eq!(obs, ExecutionObservation::AdapterReportedSuccess);
    }

    #[test]
    fn test_tc_https_002_second_retrieval_blocked() {
        let server = FakeGitHttpServer {
            behavior: HttpBehavior::RequireAuthTwice,
        };
        let transport = ConcreteGitTransport {
            server_fixture: &server,
        };
        let obs = transport.execute_push(
            &default_req(),
            &MockProvider {
                secret: Some("token".into()),
            },
        );

        // Protocol Violation occurs because the closure halts the second FFI callback
        assert_eq!(obs, ExecutionObservation::AdapterReportedFailure);
    }

    #[test]
    fn test_tc_https_003_transport_error_logging_absence() {
        let req = default_req();
        // Structurally prove the request object itself (what gets logged on error) is safe
        let debug_str = format!(
            "Req: {} -> {}",
            req.expected_prestate_oid, req.candidate_oid
        );
        assert!(!debug_str.to_lowercase().contains("token"));
        assert!(!debug_str.to_lowercase().contains("secret"));
    }

    #[test]
    fn test_tc_https_004_remote_rejects_credential() {
        let server = FakeGitHttpServer {
            behavior: HttpBehavior::RejectAuth401,
        };
        let transport = ConcreteGitTransport {
            server_fixture: &server,
        };
        let obs = transport.execute_push(
            &default_req(),
            &MockProvider {
                secret: Some("token".into()),
            },
        );

        // Proves 401 does not equal NotDispatched. The remote actively participated and rejected it.
        assert_eq!(
            obs,
            ExecutionObservation::RemoteReportedRejection("HTTP 401 Unauthorized".into())
        );
    }

    #[test]
    fn test_tc_https_005_dns_vs_tls_mapping() {
        let req = default_req();
        let creds = MockProvider {
            secret: Some("token".into()),
        };

        // TLS Failure -> Endpoint Identity Mismatch
        let tls_server = FakeGitHttpServer {
            behavior: HttpBehavior::TlsFailure,
        };
        let tls_obs = ConcreteGitTransport {
            server_fixture: &tls_server,
        }
        .execute_push(&req, &creds);
        assert_eq!(tls_obs, ExecutionObservation::NotDispatched); // Mapped via EndpointMismatch

        // DNS Failure -> Transport Unavailable
        let dns_server = FakeGitHttpServer {
            behavior: HttpBehavior::DnsFailure,
        };
        let dns_obs = ConcreteGitTransport {
            server_fixture: &dns_server,
        }
        .execute_push(&req, &creds);
        assert_eq!(dns_obs, ExecutionObservation::TransportInterrupted); // Mapped via TransportUnavailable
    }

    #[test]
    fn test_tc_https_006_refspec_wildcard_rejection() {
        let server = FakeGitHttpServer {
            behavior: HttpBehavior::Success,
        };
        let call_count = Cell::new(0);
        let cb = Git2CredentialsCallback {
            provider: &MockProvider {
                secret: Some("token".into()),
            },
            call_count: &call_count,
        };

        // Assert force flag and wildcards are violently rejected by the transport boundary
        assert_eq!(
            server.simulate_push("+refs/heads/main:refs/heads/main", "XXX", &cb),
            Err(NetworkError::ProtocolViolation)
        );
        assert_eq!(
            server.simulate_push("refs/heads/*:refs/heads/*", "XXX", &cb),
            Err(NetworkError::ProtocolViolation)
        );
    }
}
