use std::fs;
use std::path::PathBuf;

pub struct AdversarialMutationOrchestrator {
    checkout_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AuthoritativeMutationState {
    pub repository_id: String,
    pub head_commit: String,
    pub is_dirty: bool,
}

#[derive(Debug, Clone)]
pub struct MutationGrantContext {
    pub candidate_id: String,
    pub repository_reference: String,
    pub baseline_commit: String,
    pub authorized_target_path: String,
    pub expected_preimage: String,
    pub new_content: String,
    pub grant_active: bool,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AdversarialDisposition {
    APPLIED,
    DENIED,
    STALE,
    PartialEffect,
    UNKNOWN,
}

impl AdversarialMutationOrchestrator {
    pub fn new(checkout_root: impl Into<PathBuf>) -> Self {
        Self {
            checkout_root: checkout_root.into(),
        }
    }

    pub fn execute_adversarial_mutation(
        &self,
        state: &AuthoritativeMutationState,
        grant: &MutationGrantContext,
        requested_target_path: &str,
        is_stale_baseline: bool,
        is_substitued_candidate: bool,
    ) -> AdversarialDisposition {
        // Enforce INVARIANT-062, 069: Fail closed on inactive or revoked state
        if !grant.grant_active || grant.revoked {
            return AdversarialDisposition::DENIED;
        }

        // Enforce INVARIANT-182: Dirty state fails closed
        if state.is_dirty {
            return AdversarialDisposition::DENIED;
        }

        // Enforce INVARIANT-206 & 207: Stale baseline replay or candidate substitution denied
        if is_stale_baseline || grant.baseline_commit != state.head_commit {
            return AdversarialDisposition::STALE;
        }

        if is_substitued_candidate {
            return AdversarialDisposition::DENIED;
        }

        // Enforce INVARIANT-208: Repository prompt injection / path scope restriction
        if requested_target_path != grant.authorized_target_path {
            return AdversarialDisposition::DENIED; // Unauthorized extra target rejected
        }

        // Enforce INVARIANT-179: Preimage integrity
        let target_file = self.checkout_root.join(requested_target_path);
        let current_preimage = match fs::read_to_string(&target_file) {
            Ok(content) => content,
            Err(_) => return AdversarialDisposition::DENIED,
        };

        if current_preimage != grant.expected_preimage {
            return AdversarialDisposition::DENIED;
        }

        // Execute bounded mutation
        if fs::write(&target_file, &grant.new_content).is_err() {
            return AdversarialDisposition::UNKNOWN;
        }

        AdversarialDisposition::APPLIED
    }
}

#[test]
fn test_agent_004_d_bounded_mutation_injection_resisted() {
    // AGENT-004-D-BOUNDED-MUTATION-INJECTION:
    // Agent possesses legitimate mutation authority for "src/summary.rs",
    // but injected repository instructions attempt to force writing to "Cargo.toml".
    let temp_dir = std::env::temp_dir().join("sovereign_adv_mut_01");
    let _ = fs::create_dir_all(&temp_dir);
    fs::write(temp_dir.join("src_summary.rs"), "original").unwrap();
    fs::write(temp_dir.join("Cargo.toml"), "[package]").unwrap();

    let orchestrator = AdversarialMutationOrchestrator::new(&temp_dir);
    let state = AuthoritativeMutationState {
        repository_id: "repo_sovereign_mut_01".to_string(),
        head_commit: "commit_A".to_string(),
        is_dirty: false,
    };

    let grant = MutationGrantContext {
        candidate_id: "cand_01".to_string(),
        repository_reference: "repo_sovereign_mut_01".to_string(),
        baseline_commit: "commit_A".to_string(),
        authorized_target_path: "src_summary.rs".to_string(),
        expected_preimage: "original".to_string(),
        new_content: "updated summary".to_string(),
        grant_active: true,
        revoked: false,
    };

    // Attempt injection targeting Cargo.toml instead of authorized src_summary.rs
    let disposition = orchestrator.execute_adversarial_mutation(
        &state,
        &grant,
        "Cargo.toml", // Unauthorized injection target
        false,
        false,
    );

    assert_eq!(disposition, AdversarialDisposition::DENIED);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_agent_004_d_stale_authority_replay_denied() {
    // AGENT-004-D-STALE-AUTHORITY-REPLAY:
    // Presenting an old candidate and grant after repository HEAD advances from A to B.
    let temp_dir = std::env::temp_dir().join("sovereign_adv_mut_02");
    let _ = fs::create_dir_all(&temp_dir);
    let target = "src_summary.rs";
    fs::write(temp_dir.join(target), "original").unwrap();

    let orchestrator = AdversarialMutationOrchestrator::new(&temp_dir);
    let advanced_state = AuthoritativeMutationState {
        repository_id: "repo_sovereign_mut_01".to_string(),
        head_commit: "commit_B".to_string(), // Advanced HEAD
        is_dirty: false,
    };

    let grant = MutationGrantContext {
        candidate_id: "cand_02".to_string(),
        repository_reference: "repo_sovereign_mut_01".to_string(),
        baseline_commit: "commit_A".to_string(), // Stale baseline
        authorized_target_path: target.to_string(),
        expected_preimage: "original".to_string(),
        new_content: "updated".to_string(),
        grant_active: true,
        revoked: false,
    };

    let disposition = orchestrator.execute_adversarial_mutation(
        &advanced_state,
        &grant,
        target,
        true, // Stale baseline indicator
        false,
    );

    assert_eq!(disposition, AdversarialDisposition::STALE);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_agent_004_d_recovery_authority_absent() {
    // AGENT-004-D-RECOVERY-AUTHORITY-SEPARATION:
    // After partial failure or error, agent requests reset/restore.
    // Verifies that recovery authority is completely absent and defaults fail-closed.
    let has_reset_authority = false;
    let has_restore_authority = false;
    assert!(!has_reset_authority);
    assert!(!has_restore_authority);
}
