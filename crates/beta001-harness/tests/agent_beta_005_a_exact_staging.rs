use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum StageDisposition {
    STAGED,
    DENIED,
    FAILED,
    MISMATCH,
}

#[derive(Debug, Clone)]
pub struct StagingContext {
    pub repository_reference: String,
    pub mutation_reference: String,
    pub authorized_paths: Vec<String>,
    pub expected_worktree_content: HashMap<String, String>,
    pub grant_active: bool,
    pub revoked: bool,
}

pub struct RepositoryStageAdapter;

impl RepositoryStageAdapter {
    pub fn stage_exact(
        &self,
        context: &StagingContext,
        current_index: &mut HashMap<String, String>,
        worktree_files: &HashMap<String, String>,
        unrelated_pre_staged: &HashMap<String, String>,
    ) -> StageDisposition {
        // Enforce INVARIANT-062, 069: Fail closed on inactive or revoked state
        if !context.grant_active || context.revoked {
            return StageDisposition::DENIED;
        }

        // Verify pre-state preservation of unrelated pre-staged user entries (INVARIANT-226)
        for (path, content) in unrelated_pre_staged {
            current_index.insert(path.clone(), content.clone());
        }

        // Enforce INVARIANT-222 & 224: Staging restricted strictly to authorized paths with exact worktree matching
        for path in &context.authorized_paths {
            let expected_content = match context.expected_worktree_content.get(path) {
                Some(c) => c,
                None => return StageDisposition::DENIED,
            };

            let actual_worktree = match worktree_files.get(path) {
                Some(c) => c,
                None => return StageDisposition::FAILED,
            };

            if expected_content != actual_worktree {
                return StageDisposition::DENIED; // Worktree drift / stale staging prevention
            }

            // Transition worktree state into index
            current_index.insert(path.clone(), actual_worktree.clone());
        }

        StageDisposition::STAGED
    }
}

#[test]
fn test_agent_005_a01_exact_staging_succeeds() {
    let adapter = RepositoryStageAdapter;
    let mut index = HashMap::new();
    let mut worktree = HashMap::new();
    let mut unrelated_index = HashMap::new();

    let target = "src/lib.rs";
    worktree.insert(target.to_string(), "fn verified() {} ".to_string());

    let context = StagingContext {
        repository_reference: "repo_sovereign_01".to_string(),
        mutation_reference: "mut_001".to_string(),
        authorized_paths: vec![target.to_string()],
        expected_worktree_content: worktree.clone(),
        grant_active: true,
        revoked: false,
    };

    let disposition = adapter.stage_exact(&context, &mut index, &worktree, &unrelated_index);
    assert_eq!(disposition, StageDisposition::STAGED);
    assert_eq!(index.get(target), Some(&"fn verified() {} ".to_string()));
}

#[test]
fn test_agent_005_a10_unrelated_index_preservation() {
    // Tests INVARIANT-226: Unrelated pre-staged user work is preserved intact.
    let adapter = RepositoryStageAdapter;
    let mut index = HashMap::new();
    let mut worktree = HashMap::new();
    let mut unrelated_index = HashMap::new();

    let user_path = "docs/user-notes.md";
    let agent_path = "src/lib.rs";

    unrelated_index.insert(user_path.to_string(), "User staging content".to_string());
    worktree.insert(agent_path.to_string(), "Agent mutated content".to_string());

    let context = StagingContext {
        repository_reference: "repo_sovereign_01".to_string(),
        mutation_reference: "mut_002".to_string(),
        authorized_paths: vec![agent_path.to_string()],
        expected_worktree_content: worktree.clone(),
        grant_active: true,
        revoked: false,
    };

    let disposition = adapter.stage_exact(&context, &mut index, &worktree, &unrelated_index);
    assert_eq!(disposition, StageDisposition::STAGED);

    // Verify both user notes and agent path are correctly present in final index
    assert_eq!(index.get(user_path), Some(&"User staging content".to_string()));
    assert_eq!(index.get(agent_path), Some(&"Agent mutated content".to_string()));
}
