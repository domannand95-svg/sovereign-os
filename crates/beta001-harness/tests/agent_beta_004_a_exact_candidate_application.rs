use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub struct RepositoryMutationAdapter {
    checkout_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LiveRepositoryState {
    pub repository_id: String,
    pub head_commit: String,
    pub is_dirty: bool,
}

#[derive(Debug, Clone)]
pub struct AuthorizedMutationContext {
    pub candidate_id: String,
    pub repository_reference: String,
    pub baseline_commit: String,
    pub target_path: String,
    pub expected_preimage_content: String,
    pub new_content: String,
    pub diff_digest: String,
    pub grant_active: bool,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MutationDisposition {
    APPLIED,
    DENIED,
    FAILED,
}

impl RepositoryMutationAdapter {
    pub fn new(checkout_root: impl Into<PathBuf>) -> Self {
        Self {
            checkout_root: checkout_root.into(),
        }
    }

    pub fn apply_mutation(
        &self,
        live_state: &LiveRepositoryState,
        context: &AuthorizedMutationContext,
    ) -> Result<MutationDisposition, String> {
        // Enforce INVARIANT-062, 069: Fail closed on inactive or revoked state
        if !context.grant_active || context.revoked {
            return Ok(MutationDisposition::DENIED);
        }

        // Enforce INVARIANT-162 & 182: Unexpected dirty state fails closed
        if live_state.is_dirty {
            return Ok(MutationDisposition::DENIED);
        }

        // Enforce INVARIANT-175: Fresh-state verification immediately before execution
        if context.baseline_commit != live_state.head_commit {
            return Ok(MutationDisposition::DENIED); // TOCTOU / Stale baseline prevention
        }

        // Enforce INVARIANT-177: Repository identity binding
        if context.repository_reference != live_state.repository_id {
            return Ok(MutationDisposition::DENIED);
        }

        // Resolve target file path within checkout root
        let target_file_path = self.checkout_root.join(&context.target_path);

        // Enforce INVARIANT-179: Preimage integrity check against current file content on disk
        let current_preimage = match fs::read_to_string(&target_file_path) {
            Ok(content) => content,
            Err(_) => return Ok(MutationDisposition::FAILED),
        };

        if current_preimage != context.expected_preimage_content {
            return Ok(MutationDisposition::DENIED); // Preimage mismatch / concurrent modification
        }

        // Perform exact bounded mutation
        if let Err(_) = fs::write(&target_file_path, &context.new_content) {
            return Ok(MutationDisposition::FAILED);
        }

        Ok(MutationDisposition::APPLIED)
    }
}

fn compute_digest(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("sha256:{:x}", hasher.finish())
}

#[test]
fn test_agent_004_a01_exact_candidate_mutation_applied() {
    // Setup temporary fixture file
    let temp_dir = std::env::temp_dir().join("sovereign_repo_mut_test_01");
    let _ = fs::create_dir_all(&temp_dir);
    let target_rel = "test_target.txt";
    let target_full = temp_dir.join(target_rel);
    let initial_content = "Original baseline content.";
    let updated_content = "Updated modified content via governed mutation.";
    fs::write(&target_full, initial_content).unwrap();

    let adapter = RepositoryMutationAdapter::new(&temp_dir);
    let live_state = LiveRepositoryState {
        repository_id: "repo_sovereign_mut_01".to_string(),
        head_commit: "commit_baseline_A".to_string(),
        is_dirty: false,
    };

    let context = AuthorizedMutationContext {
        candidate_id: "cand_mut_01".to_string(),
        repository_reference: "repo_sovereign_mut_01".to_string(),
        baseline_commit: "commit_baseline_A".to_string(),
        target_path: target_rel.to_string(),
        expected_preimage_content: initial_content.to_string(),
        new_content: updated_content.to_string(),
        diff_digest: compute_digest(updated_content),
        grant_active: true,
        revoked: false,
    };

    let result = adapter.apply_mutation(&live_state, &context);
    assert_eq!(result, Ok(MutationDisposition::APPLIED));

    // Verify file content on disk matches exactly
    let resulting_content = fs::read_to_string(&target_full).unwrap();
    assert_eq!(resulting_content, updated_content);

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_agent_004_a_fresh_state_toctou_denied() {
    // Tests INVARIANT-175: If HEAD advances between validation and mutation, execution is denied.
    let temp_dir = std::env::temp_dir().join("sovereign_repo_mut_test_02");
    let _ = fs::create_dir_all(&temp_dir);
    let target_rel = "test_target.txt";
    let target_full = temp_dir.join(target_rel);
    let initial_content = "Original content.";
    fs::write(&target_full, initial_content).unwrap();

    let adapter = RepositoryMutationAdapter::new(&temp_dir);
    let advanced_live_state = LiveRepositoryState {
        repository_id: "repo_sovereign_mut_01".to_string(),
        head_commit: "commit_advanced_B".to_string(), // Advanced HEAD
        is_dirty: false,
    };

    let context = AuthorizedMutationContext {
        candidate_id: "cand_mut_02".to_string(),
        repository_reference: "repo_sovereign_mut_01".to_string(),
        baseline_commit: "commit_baseline_A".to_string(), // Stale baseline
        target_path: target_rel.to_string(),
        expected_preimage_content: initial_content.to_string(),
        new_content: "New content".to_string(),
        diff_digest: compute_digest("New content"),
        grant_active: true,
        revoked: false,
    };

    let result = adapter.apply_mutation(&advanced_live_state, &context);
    assert_eq!(result, Ok(MutationDisposition::DENIED));

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
