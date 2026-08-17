# SPEC-REG-001-A05B

**Title:** Authoritative Issuer-State Resolution Contract

**Document Status:** Approved / Locked

**Domain:** SOL Registry / Admission Pipeline

## 1. Scope

This specification defines the minimum implementation-facing contract for resolving authoritative issuer operational eligibility and Capability V1 issuer authority for Gate 5 admission.

It exists to bridge the normative Gate 5 requirements defined by SPEC-REG-001-A05 and the concrete implementation interfaces used by `sovereign-registry`.

This specification defines:

* the authoritative inputs required for Gate 5 issuer-state resolution;
* the minimum logical outcome that a resolver must provide;
* deterministic and replay-safe resolution semantics;
* the separation between issuer-state resolution and Gate 5 admission failure mapping;
* the boundaries that prevent Gate 5 authority state from leaking into identity resolution, policy evaluation, or unrelated node-management subsystems.

This specification does not define a concrete storage engine, database schema, ledger representation, persistent wire format, or specific Rust trait name.

## 2. Authority Boundary

Gate 5 operates under the existing invariant:

**Identity existence ≠ identity activity ≠ issuer authority ≠ governing-policy authorization.**

SPEC-REG-001-A03B establishes structural identity resolution only.

SPEC-REG-001-A05 establishes the Gate 5 requirement to prove:

1. issuer operational eligibility; and
2. authoritative Capability V1 issuer permission.

This specification defines only the resolution contract required to obtain those Gate 5 state determinations.

It SHALL NOT expand the responsibilities of Gate 3B or Gate 6.

## 3. Authoritative Issuer-State Input

Issuer-state resolution MUST receive:

1. the exact `IdentityId` of the capability issuer; and
2. an explicit authoritative issuer-state reference.

The authoritative issuer-state reference MUST identify or deterministically bind the resolution to issuer authority state belonging to the same authoritative admission context used by the surrounding admission pipeline.

The issuer-state reference MUST be supplied explicitly by the governing admission environment.

The resolver SHALL NOT obtain issuer authority state from:

* ambient mutable process state;
* an implicitly current database snapshot;
* an implicitly current registry or ledger head;
* wall-clock time;
* hardware time;
* mutable singleton state;
* an unbound cache;
* an unversioned "latest" authority record;
* any other source whose authoritative state cannot be reproduced during deterministic replay.

The issuer-state reference MAY share the same concrete representation as the Gate 3B `IdentityStateRef`.

The issuer-state reference MAY instead use a distinct type or representation.

This specification does not require Gate 3B identity state and Gate 5 issuer authority state to share the same storage mechanism or reference type.

If separate references are used, they MUST nevertheless belong to the same authoritative admission context.

## 4. Storage Neutrality

The authoritative issuer-state reference is storage-neutral.

This specification does not require it to be:

* a ledger sequence number;
* a block height;
* a database transaction identifier;
* a state root;
* a content address;
* a registry object;
* a serialized authority record;
* a timestamp.

An implementation MAY use any deterministic representation capable of identifying the authoritative issuer state required by Gate 5.

Persistent encoding and maximum serialized size for such a reference remain unallocated unless separately specified.

## 5. Minimum Resolution Outcome

A successful authoritative issuer-state resolution MUST provide two logically independent determinations.

### 5.1 Operational Eligibility

The resolver MUST establish one of:

* `Eligible`
* `Ineligible`

These names define normative logical outcomes only.

They do not require a concrete Rust enum, persistent representation, or canonical identity lifecycle model.

The resolver MAY derive this result from a richer authoritative lifecycle model defined elsewhere.

Gate 5 SHALL consume only the resulting eligibility determination.

### 5.2 Capability V1 Issuer Authority

The resolver MUST additionally determine whether authoritative evidence establishes that the identity may act as a Capability V1 issuer.

The logical result is:

* issuer authority established; or
* issuer authority not established.

This specification does not require a concrete boolean, enum, bit flag, role object, delegation record, or authorization token representation.

The two determinations are independent.

Operational eligibility MUST NOT imply Capability V1 issuer authority.

Capability V1 issuer authority MUST NOT imply operational eligibility.

Gate 5 succeeds only when both required conditions are established by authoritative state.

## 6. Resolver Semantics

For a given:

* `IdentityId`; and
* authoritative issuer-state reference,

issuer-state resolution MUST be deterministic.

Equivalent resolution against the same authoritative issuer state MUST produce the same logical outcome.

The resolver MUST NOT incorporate mutable ambient state into its result.

The resolver MUST NOT infer authority from:

* identity existence alone;
* `IdentityKind`;
* target scope;
* operation code;
* authorized executable;
* resource constraints;
* `governing_policy`;
* current wall-clock time;
* unrelated registry-node lifecycle state;
* workload scheduler state;
* runtime process state.

The resolver SHALL resolve only the authoritative Gate 5 issuer-state concerns allocated by SPEC-REG-001-A05.

## 7. Replay Requirements

Historical replay MUST evaluate issuer state against the same authoritative admission-context relationship used during the original admission decision.

Later changes to issuer state MUST NOT alter the outcome of replay performed against the original authoritative issuer-state reference.

This includes later:

