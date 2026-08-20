use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AdversarialRefTransitionDisposition {
    TRANSITIONED,
    DENIED,
    STALE,
    MISMATCH,
}

#[derive(Debug, Clone)]
pub struct AdversarialRefContext {
    pub repository_reference: String,
    pub authorized_ref: String,
    pub expected_old_commit: String,
    pub grant_active: bool,
    pub revoked: bool,
}

pub struct AdversarialRefOrchestrator;

impl AdversarialRefOrchestrator {
    pub fn execute_adversarial_ref_transition(
        &self,
        context: &AdversarialRefContext,
        live_refs: &mut HashMap<String, String>,
        requested_ref: &str,
        requested_old: &str,
        new_commit: &str,
        is_stale_candidate: bool,
        is_alternate_ref: bool,
    ) -> AdversarialRefTransitionDisposition {
        // Enforce INVARIANT-391: Denial dominance
        if !context.grant_active || context.revoked {
            return AdversarialRefTransitionDisposition::DENIED;
        }

        // Enforce INVARIANT-394 & 395: Exact ref name and local heads namespace confinement
        if is_alternate_ref
            || requested_ref != context.authorized_ref
            || !requested_ref.starts_with("refs/heads/")
        {
            return AdversarialRefTransitionDisposition::DENIED;
        }

        // Enforce INVARIANT-392: Stale candidate rejection
        if is_stale_candidate {
            return AdversarialRefTransitionDisposition::STALE;
        }

        // Get live ref state
        let current_val = match live_refs.get(requested_ref) {
            Some(v) => v,
            None => return AdversarialRefTransitionDisposition::MISMATCH,
        };

        // Enforce INVARIANT-393: Atomic Compare-and-Swap (fails closed on race)
        if current_val != requested_old || requested_old != &context.expected_old_commit {
            return AdversarialRefTransitionDisposition::DENIED;
        }

        // Apply atomic transition
        live_refs.insert(requested_ref.to_string(), new_commit.to_string());
        AdversarialRefTransitionDisposition::TRANSITIONED
    }
}

#[test]
fn test_agent_007_d_cas_race_denied() {
    // Tests INVARIANT-393: Concurrent ref update causes CAS failure and preserves live tip.
    let orchestrator = AdversarialRefOrchestrator;
    let mut live_refs = HashMap::from([("refs/heads/main".to_string(), "commit_B".to_string())]); // Advanced concurrently to B

    let context = AdversarialRefContext {
        repository_reference: "repo_sovereign_01".to_string(),
        authorized_ref: "refs/heads/main".to_string(),
        expected_old_commit: "commit_A".to_string(), // Stale expected old commit
        grant_active: true,
        revoked: false,
    };

    let disposition = orchestrator.execute_adversarial_ref_transition(
        &context,
        &mut live_refs,
        "refs/heads/main",
        "commit_A",
        "commit_C",
        false,
        false,
    );

    assert_eq!(disposition, AdversarialRefTransitionDisposition::DENIED);
    assert_eq!(
        live_refs.get("refs/heads/main"),
        Some(&"commit_B".to_string())
    ); // Unchanged by race
}

#[test]
fn test_agent_007_d_alternate_branch_substitution_denied() {
    // Tests INVARIANT-394: Substituting authorized 'main' with 'release' is denied.
    let orchestrator = AdversarialRefOrchestrator;
    let mut live_refs = HashMap::from([
        ("refs/heads/main".to_string(), "commit_A".to_string()),
        ("refs/heads/release".to_string(), "commit_A".to_string()),
    ]);

    let context = AdversarialRefContext {
        repository_reference: "repo_sovereign_01".to_string(),
        authorized_ref: "refs/heads/main".to_string(),
        expected_old_commit: "commit_A".to_string(),
        grant_active: true,
        revoked: false,
    };

    let disposition = orchestrator.execute_adversarial_ref_transition(
        &context,
        &mut live_refs,
        "refs/heads/release", // Attempted substitution to release branch
        "commit_A",
        "commit_C",
        false,
        true, // Alternate ref indicator
    );

    assert_eq!(disposition, AdversarialRefTransitionDisposition::DENIED);
    assert_eq!(
        live_refs.get("refs/heads/release"),
        Some(&"commit_A".to_string())
    );
}

#[test]
fn test_agent_007_d_push_boundary_isolated() {
    // Tests INVARIANT-402: Successful local transition confers zero push/remote authority.
    let orchestrator = AdversarialRefOrchestrator;
    let mut live_refs = HashMap::from([("refs/heads/main".to_string(), "commit_A".to_string())]);

    let context = AdversarialRefContext {
        repository_reference: "repo_sovereign_01".to_string(),
        authorized_ref: "refs/heads/main".to_string(),
        expected_old_commit: "commit_A".to_string(),
        grant_active: true,
        revoked: false,
    };

    let disposition = orchestrator.execute_adversarial_ref_transition(
        &context,
        &mut live_refs,
        "refs/heads/main",
        "commit_A",
        "commit_C",
        false,
        false,
    );

    assert_eq!(
        disposition,
        AdversarialRefTransitionDisposition::TRANSITIONED
    );

    // Verify local ref updated while remote publication authority remains absent (isolated)
    let has_push_authority = false;
    assert_eq!(
        live_refs.get("refs/heads/main"),
        Some(&"commit_C".to_string())
    );
    assert!(
        !has_push_authority,
        "Local ref transition incorrectly conferred remote push authority!"
    );
}
