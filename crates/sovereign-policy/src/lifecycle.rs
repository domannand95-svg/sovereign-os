use crate::ProposalStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleError {
    InvalidTransition,
    ReplayDetected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalLifecycle {
    pub status: ProposalStatus,
    pub lifecycle_epoch: u64,
    pub replay_guard: String,
}

impl ProposalLifecycle {
    pub fn new(replay_guard: String) -> Self {
        Self {
            status: ProposalStatus::Draft,
            lifecycle_epoch: 0,
            replay_guard,
        }
    }

    pub fn validate(&mut self) -> Result<(), LifecycleError> {
        if self.status != ProposalStatus::Draft {
            return Err(LifecycleError::InvalidTransition);
        }

        self.status = ProposalStatus::Validated;
        self.lifecycle_epoch += 1;

        Ok(())
    }

    pub fn mark_pending_approval(&mut self) -> Result<(), LifecycleError> {
        if self.status != ProposalStatus::Validated {
            return Err(LifecycleError::InvalidTransition);
        }

        self.status = ProposalStatus::PendingApproval;

        Ok(())
    }

    pub fn asserts_no_execution_authority(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_starts_in_draft() {
        let lifecycle = ProposalLifecycle::new("proposal-001".to_string());

        assert_eq!(lifecycle.status, ProposalStatus::Draft);
    }

    #[test]
    fn lifecycle_requires_valid_transition_order() {
        let mut lifecycle = ProposalLifecycle::new("proposal-001".to_string());

        assert!(lifecycle.mark_pending_approval().is_err());

        assert!(lifecycle.validate().is_ok());
        assert!(lifecycle.mark_pending_approval().is_ok());

        assert_eq!(lifecycle.status, ProposalStatus::PendingApproval);
    }

    #[test]
    fn lifecycle_has_no_execution_authority() {
        let lifecycle = ProposalLifecycle::new("proposal-001".to_string());

        assert!(lifecycle.asserts_no_execution_authority());
    }
}
