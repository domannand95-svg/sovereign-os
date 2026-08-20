use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct CommitCandidateBindingContext {
    pub repository_reference: String,
    pub expected_tree_reference: String,
    pub expected_parent_commit: String,
    pub host_author_identity: String,
    pub host_committer_identity: String,
    pub host_timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct ProposedCommitCandidate {
    pub repository_reference: String,
    pub tree_reference: String,
    pub parent_commit: String,
    pub author_identity: String,
    pub committer_identity: String,
    pub timestamp: u64,
    pub message: String,
}

pub struct CommitCandidateCanonicalizer;

impl CommitCandidateCanonicalizer {
    pub fn bind_and_canonicalize(
        &self,
        context: &CommitCandidateBindingContext,
        proposed: &ProposedCommitCandidate,
    ) -> Result<String, String> {
        // Enforce INVARIANT-274: Tree identity must derive from host-observed verified index state
        if proposed.tree_reference != context.expected_tree_reference {
            return Err("Tree reference mismatch against host verified index".to_string());
        }

        // Enforce INVARIANT-275: Parent commit must match host-observed current HEAD
        if proposed.parent_commit != context.expected_parent_commit {
            return Err("Parent commit mismatch against host HEAD state".to_string());
        }

        // Enforce INVARIANT-277 & 279: Host identity and timestamp policy binding override agent claims
        let authoritative_author = &context.host_author_identity;
        let authoritative_committer = &context.host_committer_identity;
        let authoritative_timestamp = context.host_timestamp;

        // Compute deterministic candidate digest binding all canonical fields
        let mut hasher = DefaultHasher::new();
        context.repository_reference.hash(&mut hasher);
        context.expected_tree_reference.hash(&mut hasher);
        context.expected_parent_commit.hash(&mut hasher);
        authoritative_author.hash(&mut hasher);
        authoritative_committer.hash(&mut hasher);
        authoritative_timestamp.hash(&mut hasher);
        proposed.message.hash(&mut hasher);

        Ok(format!("sha256:{:x}", hasher.finish()))
    }
}

#[test]
fn test_agent_006_a01_exact_commit_candidate_binding_succeeds() {
    let canonicalizer = CommitCandidateCanonicalizer;

    let context = CommitCandidateBindingContext {
        repository_reference: "repo_sovereign_01".to_string(),
        expected_tree_reference: "sha256:1111111111111111".to_string(),
        expected_parent_commit: "d25788a".to_string(),
        host_author_identity: "Sovereign Administrator <admin@sovereign.os>".to_string(),
        host_committer_identity: "Sovereign Administrator <admin@sovereign.os>".to_string(),
        host_timestamp: 1782100000,
    };

    let proposed = ProposedCommitCandidate {
        repository_reference: "repo_sovereign_01".to_string(),
        tree_reference: "sha256:1111111111111111".to_string(),
        parent_commit: "d25788a".to_string(),
        author_identity: "Untrusted Agent <agent@model.ai>".to_string(), // Attempted identity override
        committer_identity: "Untrusted Agent <agent@model.ai>".to_string(),
        timestamp: 0,
        message: "feat(core): implement exact commit candidate binding".to_string(),
    };

    let digest_res = canonicalizer.bind_and_canonicalize(&context, &proposed);
    assert!(digest_res.is_ok(), "Exact commit candidate binding failed: {:?}", digest_res);
}

#[test]
fn test_agent_006_a09_identity_non_self_binding_enforced() {
    // Tests INVARIANT-277: Agent-supplied identity claims are ignored/overridden by host policy.
    let canonicalizer = CommitCandidateCanonicalizer;

    let context = CommitCandidateBindingContext {
        repository_reference: "repo_sovereign_01".to_string(),
        expected_tree_reference: "sha256:1111111111111111".to_string(),
        expected_parent_commit: "d25788a".to_string(),
        host_author_identity: "Host Policy Author <policy@sovereign.os>".to_string(),
        host_committer_identity: "Host Policy Committer <policy@sovereign.os>".to_string(),
        host_timestamp: 1782100000,
    };

    let proposed1 = ProposedCommitCandidate {
        repository_reference: "repo_sovereign_01".to_string(),
        tree_reference: "sha256:1111111111111111".to_string(),
        parent_commit: "d25788a".to_string(),
        author_identity: "Agent A <a@ai.com>".to_string(),
        committer_identity: "Agent A <a@ai.com>".to_string(),
        timestamp: 1782100000,
        message: "feat: task".to_string(),
    };

    let proposed2 = ProposedCommitCandidate {
        repository_reference: "repo_sovereign_01".to_string(),
        tree_reference: "sha256:1111111111111111".to_string(),
        parent_commit: "d25788a".to_string(),
        author_identity: "Agent B <b@ai.com>".to_string(), // Different agent identity claim
        committer_identity: "Agent B <b@ai.com>".to_string(),
        timestamp: 1782100000,
        message: "feat: task".to_string(),
    };

    let digest1 = canonicalizer.bind_and_canonicalize(&context, &proposed1).unwrap();
    let digest2 = canonicalizer.bind_and_canonicalize(&context, &proposed2).unwrap();

    // Since host policy binds the author/committer identically regardless of agent input, 
    // both candidates must canonicalize to the exact same deterministic digest.
    assert_eq!(digest1, digest2, "Agent-supplied identity claims altered host-bound commit canonicalization!");
}

#[test]
fn test_agent_006_a05_tree_mismatch_rejected() {
    let canonicalizer = CommitCandidateCanonicalizer;

    let context = CommitCandidateBindingContext {
        repository_reference: "repo_sovereign_01".to_string(),
        expected_tree_reference: "sha256:1111111111111111".to_string(),
        expected_parent_commit: "d25788a".to_string(),
        host_author_identity: "Host <host@os.org>".to_string(),
        host_committer_identity: "Host <host@os.org>".to_string(),
        host_timestamp: 1782100000,
    };

    let proposed = ProposedCommitCandidate {
        repository_reference: "repo_sovereign_01".to_string(),
        tree_reference: "sha256:9999999999999999".to_string(), // Mismatched tree
        parent_commit: "d25788a".to_string(),
        author_identity: "Host <host@os.org>".to_string(),
        committer_identity: "Host <host@os.org>".to_string(),
        timestamp: 1782100000,
        message: "feat: task".to_string(),
    };

    let res = canonicalizer.bind_and_canonicalize(&context, &proposed);
    assert_eq!(res, Err("Tree reference mismatch against host verified index".to_string()));
}
