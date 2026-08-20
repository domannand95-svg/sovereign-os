use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq)]
pub enum CommitVerificationDisposition {
    VERIFIED,
    MISMATCH,
    INCOMPLETE,
    UNKNOWN,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectEffectClassification {
    NoEffect,
    PartialEffect,
    FullEffectUnverified,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CommitVerificationContext {
    pub expected_tree: String,
    pub expected_parent: String,
    pub expected_author: String,
    pub expected_committer: String,
    pub expected_timestamp: u64,
    pub expected_message: String,
    pub expected_commit_id: String,
}

#[derive(Debug, Clone)]
pub struct ParsedCommitObject {
    pub tree: String,
    pub parent: String,
    pub author: String,
    pub committer: String,
    pub timestamp: u64,
    pub message: String,
    pub commit_id: String,
}

pub struct CommitVerifier;

impl CommitVerifier {
    pub fn verify_commit(
        &self,
        context: &CommitVerificationContext,
        observed_object: Option<&ParsedCommitObject>,
    ) -> CommitVerificationDisposition {
        let obj = match observed_object {
            Some(o) => o,
            None => return CommitVerificationDisposition::INCOMPLETE,
        };

        // Enforce INVARIANT-306: Exact commit object ID equality
        if obj.commit_id != context.expected_commit_id {
            return CommitVerificationDisposition::MISMATCH;
        }

        // Enforce INVARIANT-308, 309, 310, 311, 312: Exact parsed field correlation
        if obj.tree != context.expected_tree
            || obj.parent != context.expected_parent
            || obj.author != context.expected_author
            || obj.committer != context.expected_committer
            || obj.timestamp != context.expected_timestamp
            || obj.message != context.expected_message
        {
            return CommitVerificationDisposition::MISMATCH;
        }

        CommitVerificationDisposition::VERIFIED
    }

    pub fn classify_faulty_construction(
        &self,
        tree_written: bool,
        commit_written: bool,
        is_unknown: bool,
    ) -> ObjectEffectClassification {
        if is_unknown {
            return ObjectEffectClassification::Unknown;
        }
        if !tree_written && !commit_written {
            return ObjectEffectClassification::NoEffect;
        }
        if tree_written && !commit_written {
            return ObjectEffectClassification::PartialEffect;
        }
        ObjectEffectClassification::FullEffectUnverified
    }
}

fn compute_commit_id(
    tree: &str,
    parent: &str,
    author: &str,
    committer: &str,
    timestamp: u64,
    message: &str,
) -> String {
    let mut hasher = DefaultHasher::new();
    tree.hash(&mut hasher);
    parent.hash(&mut hasher);
    author.hash(&mut hasher);
    committer.hash(&mut hasher);
    timestamp.hash(&mut hasher);
    message.hash(&mut hasher);
    format!("sha256:{:x}", hasher.finish())
}

#[test]
fn test_agent_006_c01_exact_commit_object_verified() {
    let verifier = CommitVerifier;

    let tree = "sha256:abc123tree";
    let parent = "d25788a";
    let author = "Sovereign Host <host@sovereign.os>";
    let committer = "Sovereign Host <host@sovereign.os>";
    let timestamp = 1782100000;
    let message = "feat: verified commit object";
    let commit_id = compute_commit_id(tree, parent, author, committer, timestamp, message);

    let context = CommitVerificationContext {
        expected_tree: tree.to_string(),
        expected_parent: parent.to_string(),
        expected_author: author.to_string(),
        expected_committer: committer.to_string(),
        expected_timestamp: timestamp,
        expected_message: message.to_string(),
        expected_commit_id: commit_id.clone(),
    };

    let observed = ParsedCommitObject {
        tree: tree.to_string(),
        parent: parent.to_string(),
        author: author.to_string(),
        committer: committer.to_string(),
        timestamp,
        message: message.to_string(),
        commit_id,
    };

    let disposition = verifier.verify_commit(&context, Some(&observed));
    assert_eq!(disposition, CommitVerificationDisposition::VERIFIED);
}

#[test]
fn test_agent_006_c03_object_id_mismatch_detected() {
    let verifier = CommitVerifier;

    let context = CommitVerificationContext {
        expected_tree: "sha256:tree1".to_string(),
        expected_parent: "parent1".to_string(),
        expected_author: "Author".to_string(),
        expected_committer: "Committer".to_string(),
        expected_timestamp: 1000,
        expected_message: "msg".to_string(),
        expected_commit_id: "sha256:expected_id".to_string(),
    };

    let observed = ParsedCommitObject {
        tree: "sha256:tree1".to_string(),
        parent: "parent1".to_string(),
        author: "Author".to_string(),
        committer: "Committer".to_string(),
        timestamp: 1000,
        message: "msg".to_string(),
        commit_id: "sha256:different_id".to_string(), // Mismatched ID!
    };

    let disposition = verifier.verify_commit(&context, Some(&observed));
    assert_eq!(disposition, CommitVerificationDisposition::MISMATCH);
}

#[test]
fn test_agent_006_c15_partial_object_effect_classified() {
    // Tests partial object write classification (Tree written, commit absent -> PARTIAL_EFFECT)
    let verifier = CommitVerifier;
    let classification = verifier.classify_faulty_construction(true, false, false);
    assert_eq!(classification, ObjectEffectClassification::PartialEffect);
}
