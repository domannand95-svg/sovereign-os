use sovereign_agent_runtime::identity::AgentIdentityId;
use sovereign_agent_runtime::proposal::{
    derive_proposal_id, validate_proposal_identity, EffectProposal, ProposalStatus, ProposalType,
};
use sovereign_audit::RecordId;

#[test]
fn o18_c_001_deterministic_proposal_identity() {
    let identity = AgentIdentityId("agent_01".into());

    let objective = RecordId::from_bytes([1u8; 32]);

    let evidence = vec![
        RecordId::from_bytes([2u8; 32]),
        RecordId::from_bytes([3u8; 32]),
    ];

    let proposal_type = ProposalType::Effect;

    let intent_digest = "intent_hash";

    let id1 = derive_proposal_id(
        &identity,
        &objective,
        &evidence,
        &proposal_type,
        intent_digest,
    );

    let id2 = derive_proposal_id(
        &identity,
        &objective,
        &evidence,
        &proposal_type,
        intent_digest,
    );

    assert_eq!(id1, id2);

    let mut proposal = EffectProposal {
        proposal_id: id1,
        proposer: identity,
        objective,
        evidence,
        proposal_type,
        status: ProposalStatus::Proposed,
        intent_digest: intent_digest.into(),
    };
    assert!(validate_proposal_identity(&proposal));
    proposal.intent_digest = "tampered_intent".into();
    assert!(!validate_proposal_identity(&proposal));
}
