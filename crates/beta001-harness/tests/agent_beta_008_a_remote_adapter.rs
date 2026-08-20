#[derive(Debug, PartialEq, Eq)]
pub enum NetworkError {
    Timeout,
    Rejected(String),
    CredentialFailure(String),
    ProtocolViolation,
}

pub struct GitPushRequest {
    pub remote_url: String,
    pub refspec: String,
}

// Dyn-compatible trait
pub trait ScopedCredentialProvider {
    fn get_secret(&self) -> Option<String>;
}

pub trait NetworkTransport {
    fn execute_push(
        &self,
        req: &GitPushRequest,
        creds: &dyn ScopedCredentialProvider,
    ) -> Result<(), NetworkError>;
}

pub struct RemoteAdapter<'a> {
    pub network_client: &'a dyn NetworkTransport,
}

impl<'a> RemoteAdapter<'a> {
    pub fn dispatch_exact(
        &self,
        push_req: &GitPushRequest,
        credentials: &dyn ScopedCredentialProvider,
    ) -> Result<(), NetworkError> {
        self.network_client.execute_push(push_req, credentials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCredentialProvider {
        secret: Option<String>,
    }

    impl ScopedCredentialProvider for MockCredentialProvider {
        fn get_secret(&self) -> Option<String> {
            self.secret.clone()
        }
    }

    struct MockNetwork {
        expected_result: Result<(), NetworkError>,
    }

    impl NetworkTransport for MockNetwork {
        fn execute_push(
            &self,
            _req: &GitPushRequest,
            _creds: &dyn ScopedCredentialProvider,
        ) -> Result<(), NetworkError> {
            match &self.expected_result {
                Ok(_) => Ok(()),
                Err(e) => match e {
                    NetworkError::Timeout => Err(NetworkError::Timeout),
                    NetworkError::Rejected(s) => Err(NetworkError::Rejected(s.clone())),
                    NetworkError::CredentialFailure(s) => {
                        Err(NetworkError::CredentialFailure(s.clone()))
                    }
                    NetworkError::ProtocolViolation => Err(NetworkError::ProtocolViolation),
                },
            }
        }
    }

    fn default_request() -> GitPushRequest {
        GitPushRequest {
            remote_url: "https://example.com/repo.git".into(),
            refspec: "refs/heads/main".into(),
        }
    }

    #[test]
    fn tc_remote_001_successful_push() {
        let adapter = RemoteAdapter {
            network_client: &MockNetwork {
                expected_result: Ok(()),
            },
        };
        let obs = adapter.dispatch_exact(
            &default_request(),
            &MockCredentialProvider {
                secret: Some("token".into()),
            },
        );
        assert!(obs.is_ok());
    }

    #[test]
    fn tc_remote_002_timeout() {
        let adapter = RemoteAdapter {
            network_client: &MockNetwork {
                expected_result: Err(NetworkError::Timeout),
            },
        };
        let obs = adapter.dispatch_exact(
            &default_request(),
            &MockCredentialProvider {
                secret: Some("token".into()),
            },
        );
        assert_eq!(obs, Err(NetworkError::Timeout));
    }

    #[test]
    fn tc_remote_003_rejected() {
        let adapter = RemoteAdapter {
            network_client: &MockNetwork {
                expected_result: Err(NetworkError::Rejected("protected".into())),
            },
        };
        let obs = adapter.dispatch_exact(
            &default_request(),
            &MockCredentialProvider {
                secret: Some("token".into()),
            },
        );
        assert_eq!(obs, Err(NetworkError::Rejected("protected".into())));
    }

    #[test]
    fn tc_remote_004_credential_failure() {
        let adapter = RemoteAdapter {
            network_client: &MockNetwork {
                expected_result: Err(NetworkError::CredentialFailure("none".into())),
            },
        };
        let obs =
            adapter.dispatch_exact(&default_request(), &MockCredentialProvider { secret: None });
        assert_eq!(obs, Err(NetworkError::CredentialFailure("none".into())));
    }

    #[test]
    fn tc_remote_005_protocol_violation() {
        let adapter = RemoteAdapter {
            network_client: &MockNetwork {
                expected_result: Err(NetworkError::ProtocolViolation),
            },
        };
        let obs = adapter.dispatch_exact(
            &default_request(),
            &MockCredentialProvider {
                secret: Some("token".into()),
            },
        );
        assert_eq!(obs, Err(NetworkError::ProtocolViolation));
    }
}
