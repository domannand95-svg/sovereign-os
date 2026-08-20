use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum EffectClassification {
    NoEffect,
    PartialEffect,
    FullEffectUnverified,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FaultInjectingStagingContext {
    pub authorized_paths: Vec<String>,
    pub expected_worktree_content: HashMap<String, String>,
    pub expected_pre_state: HashMap<String, String>,
    pub grant_active: bool,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StagingFaultMode {
    Normal,
    FailBeforeAnyTransition,
    FailAfterPathIndex(usize),
    ReturnUnknownState,
    PreStateMismatch,
}

pub struct FaultInjectingStageAdapter;

impl FaultInjectingStageAdapter {
    pub fn execute_fault_tolerant_stage(
        &self,
        context: &FaultInjectingStagingContext,
        current_index: &mut HashMap<String, String>,
        actual_pre_state: &HashMap<String, String>,
        worktree_files: &HashMap<String, String>,
        unrelated_pre_staged: &HashMap<String, String>,
        fault_mode: StagingFaultMode,
    ) -> EffectClassification {
        if !context.grant_active || context.revoked {
            return EffectClassification::NoEffect;
        }

        if fault_mode == StagingFaultMode::PreStateMismatch
            || actual_pre_state != &context.expected_pre_state
        {
            return EffectClassification::NoEffect;
        }

        if fault_mode == StagingFaultMode::ReturnUnknownState {
            return EffectClassification::Unknown;
        }

        if fault_mode == StagingFaultMode::FailBeforeAnyTransition {
            return EffectClassification::NoEffect;
        }

        for (path, content) in unrelated_pre_staged {
            current_index.insert(path.clone(), content.clone());
        }

        let mut transitioned_count = 0;
        let total_paths = context.authorized_paths.len();

        for (idx, path) in context.authorized_paths.iter().enumerate() {
            let worktree_content = match worktree_files.get(path) {
                Some(c) => c,
                None => continue,
            };

            current_index.insert(path.clone(), worktree_content.clone());
            transitioned_count += 1;

            if fault_mode == StagingFaultMode::FailAfterPathIndex(idx) {
                break; // Break *after* successfully writing this index entry
            }
        }

        if transitioned_count == 0 {
            EffectClassification::NoEffect
        } else if transitioned_count < total_paths {
            EffectClassification::PartialEffect
        } else {
            EffectClassification::FullEffectUnverified
        }
    }
}

#[test]
fn test_agent_005_c_partial_index_effect_classified() {
    let adapter = FaultInjectingStageAdapter;
    let mut current_index = HashMap::new();
    let mut pre_state = HashMap::new();
    let mut worktree = HashMap::new();
    let mut unrelated = HashMap::new();

    let path0 = "src/a.rs";
    let path1 = "src/b.rs";
    let user_file = "docs/user-work.md";

    pre_state.insert(user_file.to_string(), "user content".to_string());
    unrelated.insert(user_file.to_string(), "user content".to_string());

    worktree.insert(path0.to_string(), "content a".to_string());
    worktree.insert(path1.to_string(), "content b".to_string());

    let context = FaultInjectingStagingContext {
        authorized_paths: vec![path0.to_string(), path1.to_string()],
        expected_worktree_content: worktree.clone(),
        expected_pre_state: pre_state.clone(),
        grant_active: true,
        revoked: false,
    };

    let classification = adapter.execute_fault_tolerant_stage(
        &context,
        &mut current_index,
        &pre_state,
        &worktree,
        &unrelated,
        StagingFaultMode::FailAfterPathIndex(0), // Fails after writing path 0 (partial)
    );

    assert_eq!(classification, EffectClassification::PartialEffect);
    assert_eq!(current_index.get(path0), Some(&"content a".to_string()));
    assert_eq!(current_index.get(path1), None); // Not transitioned
    assert_eq!(
        current_index.get(user_file),
        Some(&"user content".to_string())
    );
}

#[test]
fn test_agent_005_c_unknown_state_contains_pipeline() {
    let adapter = FaultInjectingStageAdapter;
    let mut current_index = HashMap::new();
    let pre_state = HashMap::new();
    let worktree = HashMap::new();
    let unrelated = HashMap::new();

    let context = FaultInjectingStagingContext {
        authorized_paths: vec!["src/lib.rs".to_string()],
        expected_worktree_content: worktree.clone(),
        expected_pre_state: pre_state.clone(),
        grant_active: true,
        revoked: false,
    };

    let classification = adapter.execute_fault_tolerant_stage(
        &context,
        &mut current_index,
        &pre_state,
        &worktree,
        &unrelated,
        StagingFaultMode::ReturnUnknownState,
    );

    assert_eq!(classification, EffectClassification::Unknown);
}
