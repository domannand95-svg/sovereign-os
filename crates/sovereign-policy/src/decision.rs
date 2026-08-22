//! BETA-015 policy decision boundary.
//!
//! Invariant:
//! Policy evaluation may derive a decision.
//! Policy evaluation may not expand authority.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedPolicyDecision {
    Permit,
    Deny,
    Quarantine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEvaluationResult {
    pub decision: DerivedPolicyDecision,
    pub evaluated_rule_id: String,
    pub authority_expansion: usize,
}

impl PolicyEvaluationResult {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.authority_expansion != 0 {
            return Err("Policy evaluation expanded authority");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_evaluation_cannot_expand_authority() {
        let result = PolicyEvaluationResult {
            decision: DerivedPolicyDecision::Permit,
            evaluated_rule_id: "RULE-001".to_string(),
            authority_expansion: 0,
        };

        assert!(result.validate().is_ok());
    }

    #[test]
    fn authority_expansion_is_rejected() {
        let result = PolicyEvaluationResult {
            decision: DerivedPolicyDecision::Permit,
            evaluated_rule_id: "RULE-001".to_string(),
            authority_expansion: 1,
        };

        assert_eq!(
            result.validate(),
            Err("Policy evaluation expanded authority")
        );
    }
}
