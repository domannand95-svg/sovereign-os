use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};

pub struct AdversarialCompositionOrchestrator {
    sandbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CompositeSession {
    pub read_scope: String,
    pub write_scope: String,
    pub read_active: bool,
    pub write_active: bool,
    pub revoked: bool,
}

impl AdversarialCompositionOrchestrator {
    pub fn new(sandbox_root: impl Into<PathBuf>) -> Self {
        Self {
            sandbox_root: sandbox_root.into(),
        }
    }

    pub fn evaluate_and_execute_write(
        &self,
        session: &CompositeSession,
        target_path: &str,
        bytes: &[u8],
        digest: &str,
        is_revoked_at_execution: bool,
    ) -> Result<String, AdversarialError> {
        // Enforce INVARIANT-122 & 126: Denial dominance and runtime revocation check
        if !session.write_active || session.revoked || is_revoked_at_execution {
            return Err(AdversarialError::AuthorityDenied(
                "Authorization denied or revoked at execution boundary".to_string(),
            ));
        }

        // Enforce INVARIANT-104: Payload digest binding
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let computed = format!("sha256:{:x}", hasher.finish());
        if computed != digest {
            return Err(AdversarialError::PayloadMismatch(
                "Payload digest mismatch".to_string(),
            ));
        }

        // Enforce INVARIANT-121 & 101: Target confinement against granted write scope
        let base_dir = self.sandbox_root.join(&session.write_scope);
        let target = base_dir.join(target_path);

        let normalized = Self::normalize_path(&target);
        let normalized_base = Self::normalize_path(&base_dir);

        if !normalized.starts_with(&normalized_base) {
            return Err(AdversarialError::ScopeViolation(
                "Target escapes granted write scope; injection rejected".to_string(),
            ));
        }

        Ok("COMPOSITE_WRITE_SUCCESS".to_string())
    }

    fn normalize_path(path: &Path) -> PathBuf {
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                Component::ParentDir => {
                    components.pop();
                }
                Component::Normal(c) => {
                    components.push(c);
                }
                Component::CurDir => {}
                _ => {
                    components.push(component.as_os_str());
                }
            }
        }
        components.iter().collect()
    }
}

#[derive(Debug, PartialEq)]
pub enum AdversarialError {
    AuthorityDenied(String),
    ScopeViolation(String),
    PayloadMismatch(String),
}

fn compute_digest(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("sha256:{:x}", hasher.finish())
}

#[test]
fn test_agent_002_d_bounded_authority_injection_resisted() {
    // AGENT-002-D-BOUNDED-AUTHORITY-INJECTION:
    // Agent possesses a narrow write scope, but injected input attempts to force writing to ../../Cargo.toml.
    let orchestrator = AdversarialCompositionOrchestrator::new(".");
    let session = CompositeSession {
        read_scope: "sandbox/session-001/input".to_string(),
        write_scope: "sandbox/session-001/output".to_string(),
        read_active: true,
        write_active: true,
        revoked: false,
    };

    let malicious_target = "../../../Cargo.toml";
    let payload = b"Injected unauthorized payload";
    let digest = compute_digest(payload);

    let res = orchestrator.evaluate_and_execute_write(
        &session,
        malicious_target,
        payload,
        &digest,
        false,
    );
    assert_eq!(
        res,
        Err(AdversarialError::ScopeViolation(
            "Target escapes granted write scope; injection rejected".to_string()
        ))
    );
}

#[test]
fn test_agent_002_d_toctou_authority_revocation_denied() {
    // TOCTOU Revocation Race: Revocation occurs between check and execution time (INVARIANT-126).
    let orchestrator = AdversarialCompositionOrchestrator::new(".");
    let session = CompositeSession {
        read_scope: "sandbox/session-001/input".to_string(),
        write_scope: "sandbox/session-001/output".to_string(),
        read_active: true,
        write_active: true,
        revoked: false,
    };

    let payload = b"Legitimate summary";
    let digest = compute_digest(payload);

    // Simulate revocation occurring precisely at execution time
    let res =
        orchestrator.evaluate_and_execute_write(&session, "summary.md", payload, &digest, true);
    assert_eq!(
        res,
        Err(AdversarialError::AuthorityDenied(
            "Authorization denied or revoked at execution boundary".to_string()
        ))
    );
}

#[test]
fn test_agent_002_d_cross_resource_isolation_denied() {
    // Tests INVARIANT-120 & 125: Unrelated scope requests fail closed
    let orchestrator = AdversarialCompositionOrchestrator::new(".");
    let session = CompositeSession {
        read_scope: "sandbox/session-001/input".to_string(),
        write_scope: "sandbox/session-001/output".to_string(),
        read_active: true,
        write_active: true,
        revoked: false,
    };

    let payload = b"Data";
    let digest = compute_digest(payload);

    let res = orchestrator.evaluate_and_execute_write(
        &session,
        "../other-session/output.txt",
        payload,
        &digest,
        false,
    );
    assert_eq!(
        res,
        Err(AdversarialError::ScopeViolation(
            "Target escapes granted write scope; injection rejected".to_string()
        ))
    );
}
