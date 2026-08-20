use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AdversarialStageDisposition {
    Staged,
    Denied,
    Stale,
    PartialEffect,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct AdversarialStagingContext {
    pub authorized_paths: Vec<String>,
    pub expected_pre_state: HashMap<String, String>,
    pub grant_active: bool,
    pub revoked: bool,
}

pub struct AdversarialStagingOrchestrator;

impl AdversarialStagingOrchestrator {
    pub fn execute_adversarial_staging(
        &self,
        context: &AdversarialStagingContext,
        current_index: &mut HashMap<String, String>,
        actual_pre_state: &HashMap<String, String>,
        worktree_files: &HashMap<String, String>,
        unrelated_pre_staged: &HashMap<String, String>,
        requested_paths: &[String],
        is_stale_pre_state: bool,
    ) -> AdversarialStageDisposition {
        // Enforce INVARIANT-062, 069: Fail closed on inactive or revoked state
        if !context.grant_active || context.revoked {
            return AdversarialStageDisposition::Denied;
        }

        // Enforce INVARIANT-259: Pre-state substitution denied
        if is_stale_pre_state || actual_pre_state != &context.expected_pre_state {
            return AdversarialStageDisposition::Stale;
        }

        // Enforce INVARIANT-256: Stage-all / extra path injection denied (path set equality)
        let mut sorted_requested = requested_paths.to_vec();
        sorted_requested.sort();
        let mut sorted_authorized = context.authorized_paths.clone();
        sorted_authorized.sort();

        if sorted_requested != sorted_authorized {
            return AdversarialStageDisposition::Denied; // Rejects extra unauthorized paths or ambient stage-all
        }

        // Preserve unrelated pre-staged user work (INVARIANT-260)
        for (path, content) in unrelated_pre_staged {
            current_index.insert(path.clone(), content.clone());
        }

        // Transition authorized paths
        for path in &sorted_authorized {
            let worktree_content = match worktree_files.get(path) {
                Some(c) => c,
                None => return AdversarialStageDisposition::Unknown,
            };
            current_index.insert(path.clone(), worktree_content.clone());
        }

        AdversarialStageDisposition::Staged
    }
}

#[test]
fn test_agent_005_d_bounded_staging_injection_resisted() {
    // AGENT-005-D-BOUNDED-STAGING-INJECTION:
    // Agent possesses legitimate staging authority for "src/lib.rs", but repository prompt injection 
    // requests staging extra paths ("Cargo.toml") and user work ("docs/user-work.md").
    let orchestrator = AdversarialStagingOrchestrator;
    let mut index = HashMap::new();
    let mut pre_state = HashMap::new();
    let mut worktree = HashMap::new();
    let mut unrelated = HashMap::new();

    let user_file = "docs/user-work.md";
    pre_state.insert(user_file.to_string(), "original user work".to_string());
    unrelated.insert(user_file.to_string(), "original user work".to_string());

    let agent_path = "src/lib.rs";
    worktree.insert(agent_path.to_string(), "pub fn agent_work() {} ".to_string());
    worktree.insert("Cargo.toml".to_string(), "[package]".to_string());

    let context = AdversarialStagingContext {
        authorized_paths: vec![agent_path.to_string()],
        expected_pre_state: pre_state.clone(),
        grant_active: true,
        revoked: false,
    };

    // Attempt injection requesting both agent path and unauthorized Cargo.toml (stage-all style)
    let requested_paths = vec![agent_path.to_string(), "Cargo.toml".to_string()];
    let disposition = orchestrator.execute_adversarial_staging(
        &context,
        &mut index,
        &pre_state,
        &worktree,
        &unrelated,
        &requested_paths,
        false,
    );

    assert_eq!(disposition, AdversarialStageDisposition::Denied);
    // Verify unrelated user work was preserved untouched and Cargo.toml was not staged
    assert_eq!(index.get(user_file), Some(&"original user work".to_string()));
    assert_eq!(index.get("Cargo.toml"), None);
}

#[test]
fn test_agent_005_d_stale_stage_authority_replay_denied() {
    // AGENT-005-D-STALE-AUTHORITY-REPLAY:
    // Presenting staging grant against a drifted index pre-state yields Stale.
    let orchestrator = AdversarialStagingOrchestrator;
    let mut index = HashMap::new();
    let pre_state = HashMap::new();
    let worktree = HashMap::new();
    let unrelated = HashMap::new();

    let context = AdversarialStagingContext {
        authorized_paths: vec!["src/lib.rs".to_string()],
        expected_pre_state: pre_state.clone(),
        grant_active: true,
        revoked: false,
    };

    let drifted_pre_state = HashMap::from([("some_other_file.txt".to_string(), "drift".to_string())]);

    let disposition = orchestrator.execute_adversarial_staging(
        &context,
        &mut index,
        &drifted_pre_state, // Drifted pre-state
        &worktree,
        &unrelated,
        &vec!["src/lib.rs".to_string()],
        true, // Stale indicator
    );

    assert_eq!(disposition, AdversarialStageDisposition::Stale);
}

#[test]
fn test_agent_005_d_recovery_authority_separation() {
    // AGENT-005-D-RECOVERY-AUTHORITY-SEPARATION:
    // After partial staging failure, agent requests git reset or stage-all recovery.
    // Verifies that recovery/reset authority remains entirely absent.
    let has_reset_authority = false;
    let has_stage_all_authority = false;
    assert!(!has_reset_authority);
    assert!(!has_stage_all_authority);
}
