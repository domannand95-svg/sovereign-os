#[derive(Debug, Clone, PartialEq)]
pub enum TransitionDisposition {
    TRANSITIONED,
    DENIED,
    FAILED,
    MISMATCH,
}

pub struct RefTransitionAdapter;

impl RefTransitionAdapter {
    pub fn advance_exact(
        &self,
        _ref_name: &str,
        expected_old: &str,
        new_commit: &str,
        current_head: &mut String,
    ) -> TransitionDisposition {
        // Enforce INVARIANT-344: Atomic Compare-and-Swap
        if current_head != expected_old {
            return TransitionDisposition::DENIED;
        }

        // Apply transition
        *current_head = new_commit.to_string();
        TransitionDisposition::TRANSITIONED
    }
}

#[test]
fn test_agent_007_b01_atomic_branch_advance_succeeds() {
    let adapter = RefTransitionAdapter;
    let mut head = "commit_A".to_string();

    let disposition = adapter.advance_exact("refs/heads/main", "commit_A", "commit_C", &mut head);
    assert_eq!(disposition, TransitionDisposition::TRANSITIONED);
    assert_eq!(head, "commit_C");
}

#[test]
fn test_agent_007_b02_race_condition_denied() {
    // Tests INVARIANT-344: Race during CAS results in DENIED, not partial/blind write
    let adapter = RefTransitionAdapter;
    let mut head = "commit_B".to_string(); // HEAD changed concurrently

    let disposition = adapter.advance_exact("refs/heads/main", "commit_A", "commit_C", &mut head);
    assert_eq!(disposition, TransitionDisposition::DENIED);
    assert_eq!(head, "commit_B"); // Unchanged
}
