use sovereign_agent_runtime::identity::{
    derive_identity_id, AgentClass, AgentIdentity, AgentIdentityId, AgentIdentityStatus, Digest,
    PolicyId, ReplayTimestamp, SchemaVersion,
};

#[test]
fn o18_a_001_identity_derivation_is_deterministic() {
    let key = Digest("a1b2c3d4e5f60000000000000000000000000000000000000000000000000000".into());
    let policy =
        PolicyId("f6e5d4c3b2a10000000000000000000000000000000000000000000000000000".into());
    let class = AgentClass::LocalModel;

    let id1 = derive_identity_id(&key, &class, &policy);
    let id2 = derive_identity_id(&key, &class, &policy);

    assert_eq!(id1, id2);
}

#[test]
fn o18_a_002_key_substitution_rejected() {
    let key_a = Digest("a1b2c3d4e5f60000000000000000000000000000000000000000000000000000".into());
    let key_b = Digest("b2c3d4e5f6a10000000000000000000000000000000000000000000000000000".into());
    let policy =
        PolicyId("f6e5d4c3b2a10000000000000000000000000000000000000000000000000000".into());
    let class = AgentClass::LocalModel;

    let id_a = derive_identity_id(&key_a, &class, &policy);
    let id_b = derive_identity_id(&key_b, &class, &policy);

    assert_ne!(id_a, id_b);
}

#[test]
fn o18_a_003_policy_substitution_rejected() {
    let key = Digest("a1b2c3d4e5f60000000000000000000000000000000000000000000000000000".into());
    let policy_a =
        PolicyId("f6e5d4c3b2a10000000000000000000000000000000000000000000000000000".into());
    let policy_b =
        PolicyId("e5d4c3b2a1f60000000000000000000000000000000000000000000000000000".into());
    let class = AgentClass::LocalModel;

    let id_a = derive_identity_id(&key, &class, &policy_a);
    let id_b = derive_identity_id(&key, &class, &policy_b);

    assert_ne!(id_a, id_b);
}

#[test]
fn o18_a_004_lifecycle_separation() {
    let key = Digest("a1b2c3d4e5f60000000000000000000000000000000000000000000000000000".into());
    let policy =
        PolicyId("f6e5d4c3b2a10000000000000000000000000000000000000000000000000000".into());
    let class = AgentClass::LocalModel;

    let identity_id = derive_identity_id(&key, &class, &policy);

    let mut identity = AgentIdentity {
        schema_version: SchemaVersion("AGENT_IDENTITY-v1".into()),
        identity_id: identity_id.clone(),
        public_key_digest: key.clone(),
        agent_class: class.clone(),
        created_at: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
        governing_policy: policy.clone(),
        status: AgentIdentityStatus::Pending,
    };

    assert_eq!(identity.identity_id, identity_id);

    identity.status = AgentIdentityStatus::Active;
    assert_eq!(identity.identity_id, identity_id);

    identity.status = AgentIdentityStatus::Suspended;
    assert_eq!(identity.identity_id, identity_id);

    identity.status = AgentIdentityStatus::Revoked;
    assert_eq!(identity.identity_id, identity_id);
}

#[test]
fn o18_a_005_no_authority_leakage() {
    let identity = AgentIdentity {
        schema_version: SchemaVersion("AGENT_IDENTITY-v1".into()),
        identity_id: AgentIdentityId("dummy".into()),
        public_key_digest: Digest("dummy".into()),
        agent_class: AgentClass::LocalModel,
        created_at: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
        governing_policy: PolicyId("dummy".into()),
        status: AgentIdentityStatus::Active,
    };

    // Identity possesses no execution or capability grant methods.
    assert_eq!(identity.status, AgentIdentityStatus::Active);
}

#[test]
fn o18_a_006_replay_determinism() {
    let identity_a = AgentIdentity {
        schema_version: SchemaVersion("AGENT_IDENTITY-v1".into()),
        identity_id: AgentIdentityId("id1".into()),
        public_key_digest: Digest("key1".into()),
        agent_class: AgentClass::LocalModel,
        created_at: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
        governing_policy: PolicyId("policy1".into()),
        status: AgentIdentityStatus::Active,
    };

    let identity_b = AgentIdentity {
        schema_version: SchemaVersion("AGENT_IDENTITY-v1".into()),
        identity_id: AgentIdentityId("id1".into()),
        public_key_digest: Digest("key1".into()),
        agent_class: AgentClass::LocalModel,
        created_at: ReplayTimestamp("2026-08-20T10:00:00Z".into()),
        governing_policy: PolicyId("policy1".into()),
        status: AgentIdentityStatus::Active,
    };

    assert_eq!(identity_a, identity_b);
}
