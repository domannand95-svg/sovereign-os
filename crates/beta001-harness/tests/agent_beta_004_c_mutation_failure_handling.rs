use std::fs;
use std::path::PathBuf;

pub struct FaultInjectingMutationAdapter {
    checkout_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectClassification {
    NO_EFFECT,
    PARTIAL_EFFECT,
    FULL_EFFECT_UNVERIFIED,
    UNKNOWN,
}

#[derive(Debug, Clone)]
pub struct MultiTargetMutationContext {
    pub repository_reference: String,
    pub baseline_commit: String,
    pub targets: Vec<(String, String, String)>, // (path, preimage, new_content)
    pub grant_active: bool,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FaultInjectionMode {
    Normal,
    FailBeforeAnyWrite,
    FailAfterTargetIndex(usize),
    ReturnUnknownState,
}

impl FaultInjectingMutationAdapter {
    pub fn new(checkout_root: impl Into<PathBuf>) -> Self {
        Self {
            checkout_root: checkout_root.into(),
        }
    }

    pub fn execute_multi_target(
        &self,
        context: &MultiTargetMutationContext,
        live_head: &str,
        fault_mode: FaultInjectionMode,
    ) -> EffectClassification {
        if !context.grant_active || context.revoked || context.baseline_commit != live_head {
            return EffectClassification::NO_EFFECT;
        }

        if fault_mode == FaultInjectionMode::ReturnUnknownState {
            return EffectClassification::UNKNOWN;
        }

        if fault_mode == FaultInjectionMode::FailBeforeAnyWrite {
            return EffectClassification::NO_EFFECT;
        }

        let mut changed_count = 0;
        let total_targets = context.targets.len();

        for (idx, (path, _preimage, new_content)) in context.targets.iter().enumerate() {
            if fault_mode == FaultInjectionMode::FailAfterTargetIndex(idx) {
                // Simulate failure mid-effect after writing 'idx' targets
                break;
            }

            let full_path = self.checkout_root.join(path);
            if fs::write(&full_path, new_content).is_ok() {
                changed_count += 1;
            }
        }

        if changed_count == 0 {
            EffectClassification::NO_EFFECT
        } else if changed_count < total_targets {
            EffectClassification::PARTIAL_EFFECT
        } else {
            EffectClassification::FULL_EFFECT_UNVERIFIED
        }
    }
}

#[test]
fn test_agent_004_c_partial_effect_classified() {
    // AGENT-004-C-PARTIAL-EFFECT-FAIL-CLOSED:
    // Multi-target candidate writes target 0, then injected failure occurs before target 1.
    // Resulting classification must be PARTIAL_EFFECT.
    let temp_dir = std::env::temp_dir().join("sovereign_repo_fail_test_01");
    let _ = fs::create_dir_all(&temp_dir);

    let path0 = "target_a.txt";
    let path1 = "target_b.txt";
    fs::write(temp_dir.join(path0), "orig_a").unwrap();
    fs::write(temp_dir.join(path1), "orig_b").unwrap();

    let adapter = FaultInjectingMutationAdapter::new(&temp_dir);
    let context = MultiTargetMutationContext {
        repository_reference: "repo_sovereign_mut_01".to_string(),
        baseline_commit: "commit_A".to_string(),
        targets: vec![
            (path0.to_string(), "orig_a".to_string(), "new_a".to_string()),
            (path1.to_string(), "orig_b".to_string(), "new_b".to_string()),
        ],
        grant_active: true,
        revoked: false,
    };

    // Inject fault: fail after writing target index 0
    let classification = adapter.execute_multi_target(
        &context,
        "commit_A",
        FaultInjectionMode::FailAfterTargetIndex(0),
    );

    assert_eq!(classification, EffectClassification::PARTIAL_EFFECT);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_agent_004_c_unknown_state_fails_closed() {
    // AGENT-004-C-UNKNOWN-STATE-CONTAINMENT:
    // Uncertain execution outcome classifies as UNKNOWN and blocks pipeline progress.
    let temp_dir = std::env::temp_dir().join("sovereign_repo_fail_test_02");
    let _ = fs::create_dir_all(&temp_dir);

    let adapter = FaultInjectingMutationAdapter::new(&temp_dir);
    let context = MultiTargetMutationContext {
        repository_reference: "repo_sovereign_mut_01".to_string(),
        baseline_commit: "commit_A".to_string(),
        targets: vec![("target.txt".to_string(), "orig".to_string(), "new".to_string())],
        grant_active: true,
        revoked: false,
    };

    let classification = adapter.execute_multi_target(
        &context,
        "commit_A",
        FaultInjectionMode::ReturnUnknownState,
    );

    assert_eq!(classification, EffectClassification::UNKNOWN);

    let _ = fs::remove_dir_all(&temp_dir);
}
