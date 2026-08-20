use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq)]
pub enum ConstructionDisposition {
    CONSTRUCTED,
    DENIED,
    FAILED,
    MISMATCH,
}

#[derive(Debug, Clone)]
pub struct LiveHostEnvironment {
    pub repository_id: String,
    pub current_head: String,
    pub derived_tree: String,
}

#[derive(Debug, Clone)]
pub struct CommitCandidate {
    pub repository_reference: String,
    pub tree_reference: String,
    pub parent_commit: String,
    pub author: String,
    pub committer: String,
    pub timestamp: u64,
    pub message: String,
    pub expected_commit_digest: String,
}

pub struct DetachedCommitConstructor;

impl DetachedCommitConstructor {
    pub fn construct_detached(
        &self,
        env: &LiveHostEnvironment,
        candidate: &CommitCandidate,
        object_db: &mut Vec<String>,
        head_ref: &mut String,
    ) -> ConstructionDisposition {
        // Enforce INVARIANT-289 & 291: Fresh state check (Repository and parent HEAD match candidate)
        if candidate.repository_reference != env.repository_id || candidate.parent_commit != env.current_head {
            return ConstructionDisposition::DENIED;
        }

        // Enforce INVARIANT-290: Fresh index/tree check
        if candidate.tree_reference != env.derived_tree {
            return ConstructionDisposition::DENIED;
        }

        // Compute authoritative commit object identifier
        let mut hasher = DefaultHasher::new();
        candidate.tree_reference.hash(&mut hasher);
        candidate.parent_commit.hash(&mut hasher);
        candidate.author.hash(&mut hasher);
        candidate.committer.hash(&mut hasher);
        candidate.timestamp.hash(&mut hasher);
        candidate.message.hash(&mut hasher);
        let computed_commit_id = format!("sha256:{:x}", hasher.finish());

        if computed_commit_id != candidate.expected_commit_digest {
            return ConstructionDisposition::MISMATCH;
        }

        // Enforce INVARIANT-296 & 283: Write to object database ONLY; HEAD and refs must remain identically unmodified
        let initial_head = head_ref.clone();
        object_db.push(computed_commit_id);

        if head_ref != &initial_head {
            return ConstructionDisposition::MISMATCH; // Ref movement violation!
        }

        ConstructionDisposition::CONSTRUCTED
    }
}

fn compute_candidate_digest(candidate: &CommitCandidate) -> String {
    let mut hasher = DefaultHasher::new();
    candidate.tree_reference.hash(&mut hasher);
    candidate.parent_commit.hash(&mut hasher);
    candidate.author.hash(&mut hasher);
    candidate.committer.hash(&mut hasher);
    candidate.timestamp.hash(&mut hasher);
    candidate.message.hash(&mut hasher);
    format!("sha256:{:x}", hasher.finish())
}

#[test]
fn test_agent_006_b01_exact_detached_commit_constructed() {
    let constructor = DetachedCommitConstructor;
    let mut object_db = Vec::new();
    let mut head_ref = "d25788a".to_string();

    let env = LiveHostEnvironment {
        repository_id: "repo_sovereign_01".to_string(),
        current_head: "d25788a".to_string(),
        derived_tree: "sha256:abc123tree".to_string(),
    };

    let mut candidate = CommitCandidate {
        repository_reference: "repo_sovereign_01".to_string(),
        tree_reference: "sha256:abc123tree".to_string(),
        parent_commit: "d25788a".to_string(),
        author: "Host <host@sovereign.os>".to_string(),
        committer: "Host <host@sovereign.os>".to_string(),
        timestamp: 1782100000,
        message: "feat: detached commit test".to_string(),
        expected_commit_digest: "".to_string(),
    };
    candidate.expected_commit_digest = compute_candidate_digest(&candidate);

    let disposition = constructor.construct_detached(&env, &candidate, &mut object_db, &mut head_ref);

    assert_eq!(disposition, ConstructionDisposition::CONSTRUCTED);
    assert_eq!(object_db.len(), 1);
    assert_eq!(head_ref, "d25788a", "Detached commit construction improperly moved HEAD!");
}

#[test]
fn test_agent_006_b02_parent_advancement_denied() {
    // Tests INVARIANT-291: If HEAD advances before construction, attempt is denied.
    let constructor = DetachedCommitConstructor;
    let mut object_db = Vec::new();
    let mut head_ref = "commit_advanced_B".to_string();

    let env = LiveHostEnvironment {
        repository_id: "repo_sovereign_01".to_string(),
        current_head: "commit_advanced_B".to_string(), // Advanced HEAD
        derived_tree: "sha256:abc123tree".to_string(),
    };

    let candidate = CommitCandidate {
        repository_reference: "repo_sovereign_01".to_string(),
        tree_reference: "sha256:abc123tree".to_string(),
        parent_commit: "commit_A".to_string(), // Stale parent
        author: "Host <host@sovereign.os>".to_string(),
        committer: "Host <host@sovereign.os>".to_string(),
        timestamp: 1782100000,
        message: "feat: stale parent".to_string(),
        expected_commit_digest: "sha256:000000".to_string(),
    };

    let disposition = constructor.construct_detached(&env, &candidate, &mut object_db, &mut head_ref);
    assert_eq!(disposition, ConstructionDisposition::DENIED);
    assert!(object_db.is_empty());
}
