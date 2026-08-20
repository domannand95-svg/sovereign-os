use crate::encoding::CanonicalHasher;
use crate::identity::AgentIdentityId;
use sovereign_audit::RecordId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalType {
    Effect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalStatus {
    Proposed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectProposal {
    pub proposal_id: ProposalId,
    pub proposer: AgentIdentityId,
    pub objective: RecordId,
    pub evidence: Vec<RecordId>,
    pub proposal_type: ProposalType,
    pub status: ProposalStatus,
    pub intent_digest: String,
}

pub fn derive_proposal_id(
    identity: &AgentIdentityId,
    objective: &RecordId,
    evidence: &[RecordId],
    proposal_type: &ProposalType,
    intent_digest: &str,
) -> ProposalId {
    let mut hasher = CanonicalHasher::new(b"SOVEREIGN_EFFECT_PROPOSAL_ID_V1");
    hasher.field(identity.0.as_bytes());
    hasher.field(objective.as_bytes());
    hasher.field(&(evidence.len() as u32).to_be_bytes());
    for item in evidence {
        hasher.field(item.as_bytes());
    }

    match proposal_type {
        ProposalType::Effect => hasher.field(b"Effect"),
    };

    hasher.field(intent_digest.as_bytes());
    ProposalId(hasher.finish())
}

pub fn validate_proposal_identity(proposal: &EffectProposal) -> bool {
    proposal.status == ProposalStatus::Proposed
        && !proposal.intent_digest.is_empty()
        && proposal.proposal_id
            == derive_proposal_id(
                &proposal.proposer,
                &proposal.objective,
                &proposal.evidence,
                &proposal.proposal_type,
                &proposal.intent_digest,
            )
}
