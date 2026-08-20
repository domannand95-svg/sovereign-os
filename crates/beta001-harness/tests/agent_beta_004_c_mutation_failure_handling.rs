use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationOutcome {
    NoEffect,
    PartialEffect,
    FullEffectUnverified,
    VerifiedFullEffect,
}

pub struct MutationFailureClassifier;

impl MutationFailureClassifier {
    pub fn classify(
        expected_mutations: &HashMap<String, String>,
        actual_worktree: &HashMap<String, String>,
        baseline_worktree: &HashMap<String, String>,
    ) -> MutationOutcome {
        let mut modified_count = 0;
        let mut matched_expected_count = 0;

        for (path, expected_content) in expected_mutations {
            let actual = actual_worktree.get(path);
            let baseline = baseline_worktree.get(path);

            if actual != baseline {
                modified_count += 1;
                if actual == Some(expected_content) {
                    matched_expected_count += 1;
                }
            }
        }

        if modified_count == 0 {
            MutationOutcome::NoEffect
        } else if matched_expected_count == expected_mutations.len() {
            MutationOutcome::VerifiedFullEffect
        } else {
            MutationOutcome::PartialEffect
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_004_c_unknown_state_fails_closed() {
        let expected = HashMap::from([("file1.txt".to_string(), "new_content".to_string())]);
        let baseline = HashMap::from([("file1.txt".to_string(), "old_content".to_string())]);
        let actual = baseline.clone(); // No mutation occurred

        let outcome = MutationFailureClassifier::classify(&expected, &actual, &baseline);
        assert_eq!(outcome, MutationOutcome::NoEffect);
    }

    #[test]
    fn test_agent_004_c_partial_effect_classified() {
        let expected = HashMap::from([
            ("file1.txt".to_string(), "new_content_1".to_string()),
            ("file2.txt".to_string(), "new_content_2".to_string()),
        ]);
        let baseline = HashMap::from([
            ("file1.txt".to_string(), "old_content_1".to_string()),
            ("file2.txt".to_string(), "old_content_2".to_string()),
        ]);

        // file1 updated, file2 left untouched
        let actual = HashMap::from([
            ("file1.txt".to_string(), "new_content_1".to_string()),
            ("file2.txt".to_string(), "old_content_2".to_string()),
        ]);

        let outcome = MutationFailureClassifier::classify(&expected, &actual, &baseline);
        assert_eq!(outcome, MutationOutcome::PartialEffect);
    }
}
