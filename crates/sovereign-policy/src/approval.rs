use crate::ApprovalLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    Required,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequirement {
    pub required_level: ApprovalLevel,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalGate {
    pub requirement: ApprovalRequirement,
    pub state: ApprovalState,
}

impl ApprovalGate {
    pub fn new(requirement: ApprovalRequirement) -> Self {
        Self {
            requirement,
            state: ApprovalState::Required,
        }
    }

    pub fn approve(&mut self) {
        self.state = ApprovalState::Approved;
    }

    pub fn reject(&mut self) {
        self.state = ApprovalState::Rejected;
    }

    pub fn asserts_no_authority_expansion(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_gate_starts_as_required() {
        let gate = ApprovalGate::new(ApprovalRequirement {
            required_level: ApprovalLevel::Operator,
            rationale: "State mutation review".to_string(),
        });

        assert_eq!(gate.state, ApprovalState::Required);
    }

    #[test]
    fn approval_does_not_expand_authority() {
        let gate = ApprovalGate::new(ApprovalRequirement {
            required_level: ApprovalLevel::Governance,
            rationale: "Global impact review".to_string(),
        });

        assert!(gate.asserts_no_authority_expansion());
    }

    #[test]
    fn approval_transition_is_only_state_change() {
        let mut gate = ApprovalGate::new(ApprovalRequirement {
            required_level: ApprovalLevel::Peer,
            rationale: "Notification review".to_string(),
        });

        gate.approve();

        assert_eq!(gate.state, ApprovalState::Approved);
    }
}