* lifecycle transitions;
* revocations;
* restorations;
* issuer-authority grants;
* issuer-authority removals;
* delegation changes;
* storage migrations;
* current-state changes.

Replay MUST NOT silently substitute the issuer's present state for the state referenced by the original admission context.

## 8. Separation of Resolution and Admission

Issuer-state resolution and Gate 5 admission are distinct responsibilities.

The issuer-state resolver SHOULD expose authoritative state outcomes without itself deciding the final Registry admission result.

The Gate 5 validator is responsible for applying SPEC-REG-001-A05 admission semantics.

Conceptually:

```text
IdentityId
+ authoritative issuer-state reference
        ↓
issuer-state resolution
        ↓
operational eligibility
+ Capability V1 issuer authority
        ↓
Gate 5 validator
        ↓
admit or fail with RegistryError::UnauthorizedCapabilityIssuer
```

The concrete Rust function or trait names remain unallocated by this specification.

A future implementation MAY provide an interface analogous to an issuer-state resolver, but this specification does not mandate the identifier `IssuerAuthorityResolver`, `IssuerStateResolver`, or any other specific API name.

## 9. Failure and Unavailability Semantics

Failure to obtain usable authoritative issuer state MUST fail closed at Gate 5.

This includes:

* issuer-state reference unavailable;
* authoritative state unavailable;
* authoritative state unusable;
* resolution unable to establish operational eligibility;
* resolution unable to establish Capability V1 issuer authority;
* issuer operationally `Ineligible`;
* issuer authority not established.

The resolution layer MAY internally distinguish these conditions.

However, the Gate 5 admission boundary MUST map all such authorization failures to:

`RegistryError::UnauthorizedCapabilityIssuer`

unless SPEC-REG-001-A03 is explicitly amended.

The resolver SHALL NOT substitute ambient or current state when referenced authoritative state cannot be resolved.

**Failure to prove authority is equivalent to absence of authority for admission purposes.**

**Unverifiable authority = no authority.**

## 10. Relationship to Gate 3B Identity Resolution

This specification does not modify the Gate 3B `IdentityResolver` contract.

`ResolvedIdentity` SHALL remain limited to the structural identity-resolution semantics allocated by SPEC-REG-001-A03B.

Gate 5 operational or authorization state SHALL NOT be added implicitly to `ResolvedIdentity`.

Gate 5 SHALL NOT require `IdentityRecord` to gain:

* operational status;
* revocation state;
* trust state;
* issuer-role state;
* authorization flags.

Any future extension of canonical identity records requires separate specification.

The successful completion of Gate 3B establishes identity existence only.

It does not establish operational eligibility or issuer authority.

## 11. Relationship to Existing Node Operational Status

The existing `registry-service::OperationalStatus` model:

* `Initializing`
* `Active`
* `Dormant`
* `Terminated`

is a node and workload-management lifecycle associated with `NodeRecord` and `Uuid`.

It is not the authoritative SOL identity-state contract defined by this specification.

Gate 5 SHALL NOT interpret:

`OperationalStatus::Active`

as equivalent to:

`Eligible`

and SHALL NOT derive Capability V1 issuer authority from `registry-service` node status or node capability strings.

Any future integration between compute-node lifecycle state and cryptographic identity authority requires a separately specified mapping.

No such mapping is established by this specification.

## 12. Relationship to Gate 6

Issuer-state resolution SHALL NOT evaluate `governing_policy`.

It SHALL NOT determine whether the issuer may issue the exact Capability V1 grant represented by the candidate payload.

It SHALL NOT evaluate:

* target-specific authorization;
* resource-specific authorization;
* executable-specific authorization;
* operation-specific policy permission;
* policy predicates;
* dynamic grant conditions.

Gate 5 establishes general issuer competency only.

Gate 6 remains solely responsible for determining whether the referenced governing policy authorizes the exact requested grant.

## 13. Implementation Constraints

A conforming implementation MUST preserve the following properties:

1. explicit authoritative state input;
2. deterministic resolution;
3. replay against the original authoritative state relationship;
4. separation of identity existence from issuer activity and authority;
5. separation of issuer competency from exact policy authorization;
6. fail-closed behavior when authoritative state cannot be proven;
7. no implicit dependency on unrelated node-management lifecycle state;
8. no ambient current-state fallback.

A conforming implementation MAY introduce a dedicated Rust trait, state-reference marker trait, or minimal result type if required.

Any such concrete API MUST preserve the semantics defined here and SHALL NOT broaden the authority surface beyond this specification.

## 14. Non-Goals

This specification explicitly does not define:

* the storage schema for issuer authority;
* the persistence model for issuer-state references;
* the internal lifecycle model from which `Eligible` or `Ineligible` may be derived;
* an `Active`, `Suspended`, `Revoked`, or equivalent canonical identity-state enum;
* delegation chains;
* role inheritance;
* organizational authority models;
* trust scoring;
* reputation scoring;
* key rotation semantics;
* identity revocation semantics outside their effect on an authoritative Gate 5 determination;
* policy-object evaluation;
* target-specific authorization;
* resource-specific authorization;
* subject operational state;
* subject revocation checks;
* `registry-service::OperationalStatus` integration;
* a required Rust trait name or concrete API shape.