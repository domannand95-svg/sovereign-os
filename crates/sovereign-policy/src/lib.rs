//! Deterministic directive-admission policy primitives for Sovereign OS.

pub mod decision;
pub mod proposal;

pub use decision::*;
pub use proposal::*;

use sovereign_ledger::EventType;

/// Immutable input evaluated before a directive may reach durable storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectiveRequest<'a> {
    pub event_type: EventType,
    pub payload: &'a [u8],
}

impl<'a> DirectiveRequest<'a> {
    #[must_use]
    pub const fn new(event_type: EventType, payload: &'a [u8]) -> Self {
        Self {
            event_type,
            payload,
        }
    }
}

/// Explicit, deterministic policy result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny(PolicyDenial),
}

/// Stable reasons for a fail-closed denial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDenial {
    EmptyPayload,
    EventTypeDenied,
}

/// Evaluates one directive without mutating state or performing I/O.
pub trait DirectivePolicy {
    type Error;

    fn evaluate(&self, request: DirectiveRequest<'_>) -> Result<PolicyDecision, Self::Error>;
}

/// A deterministic allowlist over the canonical ledger event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EventTypeAllowlist {
    allowed_mask: u8,
}

impl EventTypeAllowlist {
    #[must_use]
    pub fn new(allowed: &[EventType]) -> Self {
        let mut allowed_mask = 0_u8;
        for event_type in allowed {
            allowed_mask |= 1_u8 << (event_type.as_u8() - 1);
        }
        Self { allowed_mask }
    }

    #[must_use]
    pub const fn denies_all() -> Self {
        Self { allowed_mask: 0 }
    }

    #[must_use]
    pub fn permits(&self, event_type: EventType) -> bool {
        let bit = 1_u8 << (event_type.as_u8() - 1);
        self.allowed_mask & bit != 0
    }
}

impl DirectivePolicy for EventTypeAllowlist {
    type Error = core::convert::Infallible;

    fn evaluate(&self, request: DirectiveRequest<'_>) -> Result<PolicyDecision, Self::Error> {
        if request.payload.is_empty() {
            return Ok(PolicyDecision::Deny(PolicyDenial::EmptyPayload));
        }

        if !self.permits(request.event_type) {
            return Ok(PolicyDecision::Deny(PolicyDenial::EventTypeDenied));
        }

        Ok(PolicyDecision::Allow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_is_explicit_and_deterministic() {
        let policy = EventTypeAllowlist::new(&[EventType::RegistryMutation]);
        let request = DirectiveRequest::new(EventType::RegistryMutation, b"node");
        assert_eq!(policy.evaluate(request), Ok(PolicyDecision::Allow));
        assert_eq!(policy.evaluate(request), Ok(PolicyDecision::Allow));
    }

    #[test]
    fn event_types_not_in_allowlist_are_denied() {
        let policy = EventTypeAllowlist::new(&[EventType::RegistryMutation]);
        assert_eq!(
            policy.evaluate(DirectiveRequest::new(
                EventType::KernelDirective,
                b"directive",
            )),
            Ok(PolicyDecision::Deny(PolicyDenial::EventTypeDenied))
        );
    }

    #[test]
    fn empty_payload_is_denied_even_for_allowed_type() {
        let policy = EventTypeAllowlist::new(&[EventType::RegistryMutation]);
        assert_eq!(
            policy.evaluate(DirectiveRequest::new(EventType::RegistryMutation, b"")),
            Ok(PolicyDecision::Deny(PolicyDenial::EmptyPayload))
        );
    }
}
