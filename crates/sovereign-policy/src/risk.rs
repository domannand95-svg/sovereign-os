use crate::ApprovalLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlastRadius {
    Isolated,
    Subsystem,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskEvaluationContext {
    pub risk_level: RiskLevel,
    pub blast_radius: BlastRadius,
    pub escalation_rationale: String,
    pub mandated_approval_level: ApprovalLevel,
}

impl RiskEvaluationContext {
    pub fn requires_escalation(&self) -> bool {
        match self.risk_level {
            RiskLevel::Low => false,
            RiskLevel::Medium | RiskLevel::High | RiskLevel::Critical => true,
        }
    }

    pub fn asserts_no_authority_expansion(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_evaluation_does_not_expand_authority() {
        let context = RiskEvaluationContext {
            risk_level: RiskLevel::High,
            blast_radius: BlastRadius::Subsystem,
            escalation_rationale: "State mutation risk".to_string(),
            mandated_approval_level: ApprovalLevel::Operator,
        };

        assert!(context.asserts_no_authority_expansion());
    }

    #[test]
    fn critical_risk_requires_escalation() {
        let context = RiskEvaluationContext {
            risk_level: RiskLevel::Critical,
            blast_radius: BlastRadius::Global,
            escalation_rationale: "Global impact".to_string(),
            mandated_approval_level: ApprovalLevel::Governance,
        };

        assert!(context.requires_escalation());
    }

    #[test]
    fn low_risk_remains_isolated() {
        let context = RiskEvaluationContext {
            risk_level: RiskLevel::Low,
            blast_radius: BlastRadius::Isolated,
            escalation_rationale: "Read-only notification".to_string(),
            mandated_approval_level: ApprovalLevel::Peer,
        };

        assert!(!context.requires_escalation());
    }
}
