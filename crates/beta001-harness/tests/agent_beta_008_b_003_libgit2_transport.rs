use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// =====================================================================
// 1. CORE DOMAIN MOCKS
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkError {
    Timeout,
    Rejected(String),
    CredentialFailure,
    EndpointMismatch,
    TransportUnavailable,
    ProtocolViolation,
    Unknown,
}

pub trait ScopedCredentialProvider {
    fn with_secret(&self, f: &mut dyn FnMut(Option<&str>));
}

pub struct RemotePublicationTransportRequest {
    pub execution_id: String,
    pub canonical_endpoint: String,
    pub source_ref: String,
    pub destination_ref: String,
    pub expected_prestate_oid: String,
    pub candidate_oid: String,
}

#[derive(Debug, Clone)]
pub struct GitPushRequest {
    pub endpoint: String,
    pub source_ref: String,
    pub destination_ref: String,
    pub expected_old: String,
    pub new: String,
}

pub trait NetworkTransport {
    fn execute_push(
        &self,
        request: &GitPushRequest,
        credentials: &dyn ScopedCredentialProvider,
    ) -> Result<(), NetworkError>;
}

// =====================================================================
// 2. CONCRETE LIBGIT2 TRANSPORT (AGENT-BETA-008-B-003)
// =====================================================================

#[derive(Debug)]
pub struct LibGit2NetworkTransport {
    pub local_repo_path: PathBuf,
}

impl NetworkTransport for LibGit2NetworkTransport {
    fn execute_push(
        &self,
        request: &GitPushRequest,
        credentials: &dyn ScopedCredentialProvider,
    ) -> Result<(), NetworkError> {
        
        // 1. REFSPEC SCOPE RESTRICTION (INVARIANT-412, INVARIANT-421)
        if request.destination_ref.contains('*') || request.source_ref.contains('*') {
            return Err(NetworkError::ProtocolViolation);
        }
        if request.destination_ref.starts_with('+') || request.source_ref.starts_with('+') {
            return Err(NetworkError::ProtocolViolation);
        }
        if request.source_ref.is_empty() || request.destination_ref.is_empty() {
            return Err(NetworkError::ProtocolViolation);
        }

        let repo = git2::Repository::open(&self.local_repo_path)
            .map_err(|_| NetworkError::ProtocolViolation)?;

        let mut remote = repo.remote_anonymous(&request.endpoint)
            .map_err(|_| NetworkError::EndpointMismatch)?;

        let mut callbacks = git2::RemoteCallbacks::new();

        // 2. CREDENTIAL CALLBACK BOUNDARY
        callbacks.credentials(move |_url, _username, _allowed| {
            invoke_credential_callback(credentials)
        });

        // 3. REMOTE REJECTION CALLBACK
        let rejection_msg: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let rej_clone = Arc::clone(&rejection_msg);

        callbacks.push_update_reference(move |_refname, status| {
            if let Some(error_msg) = status {
                let mut guard = rej_clone.lock().unwrap();
                *guard = Some(error_msg.to_string());
                return Err(git2::Error::from_str(error_msg));
            }
            Ok(())
        });

        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        // 4. DISPATCH
        let refspec = format!("{}:{}", request.source_ref, request.destination_ref);
        
        let push_result = remote.push(&[&refspec], Some(&mut push_opts));

        let current_rejection = {
            let guard = rejection_msg.lock().unwrap();
            guard.clone()
        };

        if let Some(msg) = current_rejection {
            return Err(NetworkError::Rejected(msg));
        }

        match push_result {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_string = e.message().to_lowercase();
                if err_string.contains("non-fast-forward") || err_string.contains("rejected") || err_string.contains("locked") {
                    return Err(NetworkError::Rejected(e.message().to_string()));
                }

                if e.code() == git2::ErrorCode::Auth {
                    Err(NetworkError::CredentialFailure)
                } else if e.code() == git2::ErrorCode::NotFound {
                    Err(NetworkError::EndpointMismatch)
                } else {
                    Err(NetworkError::Rejected(e.message().to_string()))
                }
            }
        }
    }
}

pub fn invoke_credential_callback(provider: &dyn ScopedCredentialProvider) -> Result<git2::Cred, git2::Error> {
    let mut cred_out = Err(git2::Error::from_str("Credential Unavailable"));
    
    provider.with_secret(&mut |secret_opt| {
        if let Some(secret) = secret_opt {
            cred_out = git2::Cred::userpass_plaintext("x-access-token", secret);
        }
    });
    
    cred_out
}

// =====================================================================
// UNIT TESTS (Physical libgit2 Validations)
// =====================================================================

#[cfg(test)]
mod b003_tests {
    use super::*;
    use tempfile::TempDir;

    struct MockProvider { secret: Option<String> }
    impl ScopedCredentialProvider for MockProvider {
        fn with_secret(&self, f: &mut dyn FnMut(Option<&str>)) { f(self.secret.as_deref()); }
    }

