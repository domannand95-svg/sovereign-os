# SPEC-REG-001-A05

**Title:** Authoritative Issuer Authorization and Operational Validity (Gate 5)

**Document Status:** REVIEW-READY / PROVISIONAL (Pending Merge)

**Domain:** SOL Registry / Admission Pipeline

## 1. Scope

This specification defines the Gate 5 admission contract for validating the operational eligibility and authoritative permission of a capability's `issuer_identity`. It establishes the criteria for proving that a resolved issuer is operationally permitted to issue capabilities and possesses authoritative issuer-level permission to issue Capability V1 artifacts, operating strictly downstream of identity resolution (Gate 3B) and temporal validation (Gate 4), and upstream of policy evaluation (Gate 6).

## 2. Authority Boundary & Pipeline Context

Gate 5 operates under the core structural invariant:
**Identity existence ≠ identity activity ≠ issuer authority ≠ governing-policy authorization.**

The evaluation sequence across adjacent gates is strictly demarcated:

* **Gate 3B:** Proves structural identity existence and valid canonical derivation.
* **Gate 4:** Proves temporal validity under A04: absence of `expiry` imposes no expiry constraint; when `expiry` is present, validity requires `admission_context_time < expiry`.
* **Gate 5:** Proves operational eligibility and general Capability V1 issuer competency.
* **Gate 6:** Proves exact dynamic authorization against the referenced `governing_policy`.

Gate 5 SHALL NOT evaluate, mimic, or substitute for Gate 6 policy semantics.

## 3. Authoritative Issuer State and Context Binding

Gate 5 SHALL evaluate issuer operational eligibility and issuer-level permission against authoritative state bound to the same authoritative admission context used by the preceding admission gates.

Gate 5 SHALL NOT query mutable, ambient, implicitly current, or otherwise unbound system state.

The authoritative state mechanism used by Gate 5 MAY share the `IdentityStateRef` used by Gate 3B, or MAY use a distinct deterministic state reference associated with the same authoritative admission context.

This specification does not require those state references to have identical types or representations.

The existing Gate 3B `IdentityResolver` and `ResolvedIdentity` contract SHALL NOT be extended implicitly to carry operational-state or authorization information. Gate 5 authority evaluation is a distinct admission concern.

Replay and audit MUST restore the same authoritative admission-context relationship used during the original evaluation.

Later lifecycle changes, revocations, authority changes, or other subsequent state transitions SHALL NOT alter the result of a historical replay performed against the original authoritative admission context.

## 4. Operational Eligibility

Gate 5 MUST establish that the resolved issuer is operationally eligible to issue capabilities within the authoritative admission context.

The authoritative Gate 5 evaluation MUST yield one of two logical outcomes:

* `Eligible`
* `Ineligible`

These are normative evaluation outcomes only.

This specification does not allocate:

* a persistent encoding for issuer operational status;
* a required Rust enum or concrete API representation;
* canonical lifecycle states such as `Active`, `Suspended`, or `Revoked`;
* storage fields or identity-record extensions.

The authoritative identity or authority subsystem MAY derive operational eligibility from a richer lifecycle model defined elsewhere.

Gate 5 SHALL consume only the resulting authoritative eligibility decision.

## 5. Capability V1 Issuer Authority

Operational eligibility alone is insufficient to authorize capability issuance.

Gate 5 MUST additionally establish that the issuer possesses authoritative issuer-level permission to act as a Capability V1 issuer.

This authority establishes general issuer competency only. It does not establish authorization for the exact grant represented by the capability payload.

Gate 5 SHALL NOT:

* resolve or evaluate `governing_policy`;
* determine whether the issuer may issue the exact requested capability grant;
* evaluate target-specific authorization;
* evaluate resource-specific authorization;
* derive issuer authority from the requested operation, target, executable, or resource constraints unless a subsequent specification explicitly allocates such Gate 5 semantics;
* substitute issuer status for governing-policy authorization;
* infer authority merely from identity existence;
* infer authority merely from operational eligibility.

The concrete representation, delegation model, and storage mechanism for Capability V1 issuer authority remain outside this specification unless separately allocated.

Absence of authoritative evidence establishing Capability V1 issuer authority MUST be treated as absence of authority.

## 6. Gate 5 Algorithm

Upon receiving a `CapabilityPayloadV1` that has successfully passed the preceding required admission gates, Gate 5 MUST:

1. Obtain the capability's `issuer_identity`.
2. Evaluate issuer operational eligibility using authoritative state bound to the same authoritative admission context.
3. If operational eligibility cannot be established as `Eligible`, fail closed.
4. Evaluate whether the issuer possesses authoritative issuer-level permission to act as a Capability V1 issuer.
5. If Capability V1 issuer authority cannot be established, fail closed.
6. If both evaluations succeed, advance the admission pipeline to Gate 6.

Gate 5 SHALL NOT repeat Gate 3B structural identity validation or perform Gate 6 governing-policy evaluation.

## 7. Failure Mapping

All Gate 5 authorization failures MUST map to:
`RegistryError::UnauthorizedCapabilityIssuer`

This includes:

* authoritative operational eligibility resolving to `Ineligible`;
* inability to establish authoritative operational eligibility;
* absence of required Capability V1 issuer authority;
* inability to establish Capability V1 issuer authority deterministically;
* unavailable or unusable Gate 5 authority state required for the evaluation.

Gate 5 SHALL NOT silently substitute ambient or current state when authoritative state is unavailable.

Gate 5 SHALL NOT introduce additional public Registry error variants for these cases unless SPEC-REG-001-A03 is explicitly amended.

**Failure to prove authority is equivalent to absence of authority for admission purposes.**

**Unverifiable authority = no authority.**

## 8. Gate 6 Boundary

Gate 5 establishes operational eligibility and general Capability V1 issuer competency. It does not evaluate the conditional, context-dependent rules contained within the capability's `governing_policy`. Passing Gate 5 SHALL NOT be interpreted as authorization of the requested capability itself. Once Gate 5 passes, responsibility shifts entirely to Gate 6 to resolve the `governing_policy` as a formal `Policy` object and authorize the exact requested grant.

## 9. Non-Goals

This specification explicitly excludes:

* Defining the underlying data storage or schema used by the authoritative identity/authority subsystem to track operational states.
* Evaluating or processing the capability's `governing_policy` (reserved for Gate 6).
* Performing subject-state validation or subject-revocation checks.
