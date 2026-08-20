use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub struct PostStagingVerifier;

#[derive(Debug, Clone, PartialEq)]
pub enum StageVerificationDisposition {
    VERIFIED,
    MISMATCH,
    INCOMPLETE,
}

#[derive(Debug, Clone)]
pub struct StageVerificationContext {
    pub authorized_paths: Vec<String>,
    pub expected_staged_entries: HashMap<String, String>,
    pub expected_index_digest: String,
}

impl PostStagingVerifier {
    pub fn verify_staging(
        &self,
        context: &StageVerificationContext,
        observed_index: &HashMap<String, String>,
        pre_stage_index: &HashMap<String, String>,
    ) -> StageVerificationDisposition {
        // Compute observed new staged paths (entries differing from pre-stage index)
        let mut observed_paths = Vec::new();
        for (path, content) in observed_index {
            match pre_stage_index.get(path) {
                Some(pre_content) if pre_content == content => {}
                _ => observed_paths.push(path.clone()),
            }
        }
        observed_paths.sort();

        let mut expected_paths = context.authorized_paths.clone();
        expected_paths.sort();

        // Enforce INVARIANT-235: Exact path-set equality (no extra or missing staged targets)
        if observed_paths != expected_paths {
            return StageVerificationDisposition::MISMATCH;
        }

        // Verify content matches expected blobs and compute index digest
        let mut hasher = DefaultHasher::new();
        for path in &expected_paths {
            let observed_blob = match observed_index.get(path) {
                Some(b) => b,
                None => return StageVerificationDisposition::INCOMPLETE,
            };
            let expected_blob = match context.expected_staged_entries.get(path) {
                Some(b) => b,
                None => return StageVerificationDisposition::MISMATCH,
            };

            if observed_blob != expected_blob {
                return StageVerificationDisposition::MISMATCH;
            }
            path.hash(&mut hasher);
            observed_blob.hash(&mut hasher);
        }

        let observed_digest = format!("sha256:{:x}", hasher.finish());

        // Enforce INVARIANT-234: Exact index delta digest matching
        if observed_digest != context.expected_index_digest {
            return StageVerificationDisposition::MISMATCH;
        }

        // Enforce INVARIANT-237: Unrelated index preservation check
        for (path, pre_content) in pre_stage_index {
            if !expected_paths.contains(path) {
                match observed_index.get(path) {
                    Some(post_content) if post_content == pre_content => {}
                    _ => return StageVerificationDisposition::MISMATCH, // Unrelated user state altered!
                }
            }
        }

        StageVerificationDisposition::VERIFIED
    }
}

fn compute_index_digest(entries: &[(&str, &str)]) -> String {
    let mut hasher = DefaultHasher::new();
    for (path, content) in entries {
        path.hash(&mut hasher);
        content.hash(&mut hasher);
    }
    format!("sha256:{:x}", hasher.finish())
}

#[test]
fn test_agent_005_b01_exact_index_delta_verified() {
    let verifier = PostStagingVerifier;

    let path = "src/lib.rs";
    let blob_content = "pub fn staging_verified() {}";

    let pre_stage = HashMap::new();
    let mut observed_index = HashMap::new();
    observed_index.insert(path.to_string(), blob_content.to_string());

    let mut expected_entries = HashMap::new();
    expected_entries.insert(path.to_string(), blob_content.to_string());

    let digest = compute_index_digest(&[(path, blob_content)]);

    let context = StageVerificationContext {
        authorized_paths: vec![path.to_string()],
        expected_staged_entries: expected_entries,
        expected_index_digest: digest,
    };

    let disposition = verifier.verify_staging(&context, &observed_index, &pre_stage);
    assert_eq!(disposition, StageVerificationDisposition::VERIFIED);
}

#[test]
fn test_agent_005_b04_extra_unauthorized_file_staged_mismatch() {
    // Tests INVARIANT-235: Extra unauthorized file newly staged results in MISMATCH.
    let verifier = PostStagingVerifier;

    let path = "src/lib.rs";
    let extra_path = "Cargo.toml";
    let blob_content = "content";

    let pre_stage = HashMap::new();
    let mut observed_index = HashMap::new();
    observed_index.insert(path.to_string(), blob_content.to_string());
    observed_index.insert(extra_path.to_string(), "[package]".to_string()); // Unauthorized extra!

    let mut expected_entries = HashMap::new();
    expected_entries.insert(path.to_string(), blob_content.to_string());

    let digest = compute_index_digest(&[(path, blob_content)]);

    let context = StageVerificationContext {
        authorized_paths: vec![path.to_string()],
        expected_staged_entries: expected_entries,
        expected_index_digest: digest,
    };

    let disposition = verifier.verify_staging(&context, &observed_index, &pre_stage);
    assert_eq!(disposition, StageVerificationDisposition::MISMATCH);
}

#[test]
fn test_agent_005_b10_user_index_integrity_preserved() {
    // Tests INVARIANT-237: Unrelated user pre-staged entries must remain unaltered.
    let verifier = PostStagingVerifier;

    let user_path = "docs/private-work.md";
    let agent_path = "src/lib.rs";
    let agent_blob = "agent staged content";

    let mut pre_stage = HashMap::new();
    pre_stage.insert(user_path.to_string(), "original user content".to_string());

    let mut observed_index = pre_stage.clone();
    observed_index.insert(agent_path.to_string(), agent_blob.to_string());

    let mut expected_entries = HashMap::new();
    expected_entries.insert(agent_path.to_string(), agent_blob.to_string());

    let digest = compute_index_digest(&[(agent_path, agent_blob)]);

    let context = StageVerificationContext {
        authorized_paths: vec![agent_path.to_string()],
        expected_staged_entries: expected_entries,
        expected_index_digest: digest,
    };

    let disposition = verifier.verify_staging(&context, &observed_index, &pre_stage);
    assert_eq!(disposition, StageVerificationDisposition::VERIFIED);
}
