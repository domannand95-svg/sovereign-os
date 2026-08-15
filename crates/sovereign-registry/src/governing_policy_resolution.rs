use crate::{Caid, CapabilityPayloadV1, RegistryError, VersionedRegistryNode};

/// Storage-neutral contract for authoritative Gate 6 policy state.
///
/// Implementations must satisfy the deterministic replay and admission-context
/// binding requirements of SPEC-REG-001-A06 and SPEC-REG-001-A06B.
pub trait PolicyStateRef: Eq {}

/// Logical Gate 6 authorization outcome.
///
/// This enum is an implementation representation only; it does not define a
/// persistent Policy payload schema, rule language, or wire encoding.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PolicyAuthorizationOutcome {
    Authorized,
    NotAuthorized,
}

/// Exact admitted Registry v2 Policy object resolved for Gate 6.
///
/// The complete versioned node is preserved so Gate 6 does not invent a
/// reduced Policy representation while Policy payload semantics remain
/// unallocated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGoverningPolicy {
    node: VersionedRegistryNode,
}

impl ResolvedGoverningPolicy {
    pub const fn new(node: VersionedRegistryNode) -> Self {
        Self { node }
    }

    pub const fn node(&self) -> &VersionedRegistryNode {
        &self.node
    }
}

/// Authoritative governing-policy resolution and evaluation contract for Gate 6.
///
/// Resolution and evaluation are logically distinct operations performed
/// against the same authoritative policy-state reference. Exact grant
/// authorization remains bound to the candidate's declared governing-policy
/// CAID and the complete Capability V1 candidate.
pub trait GoverningPolicyAuthority {
    type StateRef: PolicyStateRef;

    fn resolve_policy(
        &self,
        governing_policy: &Caid,
        state_ref: &Self::StateRef,
    ) -> Result<ResolvedGoverningPolicy, RegistryError>;

    fn evaluate_capability(
        &self,
        policy: &ResolvedGoverningPolicy,
        capability: &CapabilityPayloadV1,
        state_ref: &Self::StateRef,
    ) -> Result<PolicyAuthorizationOutcome, RegistryError>;
}
