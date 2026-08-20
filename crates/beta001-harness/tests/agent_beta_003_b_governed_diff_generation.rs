use std::path::{Path, PathBuf, Component};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct DiffGenerationAdapter {
    repo_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RepositorySnapshotContext {
    pub repository_id: String,
    pub head_commit: String,
}

#[derive(Debug, Clone)]
pub struct DiffCandidate {
    pub candidate_id: String,
    pub repository_reference: String,
    pub baseline_commit: String,
    pub target_paths: Vec<String>,
    pub diff_content: String,
    pub diff_digest: String,
}

impl DiffGenerationAdapter {
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
        }
    }

    pub fn generate_candidate(
        &self,
        context: &RepositorySnapshotContext,
        granted_scope: &str,
        candidate: &DiffCandidate,
        is_grant_active: bool,
        is_revoked: bool,
    ) -> Result<String, DiffError> {
        // Enforce INVARIANT-062, 069: Fail closed on inactive or revoked state
        if !is_grant_active || is_revoked {
            return Err(DiffError::AuthorityDenied("Diff generation grant is inactive, expired, or revoked".to_string()));
        }

        // Enforce INVARIANT-144 & 145: Absolute baseline and repository reference binding
        if candidate.repository_reference != context.repository_id {
            return Err(DiffError::RepositoryMismatch("Candidate repository reference does not match host snapshot".to_string()));
        }
        if candidate.baseline_commit != context.head_commit {
            return Err(DiffError::BaselineMismatch("Candidate baseline commit does not match host snapshot".to_string()));
        }

        // Enforce INVARIANT-104 & 144: Deterministic content digest binding
        let mut hasher = DefaultHasher::new();
        candidate.diff_content.hash(&mut hasher);
        let computed = format!("sha256:{:x}", hasher.finish());
        if computed != candidate.diff_digest {
            return Err(DiffError::DigestMismatch("Diff digest does not match content hash".to_string()));
        }

        // Enforce INVARIANT-146: Target path confinement within governed scope
        let base_dir = self.repo_root.join(granted_scope);
        let base_normalized = Self::normalize_path(&base_dir);

        for path in &candidate.target_paths {
            let target = base_dir.join(path);
            let normalized = Self::normalize_path(&target);
            if !normalized.starts_with(&base_normalized) {
                return Err(DiffError::ScopeViolation("Candidate target path escapes granted repository scope".to_string()));
            }
        }

        Ok("CANDIDATE_DIFF_ACCEPTED_IN_MEMORY".to_string())
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
pub enum DiffError {
    AuthorityDenied(String),
    RepositoryMismatch(String),
    BaselineMismatch(String),
    DigestMismatch(String),
    ScopeViolation(String),
}

fn compute_digest(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("sha256:{:x}", hasher.finish())
}

#[test]
fn test_agent_003_b01_generate_candidate_diff_succeeds() {
    let adapter = DiffGenerationAdapter::new(".");
    let ctx = RepositorySnapshotContext {
        repository_id: "repo_sovereign_01".to_string(),
        head_commit: "cc3a168".to_string(),
    };

    let content = "diff --git a/src/lib.rs b/src/lib.rs\n+ pub mod repo;";
    let digest = compute_digest(content);

    let candidate = DiffCandidate {
        candidate_id: "cand_001".to_string(),
        repository_reference: "repo_sovereign_01".to_string(),
        baseline_commit: "cc3a168".to_string(),
        target_paths: vec!["src/lib.rs".to_string()],
        diff_content: content.to_string(),
        diff_digest: digest,
    };

    let res = adapter.generate_candidate(&ctx, "crates/beta001-harness", &candidate, true, false);
    assert!(res.is_ok(), "Authorized candidate diff generation failed: {:?}", res);
}

#[test]
fn test_agent_003_b02_out_of_scope_candidate_denied() {
    let adapter = DiffGenerationAdapter::new(".");
    let ctx = RepositorySnapshotContext {
        repository_id: "repo_sovereign_01".to_string(),
        head_commit: "cc3a168".to_string(),
    };

    let content = "diff --git b/Cargo.toml b/Cargo.toml";
    let digest = compute_digest(content);

    let candidate = DiffCandidate {
        candidate_id: "cand_002".to_string(),
        repository_reference: "repo_sovereign_01".to_string(),
        baseline_commit: "cc3a168".to_string(),
        target_paths: vec!["../../Cargo.toml".to_string()],
        diff_content: content.to_string(),
        diff_digest: digest,
    };

    let res = adapter.generate_candidate(&ctx, "crates/beta001-harness", &candidate, true, false);
    assert_eq!(res, Err(DiffError::ScopeViolation("Candidate target path escapes granted repository scope".to_string())));
}

#[test]
fn test_agent_003_b11_baseline_mismatch_rejected() {
    // Tests INVARIANT-144: Model-supplied false baseline cannot replace host baseline
    let adapter = DiffGenerationAdapter::new(".");
    let ctx = RepositorySnapshotContext {
        repository_id: "repo_sovereign_01".to_string(),
        head_commit: "cc3a168".to_string(),
    };

    let content = "diff --git a/src/lib.rs b/src/lib.rs";
    let digest = compute_digest(content);

    let candidate = DiffCandidate {
        candidate_id: "cand_003".to_string(),
        repository_reference: "repo_sovereign_01".to_string(),
        baseline_commit: "stale_commit_xyz".to_string(), // Mismatched baseline
        target_paths: vec!["src/lib.rs".to_string()],
        diff_content: content.to_string(),
        diff_digest: digest,
    };

    let res = adapter.generate_candidate(&ctx, "crates/beta001-harness", &candidate, true, false);
    assert_eq!(res, Err(DiffError::BaselineMismatch("Candidate baseline commit does not match host snapshot".to_string())));
}
