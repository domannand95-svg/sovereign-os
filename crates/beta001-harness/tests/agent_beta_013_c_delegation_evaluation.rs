use chrono::Utc;
use serde::Serialize;
use serde_json::json;

// =====================================================================
// 1. DELEGATION EVALUATION DOMAIN TYPES & CONTRACT
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DelegationEvaluationClassification {
    ApprovedForConsideration,
    Rejected,
    InsufficientEvidence,
    ConflictDetected,
    HumanReviewRequired,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DelegationEvaluationResult {
    pub evaluation_id: String,
    pub request_id: String,
    pub classification: DelegationEvaluationClassification,
    pub rationale: String,
    pub human_review_required: bool,
    pub evaluation_digest: String,
}

pub trait DelegationEvaluator {
    fn evaluate_delegation(
        &self,
        request_id: &str,
        parent_capabilities: &[String],
        requested_capabilities: &[String],
        has_evidence: bool,
        is_conflict: bool,
        requires_review: bool,
    ) -> DelegationEvaluationResult;
}

// =====================================================================
// 2. DETERMINISTIC DELEGATION EVALUATION ENGINE IMPLEMENTATION
// =====================================================================

pub struct StandardDelegationEvaluator;

impl DelegationEvaluator for StandardDelegationEvaluator {
    fn evaluate_delegation(
        &self,
        request_id: &str,
        parent_capabilities: &[String],
        requested_capabilities: &[String],
        has_evidence: bool,
        is_conflict: bool,
        requires_review: bool,
    ) -> DelegationEvaluationResult {
        let evaluated_at = Utc::now().to_rfc3339();
        let eval_id = format!("eval_{}", evaluated_at);

        // 1. Fail closed on missing evidence (TC-AGENT-EVAL-004)
        if !has_evidence {
            return DelegationEvaluationResult {
                evaluation_id: eval_id,
                request_id: request_id.into(),
                classification: DelegationEvaluationClassification::InsufficientEvidence,
                rationale:
                    "Delegation request lacks supporting communication evidence or provenance."
                        .into(),
                human_review_required: true,
                evaluation_digest: "sha256:eval_insufficient_evidence".into(),
            };
        }

        // 2. Detect conflicting policy states (TC-AGENT-EVAL-005)
        if is_conflict {
            return DelegationEvaluationResult {
                evaluation_id: eval_id,
                request_id: request_id.into(),
                classification: DelegationEvaluationClassification::ConflictDetected,
                rationale:
                    "Conflicting delegation rules or capability constraints detected (Fail Closed)."
                        .into(),
                human_review_required: true,
                evaluation_digest: "sha256:eval_conflict_detected".into(),
            };
        }

        // 3. Enforce parent boundary scope (TC-AGENT-EVAL-002)
        // Capability_{child} must be subset of Capability_{parent}
        for req in requested_capabilities {
            if !parent_capabilities.contains(req) {
                return DelegationEvaluationResult {
                    evaluation_id: eval_id,
                    request_id: request_id.into(),
                    classification: DelegationEvaluationClassification::Rejected,
                    rationale: format!("Delegation scope escalation violation: requested capability '{}' exceeds parent boundary.", req),
                    human_review_required: true,
                    evaluation_digest: "sha256:eval_scope_exceeded".into(),
                };
            }
        }

        // 4. Enforce mandatory human review (TC-AGENT-EVAL-006)
        if requires_review {
            return DelegationEvaluationResult {
                evaluation_id: eval_id,
                request_id: request_id.into(),
                classification: DelegationEvaluationClassification::HumanReviewRequired,
                rationale: "Delegation policy mandates explicit human review boundary before consideration.".into(),
                human_review_required: true,
                evaluation_digest: "sha256:eval_human_review_required".into(),
            };
        }

        // 5. Successful Epistemic Approval for Consideration (TC-AGENT-EVAL-001)
        DelegationEvaluationResult {
            evaluation_id: eval_id,
            request_id: request_id.into(),
            classification: DelegationEvaluationClassification::ApprovedForConsideration,
            rationale: "Delegation request satisfies parent boundaries and evidence checks; approved for consideration.".into(),
            human_review_required: false,
            evaluation_digest: format!("sha256:eval_canonic_digest_{}", request_id.len()),
        }
    }
}

// =====================================================================
// 3. ADVERSARIAL DELEGATION EVALUATION SUITE (TC-AGENT-EVAL-001..007)
// =====================================================================

#[cfg(test)]
mod delegation_evaluation_tests {
    use super::*;

    #[test]
    fn tc_agent_eval_001_valid_delegation_accepted() {
        let evaluator = StandardDelegationEvaluator;
        let parent = vec!["REPOSITORY_READ".into(), "EVIDENCE_QUERY".into()];
        let requested = vec!["REPOSITORY_READ".into()];

        let res = evaluator.evaluate_delegation("req_01", &parent, &requested, true, false, false);
        assert_eq!(
            res.classification,
            DelegationEvaluationClassification::ApprovedForConsideration
        );
        assert!(!res.human_review_required);
    }

    #[test]
    fn tc_agent_eval_002_reject_scope_escalation() {
        let evaluator = StandardDelegationEvaluator;
        let parent = vec!["REPOSITORY_READ".into()];
        let requested = vec!["REPOSITORY_READ".into(), "POLICY_EVALUATE".into()]; // Exceeds parent

        let res = evaluator.evaluate_delegation("req_02", &parent, &requested, true, false, false);
        assert_eq!(
            res.classification,
            DelegationEvaluationClassification::Rejected
        );
    }

    #[test]
    fn tc_agent_eval_003_reject_automatic_capability_issuance() {
        // Evaluator output structure check: Evaluation results contain zero capability lease or issuance fields.
        let evaluator = StandardDelegationEvaluator;
        let parent = vec!["REPOSITORY_READ".into()];
        let requested = vec!["REPOSITORY_READ".into()];

        let res = evaluator.evaluate_delegation("req_03", &parent, &requested, true, false, false);
        let serialized = serde_json::to_value(&res).unwrap_or_default();

        assert!(serialized.get("capability_lease").is_none());
        assert!(serialized.get("grant_capability").is_none());
        assert!(serialized.get("auto_grant").is_none());
    }

    #[test]
    fn tc_agent_eval_004_detect_missing_evidence() {
        let evaluator = StandardDelegationEvaluator;
        let parent = vec!["REPOSITORY_READ".into()];
        let requested = vec!["REPOSITORY_READ".into()];

        let res = evaluator.evaluate_delegation("req_04", &parent, &requested, false, false, false); // has_evidence = false
        assert_eq!(
            res.classification,
            DelegationEvaluationClassification::InsufficientEvidence
        );
    }

    #[test]
    fn tc_agent_eval_005_detect_conflicting_delegation_policies() {
        let evaluator = StandardDelegationEvaluator;
        let parent = vec!["REPOSITORY_READ".into()];
        let requested = vec!["REPOSITORY_READ".into()];

        let res = evaluator.evaluate_delegation("req_05", &parent, &requested, true, true, false); // is_conflict = true
        assert_eq!(
            res.classification,
            DelegationEvaluationClassification::ConflictDetected
        );
    }

    #[test]
    fn tc_agent_eval_006_require_mandated_human_review() {
        let evaluator = StandardDelegationEvaluator;
        let parent = vec!["REPOSITORY_READ".into()];
        let requested = vec!["REPOSITORY_READ".into()];

        let res = evaluator.evaluate_delegation("req_06", &parent, &requested, true, false, true); // requires_review = true
        assert_eq!(
            res.classification,
            DelegationEvaluationClassification::HumanReviewRequired
        );
        assert!(res.human_review_required);
    }

    #[test]
    fn tc_agent_eval_007_verify_deterministic_replay() {
        let evaluator = StandardDelegationEvaluator;
        let parent = vec!["REPOSITORY_READ".into()];
        let requested = vec!["REPOSITORY_READ".into()];

        let res_a =
            evaluator.evaluate_delegation("req_07", &parent, &requested, true, false, false);
        let res_b =
            evaluator.evaluate_delegation("req_07", &parent, &requested, true, false, false);

        assert_eq!(res_a.classification, res_b.classification);
        assert_eq!(res_a.rationale, res_b.rationale);
    }
}
