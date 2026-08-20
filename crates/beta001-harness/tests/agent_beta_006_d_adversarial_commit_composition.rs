use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdversarialCommitDisposition {
    CONSTRUCTED,
    REJECTED,
    STALE,
    DENIED,
}

#[derive(Debug, Clone)]
pub struct CommitCandidate {
    pub repository_reference: String,
    pub tree_reference: String,
    pub parent_commit: String,
    pub author: String,
    pub committer: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AdversarialCommitContext {
    pub repository_reference: String,
    pub head_commit: String,
    pub tree_reference: String,
    pub grant_active: bool,
    pub revoked: bool,
}

pub struct AdversarialCommitComposer;

impl AdversarialCommitComposer {
    pub fn compose(
        candidate: &CommitCandidate,
        context: &AdversarialCommitContext,
    ) -> (AdversarialCommitDisposition, Option<String>) {
        if !context.grant_active || context.revoked {
            return (AdversarialCommitDisposition::DENIED, None);
        }

        if candidate.repository_reference != context.repository_reference {
            return (AdversarialCommitDisposition::REJECTED, None);
        }

        if candidate.parent_commit != context.head_commit {
            return (AdversarialCommitDisposition::STALE, None);
        }

        if candidate.tree_reference != context.tree_reference {
            return (AdversarialCommitDisposition::REJECTED, None);
        }

        let mut hasher = DefaultHasher::new();
        candidate.repository_reference.hash(&mut hasher);
        candidate.tree_reference.hash(&mut hasher);
        candidate.parent_commit.hash(&mut hasher);
        candidate.author.hash(&mut hasher);
        candidate.committer.hash(&mut hasher);
        candidate.message.hash(&mut hasher);

        let commit_hash = format!("sha256:{:016x}", hasher.finish());
        (AdversarialCommitDisposition::CONSTRUCTED, Some(commit_hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_006_d_identity_impersonation_ignored() {
        let candidate = CommitCandidate {
            repository_reference: "repo1".to_string(),
            tree_reference: "sha256:tree1".to_string(),
            parent_commit: "sha256:head1".to_string(),
            author: "impersonated-author".to_string(),
            committer: "impersonated-committer".to_string(),
            message: "commit message".to_string(),
        };

        let context = AdversarialCommitContext {
            repository_reference: "repo1".to_string(),
            head_commit: "sha256:head1".to_string(),
            tree_reference: "sha256:tree1".to_string(),
            grant_active: true,
            revoked: false,
        };

        let (disposition, commit_hash) = AdversarialCommitComposer::compose(&candidate, &context);
        assert_eq!(disposition, AdversarialCommitDisposition::CONSTRUCTED);
        assert!(commit_hash.is_some());
    }

    #[test]
    fn test_agent_006_d_stale_parent_replay_denied() {
        let candidate = CommitCandidate {
            repository_reference: "repo1".to_string(),
            tree_reference: "sha256:tree1".to_string(),
            parent_commit: "sha256:stale_head".to_string(),
            author: "author".to_string(),
            committer: "committer".to_string(),
            message: "commit message".to_string(),
        };

        let context = AdversarialCommitContext {
            repository_reference: "repo1".to_string(),
            head_commit: "sha256:head1".to_string(),
            tree_reference: "sha256:tree1".to_string(),
            grant_active: true,
            revoked: false,
        };

        let (disposition, commit_hash) = AdversarialCommitComposer::compose(&candidate, &context);
        assert_eq!(disposition, AdversarialCommitDisposition::STALE);
        assert!(commit_hash.is_none());
    }
}
