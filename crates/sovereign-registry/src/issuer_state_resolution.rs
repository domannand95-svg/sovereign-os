use crate::{IdentityId, RegistryError};

/// Storage-neutral contract for authoritative Gate 5 issuer state.
///
/// Implementations must satisfy the deterministic replay and admission-context
/// binding requirements of SPEC-REG-001-A05 and SPEC-REG-001-A05B.
pub trait IssuerStateRef: Eq {}

/// Logical Gate 5 operational-eligibility outcome.
///
/// This enum is an implementation representation only; it does not define a
/// persistent lifecycle model or wire encoding.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum IssuerOperationalEligibility {
    Eligible,
    Ineligible,
}

/// Minimal authoritative issuer-state resolution result for Gate 5.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ResolvedIssuerState {
    operational_eligibility: IssuerOperationalEligibility,
    capability_v1_issuer_authority: bool,
}

impl ResolvedIssuerState {
    pub const fn new(
        operational_eligibility: IssuerOperationalEligibility,
        capability_v1_issuer_authority: bool,
    ) -> Self {
        Self {
            operational_eligibility,
            capability_v1_issuer_authority,
        }
    }

    pub const fn operational_eligibility(&self) -> IssuerOperationalEligibility {
        self.operational_eligibility
    }

    pub const fn has_capability_v1_issuer_authority(&self) -> bool {
        self.capability_v1_issuer_authority
    }
}

/// Authoritative issuer-state resolver contract for Gate 5.
///
/// Resolution is limited to operational eligibility and general Capability V1
/// issuer authority. Exact grant authorization remains a Gate 6 concern.
pub trait IssuerStateResolver {
    type StateRef: IssuerStateRef;

    fn resolve_issuer_state(
        &self,
        issuer_identity: &IdentityId,
        state_ref: &Self::StateRef,
    ) -> Result<ResolvedIssuerState, RegistryError>;
}
