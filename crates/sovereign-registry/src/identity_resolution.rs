use crate::identity::{IdentityId, IdentityRecord};
use crate::RegistryError;

/// Storage-neutral contract for an authoritative identity-state reference.
///
/// Implementations must satisfy the determinism and replay requirements of
/// SPEC-REG-001-A03B. This trait intentionally allocates no persistent or
/// wire representation.
pub trait IdentityStateRef: Eq {}

/// Minimal successful Gate 3B identity-resolution result.
///
/// This type contains no activity, revocation, trust, or authorization state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIdentity {
    identity_id: IdentityId,
}

impl ResolvedIdentity {
    pub fn from_record(queried_identity_id: &IdentityId, record: &IdentityRecord) -> Option<Self> {
        if record.id() == *queried_identity_id {
            Some(Self {
                identity_id: record.id(),
            })
        } else {
            None
        }
    }

    pub const fn identity_id(&self) -> IdentityId {
        self.identity_id
    }
}

/// Authoritative identity resolver contract for Gate 3B.
///
/// A successful result must correspond to the exact queried `IdentityId`
/// within the supplied authoritative state reference.
pub trait IdentityResolver {
    type StateRef: IdentityStateRef;

    fn resolve(
        &self,
        identity_id: &IdentityId,
        state_ref: &Self::StateRef,
    ) -> Result<ResolvedIdentity, RegistryError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IdentityKind;

    #[test]
    fn resolved_identity_is_derived_from_canonical_identity_record() {
        let record =
            IdentityRecord::new(IdentityKind::Agent, b"gate3b:test-agent".to_vec()).unwrap();

        let resolved = ResolvedIdentity::from_record(&record.id(), &record).unwrap();

        assert_eq!(resolved.identity_id(), record.id());
    }

    #[test]
    fn resolved_identity_rejects_record_for_different_queried_identity() {
        let queried = IdentityRecord::new(IdentityKind::Agent, b"gate3b:queried".to_vec()).unwrap();
        let different =
            IdentityRecord::new(IdentityKind::Agent, b"gate3b:different".to_vec()).unwrap();

        assert_eq!(
            ResolvedIdentity::from_record(&queried.id(), &different),
            None
        );
    }
}