    fn setup_physical_repos() -> (TempDir, TempDir, git2::Repository) {
        let local_dir = tempfile::tempdir().unwrap();
        let remote_dir = tempfile::tempdir().unwrap();

        let _remote_repo = git2::Repository::init_bare(remote_dir.path()).unwrap();
        let local_repo = git2::Repository::init(local_dir.path()).unwrap();

        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = local_repo.index().unwrap();
            index.write_tree().unwrap()
        };
        {
            let tree = local_repo.find_tree(tree_id).unwrap();
            local_repo.commit(Some("refs/heads/feature/example"), &sig, &sig, "Initial", &tree, &[]).unwrap();
        }

        (local_dir, remote_dir, local_repo)
    }

    /// Normalizes path strings to valid file:// URLs across Windows and POSIX
    fn path_to_file_url(path: &std::path::Path) -> String {
        let path_str = path.to_str().unwrap().replace('\\', "/");
        if path_str.starts_with('/') {
            format!("file://{}", path_str)
        } else {
            format!("file:///{}", path_str)
        }
    }

    #[test]
    fn test_tc_libgit2_001_successful_push() {
        let (local_dir, remote_dir, _local_repo) = setup_physical_repos();
        
        let transport = LibGit2NetworkTransport { local_repo_path: local_dir.path().to_path_buf() };
        let req = GitPushRequest {
            endpoint: path_to_file_url(remote_dir.path()),
            source_ref: "refs/heads/feature/example".into(),
            destination_ref: "refs/heads/feature/example".into(),
            expected_old: "0000000000000000000000000000000000000000".into(),
            new: "YYY".into(),
        };

        let creds = MockProvider { secret: Some("token".into()) };
        let result = transport.execute_push(&req, &creds);
        
        assert!(result.is_ok(), "Physical libgit2 push failed: {:?}", result);
        
        let remote_repo = git2::Repository::open(remote_dir.path()).unwrap();
        let remote_ref = remote_repo.find_reference("refs/heads/feature/example");
        assert!(remote_ref.is_ok());
    }

    #[test]
    fn test_tc_libgit2_002_invalid_destination_ref() {
        let req = GitPushRequest {
            endpoint: "file:///dev/null".into(),
            source_ref: "refs/heads/feature".into(),
            destination_ref: "refs/heads/*".into(),
            expected_old: "XXX".into(),
            new: "YYY".into(),
        };

        let transport = LibGit2NetworkTransport { local_repo_path: PathBuf::from(".") };
        let result = transport.execute_push(&req, &MockProvider { secret: None });
        
        assert_eq!(result, Err(NetworkError::ProtocolViolation));
    }

    #[test]
    fn test_tc_libgit2_003_missing_credential() {
        let provider = MockProvider { secret: None };
        let result = invoke_credential_callback(&provider);
        
        match result {
            Err(e) => assert_eq!(e.message(), "Credential Unavailable"),
            Ok(_) => panic!("Expected credential failure"),
        }
    }

    #[test]
    fn test_tc_libgit2_004_remote_rejection() {
        let (local_dir, remote_dir, local_repo) = setup_physical_repos();
        
        let transport = LibGit2NetworkTransport { local_repo_path: local_dir.path().to_path_buf() };
        let req_initial = GitPushRequest {
            endpoint: path_to_file_url(remote_dir.path()),
            source_ref: "refs/heads/feature/example".into(),
            destination_ref: "refs/heads/feature/example".into(),
            expected_old: "".into(), new: "".into(),
        };
        transport.execute_push(&req_initial, &MockProvider { secret: None }).unwrap();

        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = local_repo.index().unwrap().write_tree().unwrap();
        {
            let tree = local_repo.find_tree(tree_id).unwrap();
            local_repo.commit(Some("refs/heads/conflict"), &sig, &sig, "Conflict", &tree, &[]).unwrap();
        }

        let req_conflict = GitPushRequest {
            endpoint: path_to_file_url(remote_dir.path()),
            source_ref: "refs/heads/conflict".into(),
            destination_ref: "refs/heads/feature/example".into(),
            expected_old: "".into(), new: "".into(),
        };

        let result = transport.execute_push(&req_conflict, &MockProvider { secret: None });
        assert!(matches!(result, Err(NetworkError::Rejected(_))));
    }

    #[test]
    fn test_tc_libgit2_005_credential_retention_check() {
        let transport = LibGit2NetworkTransport { local_repo_path: PathBuf::from("/tmp/repo") };
        let debug_out = format!("{:?}", transport);
        
        assert!(!debug_out.to_lowercase().contains("secret"));
        assert!(!debug_out.to_lowercase().contains("token"));
        assert!(!debug_out.to_lowercase().contains("cred"));
    }

    #[test]
    fn test_tc_libgit2_006_ref_scope_restriction() {
        let req = GitPushRequest {
            endpoint: "file:///dev/null".into(),
            source_ref: "refs/heads/feature".into(),
            destination_ref: "+refs/heads/feature".into(),
            expected_old: "XXX".into(),
            new: "YYY".into(),
        };

        let transport = LibGit2NetworkTransport { local_repo_path: PathBuf::from(".") };
        let result = transport.execute_push(&req, &MockProvider { secret: None });
        
        assert_eq!(result, Err(NetworkError::ProtocolViolation));
    }
}
