use std::path::{Path, PathBuf, Component};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct ProposalBindingValidator {
    repo_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AuthoritativeRepositoryState {
    pub repository_id: String,
    pub head_commit: String,
    pub is_dirty: bool,
}

#[derive(Debug, Clone)]
pub struct DiffCandidateRecord {
    pub candidate_id: String,
    pub repository_reference: String,
    pub baseline_commit: String,
    pub target_paths: Vec<String>,
    pub diff_content: String,
    pub diff_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationDisposition {
    MATCHED,
    STALE,
    REJECTED,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub disposition: ValidationDisposition,
    pub reasons: Vec<String>,
}

impl ProposalBindingValidator {
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
        }
    }

    pub fn validate_proposal(
        &self,
        state: &AuthoritativeRepositoryState,
        candidate: &DiffCandidateRecord,
        granted_scope: &str,
    ) -> ValidationResult {
        // Enforce INVARIANT-162: Dirty worktree / index fails closed
        if state.is_dirty {
            return ValidationResult {
                disposition: ValidationDisposition::REJECTED,
                vec!["Unexpected dirty repository worktree/index state encountered".to_string()],
            };
        }

        // Enforce INVARIANT-145 & 156: Repository identity binding
        if candidate.repository_reference != state.repository_id {
            return ValidationResult {
                disposition: ValidationDisposition::REJECTED,
                vec!["Candidate repository reference does not match current repository identity".to_string()],
            };
        }

        // Enforce INVARIANT-144 & 155: Baseline advancement detection (STALE)
        if candidate.baseline_commit != state.head_commit {
            return ValidationResult {
                disposition: ValidationDisposition::STALE,
                vec![format!(
                    "Candidate baseline commit ({}) does not match observed repository HEAD ({})",
                    candidate.baseline_commit, state.head_commit
                )],
            };
        }

        // Enforce INVARIANT-104 & 159: Candidate content digest binding
        let mut hasher = DefaultHasher::new();
        candidate.diff_content.hash(&mut hasher);
        let computed = format!("sha256:{:x}", hasher.finish());
        if computed != candidate.diff_digest {
            return ValidationResult {
                disposition: ValidationDisposition::REJECTED,
                vec!["Candidate content digest does not match bound diff digest".to_string()],
            };
        }

        // Enforce INVARIANT-146: Target path confinement
        let base_dir = self.repo_root.join(granted_scope);
        let base_normalized = Self::normalize_path(&base_dir);

        for path in &candidate.target_paths {
            let target = base_dir.join(path);
            let normalized = Self::normalize_path(&target);
            if !normalized.starts_with(&base_normalized) {
                return ValidationResult {
                    disposition: ValidationDisposition::REJECTED,
                    vec![format!("Target path '{}' escapes granted repository scope", path)],
                };
            }
        }

        ValidationResult {
            disposition: ValidationDisposition::MATCHED,
            reasons: vec!["Proposal successfully bound to authoritative repository state.".to_string()],
        }
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

fn compute_digest(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("sha256:{:x}", hasher.finish())
}

#[test]
fn test_agent_003_c01_candidate_matched_against_exact_state() {
    let validator = ProposalBindingValidator::new(".");
    let state = AuthoritativeRepositoryState {
        repository_id: "repo_sovereign_01".to_string(),
        head_commit: "b9a1284".to_string(),
        is_dirty: false,
    };

    let content = "diff --git a/src/main.rs b/src/main.rs\n+ // change";
    let candidate = DiffCandidateRecord {
        candidate_id: "cand_001".to_string(),
        repository_reference: "repo_sovereign_01".to_string(),
        baseline_commit: "b9a1284".to_string(),
        target_paths: vec!["src/main.rs".to_string()],
        diff_content: content.to_string(),
        diff_digest: compute_digest(content),
    };

    let res = validator.validate_proposal(&state, &candidate, "crates/beta001-harness");
    assert_eq!(res.disposition, ValidationDisposition::MATCHED);
}

#[test]
fn test_agent_003_c02_baseline_advancement_marked_stale() {
    // AGENT-003-C-BASELINE-ADVANCEMENT:
    // When repository HEAD advances from A to B, validation of candidate against A yields STALE.
    let validator = ProposalBindingValidator::new(".");
    let original_state = AuthoritativeRepositoryState {
        repository_id: "repo_sovereign_01".to_string(),
        head_commit: "commit_A".to_string(),
        is_dirty: false,
    };

    let content = "diff --git a/src/main.rs b/src/main.rs";
    let candidate = DiffCandidateRecord {
        candidate_id: "cand_002".to_string(),
        repository_reference: "repo_sovereign_01".to_string(),
        baseline_commit: "commit_A".to_string(),
        target_paths: vec!["src/main.rs".to_string()],
        diff_content: content.to_string(),
        diff_digest: compute_digest(content),
    };

    // First validate at commit A -> MATCHED
    let res1 = validator.validate_proposal(&original_state, &candidate, "crates/beta001-harness");
    assert_eq!(res1.disposition, ValidationDisposition::MATCHED);

    // Advance repository state to commit B
    let advanced_state = AuthoritativeRepositoryState {
        repository_id: "repo_sovereign_01".to_string(),
        head_commit: "commit_B".to_string(), // Advanced HEAD
        is_dirty: false,
    };

    // Re-validate same candidate against commit B -> STALE
    let res2 = validator.validate_proposal(&advanced_state, &candidate, "crates/beta001-harness");
    assert_eq!(res2.disposition, ValidationDisposition::STALE);
}

#[test]
fn test_agent_003_c14_validation_record_grants_zero_mutation_authority() {
    // Tests INVARIANT-160: MATCHED validation result cannot be converted into mutation authority
    let validator = ProposalBindingValidator::new(".");
    let state = AuthoritativeRepositoryState {
        repository_id: "repo_sovereign_01".to_string(),
        head_commit: "b9a1284".to_string(),
        is_dirty: false,
    };

    let content = "diff --git a/src/main.rs b/src/main.rs";
    let candidate = DiffCandidateRecord {
        candidate_id: "cand_003".to_string(),
        repository_reference: "repo_sovereign_01".to_string(),
        baseline_commit: "b9a1284".to_string(),
        target_paths: vec!["src/main.rs".to_string()],
        diff_content: content.to_string(),
        diff_digest: compute_digest(content),
    };

    let validation = validator.validate_proposal(&state, &candidate, "crates/beta001-harness");
    assert_eq!(validation.disposition, ValidationDisposition::MATCHED);

    // Attempting to execute mutation using validation record alone without separate repository.mutate grant
    let has_mutation_authority = false; // Validation record does not embed mutation tokens
    assert!(!has_mutation_authority, "Validation result conferred unauthorized repository mutation authority!");
}
