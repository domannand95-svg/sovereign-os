#[derive(Debug, Clone, PartialEq)]
pub enum RefVerificationDisposition {
    VerifiedTransition,
    NoEffect,
    FullEffectUnverified,
    UNKNOWN,
    MISMATCH,
}

#[derive(Debug, Clone)]
pub struct RefVerificationContext {
    pub ref_name: String,
    pub expected_old_commit: String,
    pub expected_new_commit: String,
}

pub struct RefTransitionVerifier;

impl RefTransitionVerifier {
    pub fn verify_transition(
        &self,
        context: &RefVerificationContext,
        observed_ref_commit: &str,
        adapter_success_claimed: bool,
        is_unknown_state: bool,
    ) -> RefVerificationDisposition {
        if is_unknown_state {
            return RefVerificationDisposition::UNKNOWN;
        }

        // Enforce INVARIANT-375: Adapter success claim is untrusted; verify actual observed ref state
        if observed_ref_commit == context.expected_new_commit {
            if adapter_success_claimed {
                RefVerificationDisposition::VerifiedTransition
            } else {
                // Effect occurred even though adapter reported error/failure
                RefVerificationDisposition::FullEffectUnverified
            }
        } else if observed_ref_commit == context.expected_old_commit {
            RefVerificationDisposition::NoEffect
        } else {
            RefVerificationDisposition::MISMATCH
        }
    }
}

#[test]
fn test_agent_007_c01_exact_ref_transition_verified() {
    let verifier = RefTransitionVerifier;
    let context = RefVerificationContext {
        ref_name: "refs/heads/main".to_string(),
        expected_old_commit: "commit_A".to_string(),
        expected_new_commit: "commit_C".to_string(),
    };

    let disposition = verifier.verify_transition(&context, "commit_C", true, false);
    assert_eq!(disposition, RefVerificationDisposition::VerifiedTransition);
}

#[test]
fn test_agent_007_c03_cas_failed_no_effect() {
    // Tests INVARIANT-384: CAS failure where ref remains unchanged yields NO_EFFECT
    let verifier = RefTransitionVerifier;
    let context = RefVerificationContext {
        ref_name: "refs/heads/main".to_string(),
        expected_old_commit: "commit_A".to_string(),
        expected_new_commit: "commit_C".to_string(),
    };

    let disposition = verifier.verify_transition(&context, "commit_A", false, false);
    assert_eq!(disposition, RefVerificationDisposition::NoEffect);
}

#[test]
fn test_agent_007_c04_error_after_success_full_effect_unverified() {
    // Tests INVARIANT-385: Adapter returns error but ref successfully transitioned -> FULL_EFFECT_UNVERIFIED
    let verifier = RefTransitionVerifier;
    let context = RefVerificationContext {
        ref_name: "refs/heads/main".to_string(),
        expected_old_commit: "commit_A".to_string(),
        expected_new_commit: "commit_C".to_string(),
    };

    let disposition = verifier.verify_transition(&context, "commit_C", false, false);
    assert_eq!(
        disposition,
        RefVerificationDisposition::FullEffectUnverified
    );
}
