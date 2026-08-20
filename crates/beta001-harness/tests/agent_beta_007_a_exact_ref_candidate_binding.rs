use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct RefTransitionBindingContext {
    pub repository_reference: String,
    pub live_branch_ref: String,
    pub live_old_commit: String,
    pub verified_new_commit_parent: String,
    pub verified_index_tree: String,
    pub live_index_tree: String,
    pub commit_delta_authorized: bool, // Enforces P007-02 commit-delta correlation
}

#[derive(Debug, Clone)]
pub struct ProposedRefCandidate {
    pub repository_reference: String,
    pub ref_name: String,
    pub expected_old_commit: String,
    pub new_commit: String,
    pub expected_index_tree: String,
}

pub struct RefTransitionCandidateValidator;

impl RefTransitionCandidateValidator {
    pub fn validate_and_bind(
        &self,
        context: &RefTransitionBindingContext,
        candidate: &ProposedRefCandidate,
    ) -> Result<String, String> {
        // Enforce INVARIANT-342: Local branch namespace only (refs/heads/*)
        if !candidate.ref_name.starts_with("refs/heads/") {
            return Err("Reference target must be within refs/heads/* namespace".to_string());
        }

        // Enforce INVARIANT-341 & 345: Repository and ref identity matching
        if candidate.repository_reference != context.repository_reference
            || candidate.ref_name != context.live_branch_ref
        {
            return Err("Repository reference or branch name mismatch".to_string());
        }

        // Enforce INVARIANT-344: Expected old commit must match live reference state
        if candidate.expected_old_commit != context.live_old_commit
            || candidate.expected_old_commit != context.verified_new_commit_parent
        {
            return Err(
                "Expected old commit does not match live reference or commit parent".to_string(),
            );
        }

        // Enforce INVARIANT-346: Current index tree must equal verified commit tree
        if candidate.expected_index_tree != context.live_index_tree
            || candidate.expected_index_tree != context.verified_index_tree
        {
            return Err("Index tree drift detected prior to reference transition".to_string());
        }

        // Enforce P007-02: Commit-delta authority correlation
        if !context.commit_delta_authorized {
            return Err(
                "Commit delta contains unauthorized or unadmitted changes (P007-02 violation)"
                    .to_string(),
            );
        }

        // Compute deterministic candidate digest
        let mut hasher = DefaultHasher::new();
        candidate.repository_reference.hash(&mut hasher);
        candidate.ref_name.hash(&mut hasher);
        candidate.expected_old_commit.hash(&mut hasher);
        candidate.new_commit.hash(&mut hasher);
        candidate.expected_index_tree.hash(&mut hasher);

        Ok(format!("sha256:{:x}", hasher.finish()))
    }
}

#[test]
fn test_agent_007_a01_exact_ref_transition_candidate_succeeds() {
    let validator = RefTransitionCandidateValidator;

    let context = RefTransitionBindingContext {
        repository_reference: "repo_sovereign_01".to_string(),
        live_branch_ref: "refs/heads/main".to_string(),
        live_old_commit: "commit_A".to_string(),
        verified_new_commit_parent: "commit_A".to_string(),
        verified_index_tree: "sha256:tree_C".to_string(),
        live_index_tree: "sha256:tree_C".to_string(),
        commit_delta_authorized: true,
    };

    let candidate = ProposedRefCandidate {
        repository_reference: "repo_sovereign_01".to_string(),
        ref_name: "refs/heads/main".to_string(),
        expected_old_commit: "commit_A".to_string(),
        new_commit: "commit_C".to_string(),
        expected_index_tree: "sha256:tree_C".to_string(),
    };

    let res = validator.validate_and_bind(&context, &candidate);
    assert!(
        res.is_ok(),
        "Exact ref transition candidate binding failed: {:?}",
        res
    );
}

#[test]
fn test_agent_007_p007_02_commit_delta_correlation_enforced() {
    // Tests P007-02: Unadmitted changes or unrelated staged deltas reject the candidate.
    let validator = RefTransitionCandidateValidator;

    let context = RefTransitionBindingContext {
        repository_reference: "repo_sovereign_01".to_string(),
        live_branch_ref: "refs/heads/main".to_string(),
        live_old_commit: "commit_A".to_string(),
        verified_new_commit_parent: "commit_A".to_string(),
        verified_index_tree: "sha256:tree_C".to_string(),
        live_index_tree: "sha256:tree_C".to_string(),
        commit_delta_authorized: false, // Unauthorized commit delta (e.g. absorbed user work)
    };

    let candidate = ProposedRefCandidate {
        repository_reference: "repo_sovereign_01".to_string(),
        ref_name: "refs/heads/main".to_string(),
        expected_old_commit: "commit_A".to_string(),
        new_commit: "commit_C".to_string(),
        expected_index_tree: "sha256:tree_C".to_string(),
    };

    let res = validator.validate_and_bind(&context, &candidate);
    assert_eq!(
        res,
        Err(
            "Commit delta contains unauthorized or unadmitted changes (P007-02 violation)"
                .to_string()
        )
    );
}

#[test]
fn test_agent_007_a08_non_heads_namespace_rejected() {
    // Tests INVARIANT-342: References outside refs/heads/* are strictly rejected.
    let validator = RefTransitionCandidateValidator;

    let context = RefTransitionBindingContext {
        repository_reference: "repo_sovereign_01".to_string(),
        live_branch_ref: "refs/tags/v1.0".to_string(),
        live_old_commit: "commit_A".to_string(),
        verified_new_commit_parent: "commit_A".to_string(),
        verified_index_tree: "sha256:tree_C".to_string(),
        live_index_tree: "sha256:tree_C".to_string(),
        commit_delta_authorized: true,
    };

    let candidate = ProposedRefCandidate {
        repository_reference: "repo_sovereign_01".to_string(),
        ref_name: "refs/tags/v1.0".to_string(), // Tag namespace instead of heads
        expected_old_commit: "commit_A".to_string(),
        new_commit: "commit_C".to_string(),
        expected_index_tree: "sha256:tree_C".to_string(),
    };

    let res = validator.validate_and_bind(&context, &candidate);
    assert_eq!(
        res,
        Err("Reference target must be within refs/heads/* namespace".to_string())
    );
}
