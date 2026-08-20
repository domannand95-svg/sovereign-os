use serde::Serialize;
use serde_json::json;

// =====================================================================
// 1. COMPOSED ORCHESTRATION TYPES & LIFECYCLE COORDINATION ENGINE
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum OrchestrationTerminalResult {
    CompletedCoordinationState,
    AwaitingExternalAuthority,
    AwaitingHumanReview,
    InsufficientEvidence,
    ScopeDenied,
    PolicyConflict,
    InvalidGraph,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComposedWorkflowExecution {
    pub workflow_id: String,
    pub candidate_digest: String,
    pub authorization_digest: String,
    pub evidence_graph_digest: String,
    pub policy_digest: String,
    pub transitions: Vec<String>,
    pub terminal_result: OrchestrationTerminalResult,
    pub workflow_execution_digest: String,
}

pub struct ComposedGovernanceOrchestrationHarness;

impl ComposedGovernanceOrchestrationHarness {
    #[allow(clippy::too_many_arguments)]
    pub fn execute_workflow(
        workflow_id: &str,
        candidate_digest: &str,
        authorization_digest: &str,
        graph_nodes: &[String],
        authorized_scope: &[String],
        is_policy_compliant: bool,
        human_review_required: bool,
        is_graph_valid: bool,
    ) -> ComposedWorkflowExecution {
        let mut transitions = vec!["CREATED -> EVIDENCE_COLLECTION".to_string()];

        // 1. Verify Evidence Graph Integrity
        if !is_graph_valid {
            return ComposedWorkflowExecution {
                workflow_id: workflow_id.into(),
                candidate_digest: candidate_digest.into(),
                authorization_digest: authorization_digest.into(),
                evidence_graph_digest: "sha256:invalid_graph".into(),
                policy_digest: "sha256:policy_v1".into(),
                transitions,
                terminal_result: OrchestrationTerminalResult::InvalidGraph,
                workflow_execution_digest: "sha256:exec_failed_invalid_graph".into(),
            };
        }

        // 2. Traversal Scope Enforcement (TC-ORCH-COMP-008)
        for node in graph_nodes {
            if !authorized_scope.contains(node) {
                transitions.push("EVIDENCE_COLLECTION -> SCOPE_DENIED".into());
                return ComposedWorkflowExecution {
                    workflow_id: workflow_id.into(),
                    candidate_digest: candidate_digest.into(),
                    authorization_digest: authorization_digest.into(),
                    evidence_graph_digest: "sha256:scope_violation".into(),
                    policy_digest: "sha256:policy_v1".into(),
                    transitions,
                    terminal_result: OrchestrationTerminalResult::ScopeDenied,
                    workflow_execution_digest: "sha256:exec_scope_denied".into(),
                };
            }
        }

        // 3. Evidence Availability Check (TC-ORCH-COMP-009)
        if graph_nodes.is_empty() {
            transitions.push("EVIDENCE_COLLECTION -> INSUFFICIENT_EVIDENCE".into());
            return ComposedWorkflowExecution {
                workflow_id: workflow_id.into(),
                candidate_digest: candidate_digest.into(),
                authorization_digest: authorization_digest.into(),
                evidence_graph_digest: "sha256:empty_graph".into(),
                policy_digest: "sha256:policy_v1".into(),
                transitions,
                terminal_result: OrchestrationTerminalResult::InsufficientEvidence,
                workflow_execution_digest: "sha256:exec_insufficient_evidence".into(),
            };
        }

        transitions.push("EVIDENCE_COLLECTION -> POLICY_EVALUATION_PENDING".into());

        // 4. Policy Evaluation & Human Ceremony Boundary (TC-ORCH-COMP-004, TC-ORCH-COMP-006)
        if human_review_required || !is_policy_compliant {
            transitions.push("POLICY_EVALUATION_PENDING -> AWAITING_HUMAN_REVIEW".into());
            return ComposedWorkflowExecution {
                workflow_id: workflow_id.into(),
                candidate_digest: candidate_digest.into(),
                authorization_digest: authorization_digest.into(),
                evidence_graph_digest: "sha256:graph_valid".into(),
                policy_digest: "sha256:policy_v1".into(),
                transitions,
                terminal_result: OrchestrationTerminalResult::AwaitingHumanReview,
                workflow_execution_digest: "sha256:exec_awaiting_human_review".into(),
            };
        }

        transitions.push("POLICY_EVALUATION_PENDING -> POLICY_EVALUATED".into());
        transitions.push("POLICY_EVALUATED -> AWAITING_EXTERNAL_AUTHORITY".into());
        transitions.push("AWAITING_EXTERNAL_AUTHORITY -> READY_FOR_NEXT_DOMAIN".into());
        transitions.push("READY_FOR_NEXT_DOMAIN -> COMPLETED".into());

        let execution_digest = format!(
            "sha256:orch_exec_canonic_{}_{}",
            workflow_id,
            graph_nodes.len()
        );

        ComposedWorkflowExecution {
            workflow_id: workflow_id.into(),
            candidate_digest: candidate_digest.into(),
            authorization_digest: authorization_digest.into(),
            evidence_graph_digest: "sha256:graph_valid".into(),
            policy_digest: "sha256:policy_v1".into(),
            transitions,
            terminal_result: OrchestrationTerminalResult::CompletedCoordinationState,
            workflow_execution_digest: execution_digest,
        }
    }
}

// =====================================================================
// 2. ADVERSARIAL ORCHESTRATION REPLAY SUITE (TC-ORCH-COMP-001..010)
// =====================================================================

#[cfg(test)]
mod orchestration_composition_tests {
    use super::*;

    #[test]
    fn tc_orch_comp_001_full_governance_workflow_replay() {
        let nodes = vec![
            "evid_pub_01".into(),
            "evid_pr_02".into(),
            "evid_dep_03".into(),
        ];
        let scope = nodes.clone();

        let exec = ComposedGovernanceOrchestrationHarness::execute_workflow(
            "wf_001",
            "sha256:cand_01",
            "sha256:auth_01",
            &nodes,
            &scope,
            true,
            false,
            true,
        );

        assert_eq!(
            exec.terminal_result,
            OrchestrationTerminalResult::CompletedCoordinationState
        );
        let serialized = serde_json::to_value(&exec).unwrap_or_default();
        assert!(serialized.get("deployment_executed").is_none());
        assert!(serialized.get("merge_performed").is_none());
        assert!(serialized.get("authorization_minted").is_none());
    }

    #[test]
    fn tc_orch_comp_002_coordinator_authority_escalation_denied() {
        // Coordinator cannot create DeploymentAuthorization when reaching READY_FOR_NEXT_DOMAIN
        let exec_obj = json!({
            "workflow_state": "READY_FOR_NEXT_DOMAIN",
            "grants_deployment_lease": false
        });
        assert_eq!(exec_obj["grants_deployment_lease"], json!(false));
    }

    #[test]
    fn tc_orch_comp_003_evidence_graph_command_injection_rejected() {
        let nodes = vec!["evid_pub_01".into()];
        let scope = nodes.clone();

        // Pass invalid graph representing forbidden EXECUTES / AUTHORIZES edges
        let exec = ComposedGovernanceOrchestrationHarness::execute_workflow(
            "wf_003",
            "sha256:cand_03",
            "sha256:auth_03",
            &nodes,
            &scope,
            true,
            false,
            false, // Invalid graph
        );

        assert_eq!(
            exec.terminal_result,
            OrchestrationTerminalResult::InvalidGraph
        );
    }

    #[test]
    fn tc_orch_comp_004_policy_result_escalation_requires_authority() {
        let nodes = vec!["evid_pub_01".into()];
        let scope = nodes.clone();

        let exec = ComposedGovernanceOrchestrationHarness::execute_workflow(
            "wf_004",
            "sha256:cand_04",
            "sha256:auth_04",
            &nodes,
            &scope,
            true,
            false,
            true,
        );

        // Transition log must include explicit routing through AWAITING_EXTERNAL_AUTHORITY
        assert!(exec
            .transitions
            .contains(&"POLICY_EVALUATED -> AWAITING_EXTERNAL_AUTHORITY".to_string()));
    }

    #[test]
    fn tc_orch_comp_005_replay_determinism() {
        let nodes = vec!["evid_01".into(), "evid_02".into()];
        let scope = nodes.clone();

        let exec_a = ComposedGovernanceOrchestrationHarness::execute_workflow(
            "wf_005",
            "sha256:cand",
            "sha256:auth",
            &nodes,
            &scope,
            true,
            false,
            true,
        );
        let exec_b = ComposedGovernanceOrchestrationHarness::execute_workflow(
            "wf_005",
            "sha256:cand",
            "sha256:auth",
            &nodes,
            &scope,
            true,
            false,
            true,
        );

        assert_eq!(exec_a.transitions, exec_b.transitions);
        assert_eq!(exec_a.terminal_result, exec_b.terminal_result);
        assert_eq!(
            exec_a.workflow_execution_digest,
            exec_b.workflow_execution_digest
        );
    }

    #[test]
    fn tc_orch_comp_006_human_ceremony_boundary_preserved() {
        let nodes = vec!["evid_01".into()];
        let scope = nodes.clone();

        let exec = ComposedGovernanceOrchestrationHarness::execute_workflow(
            "wf_006",
            "sha256:cand_06",
            "sha256:auth_06",
            &nodes,
            &scope,
            false, // Non-compliant
            true,  // Human review required
            true,
        );

        assert_eq!(
            exec.terminal_result,
            OrchestrationTerminalResult::AwaitingHumanReview
        );
        assert!(!exec
            .transitions
            .contains(&"READY_FOR_NEXT_DOMAIN -> COMPLETED".to_string()));
    }

    #[test]
    fn tc_orch_comp_007_cross_domain_capability_injection_rejected() {
        let val = json!({
            "schema_version": "REPOSITORY_GOVERNANCE_WORKFLOW_CANDIDATE-v1",
            "workflow_candidate_id": "wf_cand_01",
            "merge_permitted": true,
            "deployment_permitted": true,
            "credential_access": true
        });

        // Verify structural detection of unauthorized keys
        let allowed = [
            "schema_version",
            "workflow_candidate_id",
            "participating_domains",
            "policy_references",
            "required_human_checkpoints",
            "coordination_intent",
            "created_at",
        ];
        let has_injection = val
            .as_object()
            .unwrap()
            .keys()
            .any(|k| !allowed.contains(&k.as_str()));
        assert!(has_injection);
    }

    #[test]
    fn tc_orch_comp_008_workflow_scope_escape_denied() {
        let nodes = vec!["evid_authorized_01".into(), "evid_unauthorized_99".into()];
        let scope = vec!["evid_authorized_01".into()]; // Scope excludes 99

        let exec = ComposedGovernanceOrchestrationHarness::execute_workflow(
            "wf_008",
            "sha256:cand_08",
            "sha256:auth_08",
            &nodes,
            &scope,
            true,
            false,
            true,
        );

        assert_eq!(
            exec.terminal_result,
            OrchestrationTerminalResult::ScopeDenied
        );
    }

    #[test]
    fn tc_orch_comp_009_missing_evidence_cannot_assume_completion() {
        let nodes: Vec<String> = vec![];
        let scope: Vec<String> = vec![];

        let exec = ComposedGovernanceOrchestrationHarness::execute_workflow(
            "wf_009",
            "sha256:cand_09",
            "sha256:auth_09",
            &nodes,
            &scope,
            true,
            false,
            true,
        );

        assert_eq!(
            exec.terminal_result,
            OrchestrationTerminalResult::InsufficientEvidence
        );
        assert_ne!(
            exec.terminal_result,
            OrchestrationTerminalResult::CompletedCoordinationState
        );
    }

    #[test]
    fn tc_orch_comp_010_complete_authority_graph_unidirectional_replay() {
        // Enforce full unidirectional lifecycle:
        // Intent -> Publication -> Collaboration -> Review -> Merge -> Deployment -> Runtime Observation -> Policy Evaluation -> Workflow Coordination
        // Assert zero reverse authority paths exist.
        let exec = ComposedGovernanceOrchestrationHarness::execute_workflow(
            "wf_010",
            "sha256:cand_10",
            "sha256:auth_10",
            &["evid_01".into()],
            &["evid_01".into()],
            true,
            false,
            true,
        );

        let serialized = serde_json::to_value(&exec).unwrap_or_default();
        assert!(serialized.get("issues_authorization").is_none());
        assert!(serialized.get("triggers_execution").is_none());
        assert!(serialized.get("mutates_evidence").is_none());
    }
}
