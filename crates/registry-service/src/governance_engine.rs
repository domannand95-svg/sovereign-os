use crate::CapabilityTier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Vetoed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    Approve,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub proposal_id: [u8; 16],
    pub proposer_id: [u8; 16],
    pub required_tier: CapabilityTier,
    pub resource_cost_estimate: u64,
    pub status: ProposalStatus,
    pub approvals: u32,
    pub rejections: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceError {
    InsufficientCapabilityTier,
    ResourceLimitExceeded,
    ProposalNotActive,
    InvalidApprovalThreshold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceEngine {
    pub quorum_threshold: u32,
    pub approval_threshold_basis_points: u32,
    pub max_resource_limit: u64,
}

impl GovernanceEngine {
    pub fn new(
        quorum_threshold: u32,
        approval_threshold_basis_points: u32,
        max_resource_limit: u64,
    ) -> Result<Self, GovernanceError> {
        if approval_threshold_basis_points > 10_000 {
            return Err(GovernanceError::InvalidApprovalThreshold);
        }

        Ok(Self {
            quorum_threshold,
            approval_threshold_basis_points,
            max_resource_limit,
        })
    }

    pub fn create_proposal(
        &self,
        proposal_id: [u8; 16],
        proposer_id: [u8; 16],
        proposer_tier: CapabilityTier,
        required_tier: CapabilityTier,
        resource_cost_estimate: u64,
    ) -> Result<Proposal, GovernanceError> {
        if proposer_tier < required_tier {
            return Err(GovernanceError::InsufficientCapabilityTier);
        }

        if resource_cost_estimate > self.max_resource_limit {
            return Err(GovernanceError::ResourceLimitExceeded);
        }

        Ok(Proposal {
            proposal_id,
            proposer_id,
            required_tier,
            resource_cost_estimate,
            status: ProposalStatus::Active,
            approvals: 0,
            rejections: 0,
        })
    }

    pub fn cast_vote(
        &self,
        mut proposal: Proposal,
        vote: VoteType,
    ) -> Result<Proposal, GovernanceError> {
        if proposal.status != ProposalStatus::Active {
            return Err(GovernanceError::ProposalNotActive);
        }

        match vote {
            VoteType::Approve => proposal.approvals += 1,
            VoteType::Reject => proposal.rejections += 1,
        }

        Ok(proposal)
    }

    pub fn evaluate_result(&self, mut proposal: Proposal) -> (Proposal, Option<u32>) {
        if proposal.status != ProposalStatus::Active {
            return (proposal, None);
        }

        let total_votes = proposal.approvals + proposal.rejections;

        if total_votes < self.quorum_threshold {
            return (proposal, None);
        }

        let approval_basis_points =
            (proposal.approvals as u64 * 10_000) / total_votes as u64;

        if approval_basis_points >= self.approval_threshold_basis_points as u64 {
            proposal.status = ProposalStatus::Passed;
            let merit_points = proposal.approvals * 10;
            (proposal, Some(merit_points))
        } else {
            proposal.status = ProposalStatus::Rejected;
            (proposal, None)
        }
    }

    pub fn trigger_veto(&self, mut proposal: Proposal) -> Proposal {
        proposal.status = ProposalStatus::Vetoed;
        proposal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: u8) -> [u8; 16] {
        [value; 16]
    }

    #[test]
    fn rejects_invalid_approval_threshold() {
        assert_eq!(
            GovernanceEngine::new(3, 10_001, 100),
            Err(GovernanceError::InvalidApprovalThreshold)
        );
    }

    #[test]
    fn creates_active_proposal() {
        let engine = GovernanceEngine::new(3, 6_600, 100).unwrap();

        let proposal = engine
            .create_proposal(
                id(1),
                id(2),
                CapabilityTier::Tier1Standard,
                CapabilityTier::Tier1Standard,
                50,
            )
            .unwrap();

        assert_eq!(proposal.status, ProposalStatus::Active);
        assert_eq!(proposal.approvals, 0);
        assert_eq!(proposal.rejections, 0);
    }

    #[test]
    fn rejects_proposal_above_capability_tier() {
        let engine = GovernanceEngine::new(3, 6_600, 100).unwrap();

        assert_eq!(
            engine.create_proposal(
                id(1),
                id(2),
                CapabilityTier::Tier0Sandbox,
                CapabilityTier::Tier1Standard,
                50,
            ),
            Err(GovernanceError::InsufficientCapabilityTier)
        );
    }

    #[test]
    fn rejects_resource_limit_excess() {
        let engine = GovernanceEngine::new(3, 6_600, 100).unwrap();

        assert_eq!(
            engine.create_proposal(
                id(1),
                id(2),
                CapabilityTier::Tier1Standard,
                CapabilityTier::Tier1Standard,
                101,
            ),
            Err(GovernanceError::ResourceLimitExceeded)
        );
    }

    #[test]
    fn records_votes_on_active_proposal() {
        let engine = GovernanceEngine::new(3, 6_600, 100).unwrap();

        let proposal = engine
            .create_proposal(
                id(1),
                id(2),
                CapabilityTier::Tier1Standard,
                CapabilityTier::Tier1Standard,
                50,
            )
            .unwrap();

        let proposal = engine.cast_vote(proposal, VoteType::Approve).unwrap();
        let proposal = engine.cast_vote(proposal, VoteType::Reject).unwrap();

        assert_eq!(proposal.approvals, 1);
        assert_eq!(proposal.rejections, 1);
    }

    #[test]
    fn rejects_vote_on_closed_proposal() {
        let engine = GovernanceEngine::new(3, 6_600, 100).unwrap();

        let proposal = Proposal {
            proposal_id: id(1),
            proposer_id: id(2),
            required_tier: CapabilityTier::Tier1Standard,
            resource_cost_estimate: 50,
            status: ProposalStatus::Passed,
            approvals: 3,
            rejections: 0,
        };

        assert_eq!(
            engine.cast_vote(proposal, VoteType::Approve),
            Err(GovernanceError::ProposalNotActive)
        );
    }

    #[test]
    fn evaluation_waits_for_quorum() {
        let engine = GovernanceEngine::new(3, 6_600, 100).unwrap();

        let proposal = engine
            .create_proposal(
                id(1),
                id(2),
                CapabilityTier::Tier1Standard,
                CapabilityTier::Tier1Standard,
                50,
            )
            .unwrap();

        let proposal = engine.cast_vote(proposal, VoteType::Approve).unwrap();
        let (proposal, points) = engine.evaluate_result(proposal);

        assert_eq!(proposal.status, ProposalStatus::Active);
        assert_eq!(points, None);
    }

    #[test]
    fn evaluation_passes_when_threshold_met() {
        let engine = GovernanceEngine::new(3, 6_600, 100).unwrap();

        let proposal = engine
            .create_proposal(
                id(1),
                id(2),
                CapabilityTier::Tier1Standard,
                CapabilityTier::Tier1Standard,
                50,
            )
            .unwrap();

        let proposal = engine.cast_vote(proposal, VoteType::Approve).unwrap();
        let proposal = engine.cast_vote(proposal, VoteType::Approve).unwrap();
        let proposal = engine.cast_vote(proposal, VoteType::Reject).unwrap();

        let (proposal, points) = engine.evaluate_result(proposal);

        assert_eq!(proposal.status, ProposalStatus::Passed);
        assert_eq!(points, Some(20));
    }

    #[test]
    fn evaluation_rejects_when_threshold_not_met() {
        let engine = GovernanceEngine::new(3, 6_600, 100).unwrap();

        let proposal = engine
            .create_proposal(
                id(1),
                id(2),
                CapabilityTier::Tier1Standard,
                CapabilityTier::Tier1Standard,
                50,
            )
            .unwrap();

        let proposal = engine.cast_vote(proposal, VoteType::Approve).unwrap();
        let proposal = engine.cast_vote(proposal, VoteType::Reject).unwrap();
        let proposal = engine.cast_vote(proposal, VoteType::Reject).unwrap();

        let (proposal, points) = engine.evaluate_result(proposal);

        assert_eq!(proposal.status, ProposalStatus::Rejected);
        assert_eq!(points, None);
    }

    #[test]
    fn veto_forces_vetoed_status() {
        let engine = GovernanceEngine::new(3, 6_600, 100).unwrap();

        let proposal = engine
            .create_proposal(
                id(1),
                id(2),
                CapabilityTier::Tier1Standard,
                CapabilityTier::Tier1Standard,
                50,
            )
            .unwrap();

        let proposal = engine.trigger_veto(proposal);

        assert_eq!(proposal.status, ProposalStatus::Vetoed);
    }
}
