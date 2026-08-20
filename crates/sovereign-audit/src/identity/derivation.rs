use super::model::{AgentClass, AgentIdentityId, Digest, PolicyId};

pub fn derive_identity_id(
    public_key_digest: &Digest,
    agent_class: &AgentClass,
    policy_reference: &PolicyId,
) -> AgentIdentityId {
    let mut hasher = blake3::Hasher::new();

    hasher.update(b"AGENT_IDENTITY_v1");
    hasher.update(public_key_digest.0.as_bytes());
    hasher.update(agent_class.canonical_bytes());
    hasher.update(policy_reference.0.as_bytes());

    AgentIdentityId(hex::encode(hasher.finalize().as_bytes()))
}
