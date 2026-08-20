use std::fs;
use std::path::PathBuf;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct PostMutationVerifier {
    checkout_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationDisposition {
    VERIFIED,
    MISMATCH,
    INCOMPLETE,
}

#[derive(Debug, Clone)]
pub struct VerificationContext {
    pub mutation_reference: String,
    pub repository_reference: String,
    pub candidate_reference: String,
    pub expected_target_paths: Vec<String>,
    pub expected_delta_content: String,
    pub expected_delta_digest: String,
}

impl PostMutationVerifier {
    pub fn new(checkout_root: impl Into<PathBuf>) -> Self {
        Self {
            checkout_root: checkout_root.into(),
        }
    }

    pub fn verify_mutation(
        &self,
        context: &VerificationContext,
        observed_paths: &[String],
        observed_delta_content: &str,
    ) -> VerificationDisposition {
        // Enforce INVARIANT-188: No extra target mutation
        if observed_paths != context.expected_target_paths {
            return VerificationDisposition::MISMATCH;
        }

        // Compute observed delta digest
        let mut hasher = DefaultHasher::new();
        observed_delta_content.hash(&mut hasher);
        let observed_digest = format!("sha256:{:x}", hasher.finish());

        // Enforce INVARIANT-187: Exact delta equality
        if observed_digest != context.expected_delta_digest {
            return VerificationDisposition::MISMATCH;
        }

        VerificationDisposition::VERIFIED
    }
}

fn compute_digest(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("sha256:{:x}", hasher.finish())
}

#[test]
fn test_agent_004_b01_exact_delta_verified() {
    let temp_dir = std::env::temp_dir().join("sovereign_repo_ver_test_01");
    let _ = fs::create_dir_all(&temp_dir);

    let verifier = PostMutationVerifier::new(&temp_dir);
    let delta_content = "diff --git a/target.txt b/target.txt\n+ modified content";
    let delta_digest = compute_digest(delta_content);

    let context = VerificationContext {
        mutation_reference: "mut_001".to_string(),
        repository_reference: "repo_sovereign_mut_01".to_string(),
        candidate_reference: "cand_mut_01".to_string(),
        expected_target_paths: vec!["target.txt".to_string()],
        expected_delta_content: delta_content.to_string(),
        expected_delta_digest: delta_digest,
    };

    let observed_paths = vec!["target.txt".to_string()];
    let disposition = verifier.verify_mutation(&context, &observed_paths, delta_content);

    assert_eq!(disposition, VerificationDisposition::VERIFIED);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_agent_004_b03_extra_target_mutation_mismatch() {
    // Tests INVARIANT-188: Touching extra targets results in MISMATCH
    let temp_dir = std::env::temp_dir().join("sovereign_repo_ver_test_02");
    let _ = fs::create_dir_all(&temp_dir);

    let verifier = PostMutationVerifier::new(&temp_dir);
    let delta_content = "diff --git a/target.txt b/target.txt\n+ modified";
    let delta_digest = compute_digest(delta_content);

    let context = VerificationContext {
        mutation_reference: "mut_002".to_string(),
        repository_reference: "repo_sovereign_mut_01".to_string(),
        candidate_reference: "cand_mut_01".to_string(),
        expected_target_paths: vec!["target.txt".to_string()],
        expected_delta_content: delta_content.to_string(),
        expected_delta_digest: delta_digest,
    };

    // Observed modified paths include an unauthorized extra file
    let observed_paths = vec!["target.txt".to_string(), "Cargo.toml".to_string()];
    let disposition = verifier.verify_mutation(&context, &observed_paths, delta_content);

    assert_eq!(disposition, VerificationDisposition::MISMATCH);

    let _ = fs::remove_dir_all(&temp_dir);
}
