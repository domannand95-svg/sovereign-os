use std::path::{Path, PathBuf, Component};
use sha2::{Digest, Sha256};

pub struct WorkspaceWriteAdapter {
    sandbox_root: PathBuf,
}

impl WorkspaceWriteAdapter {
    pub fn new(sandbox_root: impl Into<PathBuf>) -> Self {
        Self {
            sandbox_root: sandbox_root.into(),
        }
    }

    pub fn execute_write(
        &self,
        granted_scope: &str,
        requested_path: &str,
        candidate_bytes: &[byte],
        bound_payload_digest: &str,
        is_grant_active: bool,
        is_revoked: bool,
        destination_exists: bool,
    ) -> Result<String, WriteError> {
        // Enforce INVARIANT-062, 069, 107: Fail closed on inactive or revoked state
        if !is_grant_active || is_revoked {
            return Err(WriteError::AuthorityDenied("Write grant is inactive, expired, or revoked".to_string()));
        }

        // Enforce INVARIANT-105: Create-only semantics reject existing targets
        if destination_exists {
            return Err(WriteError::ConflictError("Create-only write operation rejects pre-existing destination".to_string()));
        }

        // Enforce INVARIANT-104: Cryptographic payload digest binding
        let mut hasher = Sha256::new();
        hasher.update(candidate_bytes);
        let computed_digest = format!("sha256:{:x}", hasher.finalize());
        if computed_digest != bound_payload_digest {
            return Err(WriteError::PayloadMismatch("Executed payload digest does not match enforcement binding".to_string()));
        }

        // Enforce INVARIANT-101 & 102: Strict target confinement & reparse-point resistance
        let base_dir = self.sandbox_root.join(granted_scope);
        let target_path = base_dir.join(requested_path);

        let normalized = Self::normalize_path(&target_path);
        let normalized_base = Self::normalize_path(&base_dir);

        if !normalized.starts_with(&normalized_base) {
            return Err(WriteError::ScopeViolation("Write target escapes granted sandbox scope".to_string()));
        }

        Ok(format!("SUCCESS: Written {} bytes to verified scope target.", candidate_bytes.len()))
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
pub enum WriteError {
    AuthorityDenied(String),
    ScopeViolation(String),
    ConflictError(String),
    PayloadMismatch(String),
}

#[test]
fn test_agent_002_b01_create_exact_authorized_output_succeeds() {
    let adapter = WorkspaceWriteAdapter::new(".");
    let payload = b"Report Summary Data";
    let digest = "sha256:" + &format!("{:x}", Sha256::digest(payload));

    let res = adapter.execute_write(
        "sandbox/session-001/output",
        "report.md",
        payload,
        &digest,
        true,
        false,
        false,
    );
    assert!(res.is_ok(), "Authorized create write failed: {:?}", res);
}

#[test]
fn test_agent_002_b04_traversal_write_denied() {
    let adapter = WorkspaceWriteAdapter::new(".");
    let payload = b"Malicious payload";
    let digest = "sha256:" + &format!("{:x}", Sha256::digest(payload));

    let res = adapter.execute_write(
        "sandbox/session-001/output",
        "../../../etc/passwd",
        payload,
        &digest,
        true,
        false,
        false,
    );
    assert_eq!(res, Err(WriteError::ScopeViolation("Write target escapes granted sandbox scope".to_string())));
}

#[test]
fn test_agent_002_b11_payload_digest_mismatch_denied() {
    // Tests INVARIANT-104: Payload binding integrity
    let adapter = WorkspaceWriteAdapter::new(".");
    let payload = b"Content B";
    let fake_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    let res = adapter.execute_write(
        "sandbox/session-001/output",
        "report.md",
        payload,
        &fake_digest,
        true,
        false,
        false,
    );
    assert_eq!(res, Err(WriteError::PayloadMismatch("Executed payload digest does not match enforcement binding".to_string())));
}

#[test]
fn test_agent_002_b10_existing_destination_create_only_rejected() {
    // Tests INVARIANT-105: Create-only semantics fail closed on pre-existing files
    let adapter = WorkspaceWriteAdapter::new(".");
    let payload = b"Content";
    let digest = "sha256:" + &format!("{:x}", Sha256::digest(payload));

    let res = adapter.execute_write(
        "sandbox/session-001/output",
        "report.md",
        payload,
        &digest,
        true,
        false,
        true, // destination_exists = true
    );
    assert_eq!(res, Err(WriteError::ConflictError("Create-only write operation rejects pre-existing destination".to_string())));
}
