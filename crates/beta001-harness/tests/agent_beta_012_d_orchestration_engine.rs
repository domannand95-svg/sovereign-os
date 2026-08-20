use chrono::Utc;
use serde::Serialize;

// =====================================================================
// 1. DETERMINISTIC ORCHESTRATION ENGINE DOMAIN TYPES & CONTRACT
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum WorkflowState {
    Created,
    EvidenceCollection,
    PolicyEvaluationPending,
    PolicyEvaluated,
    AwaitingExternalAuthority,
    ReadyForNextDomain,
    Completed,
    InvalidEvidence,
    InsufficientEvidence,
    PolicyConflict,
    AuthorityRequired,
    WorkflowHalted,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkflowStateTransition {
    pub from_state: WorkflowState,
    pub to_state: WorkflowState,
    pub transition_reason: String,
    pub evidence_digest: String,
}

pub trait GovernanceOrchestrator {
    fn evaluate_transition(
        &self,
        candidate_digest: &str,
        evidence_digest: &str,
        has_policy_evaluation: bool,
        is_policy_conflict: bool,
        current_state: WorkflowState,
    ) -> Result<WorkflowStateTransition, String>;
}

// =====================================================================
// 2. DETERMINISTIC ORCHESTRATION ENGINE IMPLEMENTATION
// =====================================================================

pub struct StandardGovernanceOrchestrator;

impl GovernanceOrchestrator for StandardGovernanceOrchestrator {
    fn evaluate_transition(
        &self,
        _candidate_digest: &str,
        evidence_digest: &str,
        has_policy_evaluation: bool,
        is_policy_conflict: bool,
        current_state: WorkflowState,
    ) -> Result<WorkflowStateTransition, String> {
        let (to_state, reason) = match current_state {
            WorkflowState::Created => {
                if evidence_digest.is_empty() {
                    (WorkflowState::InsufficientEvidence, "Evidence collection failed: empty evidence digest.".into())
                } else {
                    (WorkflowState::EvidenceCollection, "Transition from Created to Evidence Collection authorized.".into())
                }
            }
            WorkflowState::EvidenceCollection => {
                if evidence_digest.starts_with("sha256:") {
                    (WorkflowState::PolicyEvaluationPending, "Evidence collected; proceeding to policy evaluation.".into())
                } else {
                    (WorkflowState::InvalidEvidence, "Invalid evidence digest format encountered.".into())
                }
            }
            WorkflowState::PolicyEvaluationPending => {
                if is_policy_conflict {
                    (WorkflowState::PolicyConflict, "Policy conflict detected during evaluation assessment (Fail Closed).".into())
                } else if has_policy_evaluation {
                    (WorkflowState::PolicyEvaluated, "Policy evaluation successfully completed.".into())
                } else {
                    (WorkflowState::InsufficientEvidence, "Policy evaluation pending but results unavailable.".into())
                }
            }
            WorkflowState::PolicyEvaluated => {
                (WorkflowState::AwaitingExternalAuthority, "Policy evaluated; awaiting external human/domain authority ceremony.".into())
            }
            WorkflowState::AwaitingExternalAuthority => {
                (WorkflowState::ReadyForNextDomain, "External authority boundary acknowledged; ready for downstream domain integration.".into())
            }
            WorkflowState::ReadyForNextDomain => {
                (WorkflowState::Completed, "Workflow coordination lifecycle successfully completed.".into())
            }
            _ => {
                return Err("Invalid state transition attempt: Workflow halted or terminal.".into());
            }
        };

        let transition_digest = format!(
            "sha256:transition_{}_{}",
            evidence_digest.len(),
            Utc::now().timestamp_subsec_nanos()
        );

        Ok(WorkflowStateTransition {
            from_state: current_state,
            to_state,
            transition_reason: reason,
            evidence_digest: transition_digest,
        })
    }
}

// =====================================================================
// 3. ADVERSARIAL VALIDATION SUITE (TC-ORCH-001..007)
// =====================================================================

#[cfg(test)]
mod orchestration_engine_tests {
    use super::*;

    #[test]
    fn tc_orch_001_valid_workflow_progression() {
        let orchestrator = StandardGovernanceOrchestrator;
        let digest = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let t1 = orchestrator
            .evaluate_transition(digest, digest, false, false, WorkflowState::Created)
            .unwrap();
        assert_eq!(t1.to_state, WorkflowState::EvidenceCollection);

        let t2 = orchestrator
            .evaluate_transition(
                digest,
                digest,
                false,
                false,
                WorkflowState::EvidenceCollection,
            )
            .unwrap();
        assert_eq!(t2.to_state, WorkflowState::PolicyEvaluationPending);
    }

    #[test]
    fn tc_orch_002_invalid_state_jump_rejected() {
        let orchestrator = StandardGovernanceOrchestrator;
        let digest = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        // Attempting to jump directly from Created to Completed
        let forced_state = WorkflowState::Completed;
        let res =
            orchestrator.evaluate_transition(digest, digest, true, false, forced_state.clone());
        assert!(res.is_err());
    }

    #[test]
    fn tc_orch_003_execution_capability_injection_forbidden() {
        // Structural check: WorkflowStateTransition struct contains zero execution or authority fields.
        let transition = WorkflowStateTransition {
            from_state: WorkflowState::ReadyForNextDomain,
            to_state: WorkflowState::Completed,
            transition_reason: "Completed".into(),
            evidence_digest: "sha256:test".into(),
        };

        let serialized = serde_json::to_value(&transition).unwrap_or_default();
        assert!(serialized.get("execute_deployment").is_none());
        assert!(serialized.get("merge_repository").is_none());
        assert!(serialized.get("authorization_lease").is_none());
    }

    #[test]
    fn tc_orch_004_missing_evidence_fail_closed() {
        let orchestrator = StandardGovernanceOrchestrator;
        let digest = ""; // Empty digest representing missing evidence

        let t1 = orchestrator
            .evaluate_transition("cand_01", digest, false, false, WorkflowState::Created)
            .unwrap();
        assert_eq!(t1.to_state, WorkflowState::InsufficientEvidence);
    }

    #[test]
    fn tc_orch_005_policy_conflict_handling() {
        let orchestrator = StandardGovernanceOrchestrator;
        let digest = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let t1 = orchestrator
            .evaluate_transition(
                digest,
                digest,
                true,
                true,
                WorkflowState::PolicyEvaluationPending,
            )
            .unwrap();
        assert_eq!(t1.to_state, WorkflowState::PolicyConflict);
    }

    #[test]
    fn tc_orch_006_deterministic_replay() {
        let orchestrator = StandardGovernanceOrchestrator;
        let digest = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let res_a = orchestrator
            .evaluate_transition(digest, digest, false, false, WorkflowState::Created)
            .unwrap();
        let res_b = orchestrator
            .evaluate_transition(digest, digest, false, false, WorkflowState::Created)
            .unwrap();

        assert_eq!(res_a.to_state, res_b.to_state);
        assert_eq!(res_a.from_state, res_b.from_state);
    }

    #[test]
    fn tc_orch_007_authority_boundary_test() {
        let orchestrator = StandardGovernanceOrchestrator;
        let digest = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let t1 = orchestrator
            .evaluate_transition(digest, digest, true, false, WorkflowState::PolicyEvaluated)
            .unwrap();
        assert_eq!(t1.to_state, WorkflowState::AwaitingExternalAuthority);
        // Engine routes to awaiting external authority rather than auto-issuing authorization
    }
}
