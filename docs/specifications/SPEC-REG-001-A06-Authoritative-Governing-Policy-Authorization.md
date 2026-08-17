# SPEC-REG-001-A06 — Authoritative Governing-Policy Authorization

**Status:** Approved / Locked
**Specification Family:** SPEC-REG-001
**Component:** Sovereign Registry
**Admission Stage:** Capability V1 Gate 6
**Depends On:** SPEC-REG-001-A03, SPEC-REG-001-A04, SPEC-REG-001-A05, SPEC-REG-001-A05B

---

## 1. Scope

This specification defines the Gate 6 admission contract for Capability V1 governing-policy authorization.

Gate 6 is responsible for determining whether the `governing_policy` referenced by a candidate Capability V1 artifact:

1. resolves authoritatively to an admitted Registry v2 object;
2. resolves specifically to `ObjectClass::Policy`; and
3. authorizes the exact Capability V1 grant represented by the candidate payload.

Gate 6 operates downstream of:

- Gate 1 structural decoding;
- Gate 2 internal semantic coherence;
- Gate 3A governed-reference validation;
- Gate 3B authoritative identity resolution;
- Gate 4 temporal validation; and
- Gate 5 authoritative issuer eligibility and issuer competency.

Gate 6 is the final authorization boundary allocated by the Capability V1 admission taxonomy defined in SPEC-REG-001-A03.

This specification defines the semantic authorization boundary only.

It does not define a concrete persistent Policy payload schema, policy language, policy DSL, bytecode format, JSON representation, evaluator implementation, storage engine, or runtime execution model.

---

## 2. Core Authorization Invariant

Gate 6 SHALL preserve the following distinctions:

> **Policy existence ≠ Policy class validity ≠ Policy evaluability ≠ Grant authorization.**

The existence of an admitted object at the `governing_policy` `Caid` SHALL NOT by itself authorize the candidate capability.

The fact that the referenced object is classified as `ObjectClass::Policy` SHALL NOT by itself authorize the candidate capability.

The fact that the referenced policy can be successfully resolved or evaluated SHALL NOT by itself authorize the candidate capability.

Authorization exists only when the authoritative governing policy establishes that the exact requested Capability V1 grant is permitted.

A successful Gate 5 result SHALL NOT be interpreted as authorization of the requested grant.

Gate 5 proves only:

- operational eligibility of the issuer; and
- general Capability V1 issuer competency.

Gate 6 alone proves whether that otherwise competent issuer is permitted, under the referenced governing policy, to issue this exact candidate grant.

---

## 3. Governing-Policy Reference

Every Capability V1 payload contains one required `governing_policy` field.

The field is an exact 32-byte `Caid`.

Gate 6 MUST evaluate the exact `Caid` encoded in the candidate Capability V1 payload.

Gate 6 SHALL NOT:

- replace it with a root policy;
- replace it with a newer policy;
- replace it with the current policy;
- infer another policy through naming, ancestry, configuration, environment state, or mutable runtime state;
- select a policy according to implementation preference;
- follow an implicit fallback chain; or
- silently substitute another policy when the referenced object cannot be resolved.

The referenced `governing_policy` is part of the candidate grant and therefore part of the exact authorization request.

---

## 4. Authoritative Policy Resolution

Gate 6 MUST establish that the exact `governing_policy` `Caid` resolves to an admitted Registry v2 object of:

```text
ObjectClass::Policy
```

Resolution MUST be authoritative and deterministic.

A conforming implementation MUST NOT treat any of the following as sufficient substitutes for authoritative policy resolution:

- ambient in-memory state;
- the current Registry head;
- the latest known Policy object;
- the latest version of an identified policy family;
- current database state not bound to the admission context;
- current wall-clock time;
- an operating-system clock;
- a network time source;
- mutable singleton configuration;
- an unbound cache entry;
- a process-local default;
- an environment variable;
- a convenience fallback policy;
- a policy selected through runtime discovery.

If the referenced object:

- does not exist;
- cannot be resolved authoritatively;
- is unavailable in the required authoritative context;
- resolves only ambiguously; or
- is not `ObjectClass::Policy`;

Gate 6 MUST fail closed.

---

## 5. Authoritative Admission Context

Gate 6 policy resolution and evaluation MUST be bound to the same authoritative admission context as the preceding Capability V1 admission gates.

The implementation representation of that context is not defined by this specification.

The authoritative context MAY include independently represented references used by:

- Gate 3B identity resolution;
- Gate 4 admission-context time;
- Gate 5 issuer-state resolution; and
- Gate 6 policy resolution/evaluation.

These references need not:

- share a Rust type;
- share a storage representation;
- encode one another;
- be reducible to a single database identifier;
- be represented by a ledger height;
- be represented by a block hash;
- be represented by a state root;
- be represented by a timestamp.

However, all such references MUST belong to, or be deterministically bound to, the same authoritative admission context.

Gate 6 MUST NOT evaluate a policy against a different or later authoritative state than the state governing the rest of the candidate's admission.

---

## 6. Exact-Grant Authorization

Gate 6 MUST determine whether the referenced governing policy authorizes the exact Capability V1 grant represented by the candidate payload.

The authorization request includes the complete semantically relevant Capability V1 grant.

At minimum, exact-grant evaluation MUST preserve the values and absence/presence states of:

- `issuer_identity`;
- `subject_identity`;
- `operation`;
- `target_scope`;
- `authorized_executable`;
- `resource_constraints`;
- `execution_budget`;
- `expiry`; and
- `governing_policy`.

Gate 6 MUST NOT authorize a different grant and treat that result as authorization of the candidate.

In particular, the evaluator SHALL NOT silently:

- broaden an exact target into a named or global target;
- replace a named scope with another scope;
- widen filesystem access;
- widen network access;
- replace a governed namespace with an unrestricted path;
- replace a governed service with an unrestricted endpoint;
- add an executable where none was specified;
- remove an executable restriction;
- increase any execution budget;
- remove a finite execution budget;
- extend or remove an expiry;
- substitute a different issuer;
- substitute a different subject;
- substitute a different operation;
- substitute a different governing policy.

Authorization of a superset SHALL NOT automatically imply authorization of the exact candidate unless the governing policy's own semantics explicitly establish that relationship.

Gate 6 itself SHALL NOT invent grant-subsumption rules.

---

## 7. Upstream Validation Reliance

Gate 6 MAY rely on successful completion of the preceding Capability V1 gates.

It is not responsible for re-performing their full semantics.

In particular:

- Gate 1 is responsible for canonical structural decoding.
- Gate 2 is responsible for internal Capability V1 semantic coherence.
- Gate 3A is responsible for governed references allocated to that gate.
- Gate 3B is responsible for authoritative existence resolution of issuer and subject identities.
- Gate 4 is responsible for candidate temporal validity.
- Gate 5 is responsible for issuer operational eligibility and general Capability V1 issuer competency.

Gate 6 SHALL NOT reinterpret a successful upstream result as broader authority.

For example:

- Gate 3B identity existence does not establish policy authorization.
- Gate 4 temporal validity does not establish policy authorization.
- Gate 5 issuer competency does not establish exact-grant authorization.

Gate 6 owns only the final governing-policy authorization decision.

---

## 8. Temporal Semantics

Gate 4 remains the authoritative admission gate for Capability V1 `expiry` validity.

Gate 6 SHALL NOT redefine the Gate 4 expiry rule.

The Gate 4 rule remains:

```text
expiry absent  -> no Capability V1 expiry constraint
expiry present -> admission_context_time < expiry
```

Nevertheless, the exact `expiry` field, including its absence or encoded finite value, remains part of the candidate Capability V1 grant and MUST remain available to governing-policy evaluation.

A governing policy MAY distinguish between grants with different expiry values if a later policy-semantics specification allocates such behavior.

This specification does not define those policy rules.

Gate 6 SHALL NOT obtain a new ambient time source for policy evaluation.

---

## 9. Policy Decision Semantics

The minimum logical Gate 6 authorization result is:

```text
Authorized
NotAuthorized
```

Equivalent implementation representations MAY be used.

This specification does not require these logical outcomes to be represented as:

- a Rust enum;
- a boolean;
- a persistent field;
- a wire value;
- a ledger event;
- a database column.

`Authorized` means that authoritative policy evaluation has positively established authorization for the exact candidate Capability V1 grant.

`NotAuthorized` includes any policy decision that does not positively establish such authorization.

Absence of an explicit authorization result SHALL NOT be treated as authorization.

Unknown, indeterminate, unresolved, unavailable, malformed, unsupported, or otherwise unverifiable policy outcomes MUST fail closed at the Gate 6 admission boundary.

The governing rule is:

> **Unverifiable policy authorization = no authorization.**

---

## 10. Determinism

For the same:

- exact candidate Capability V1 payload;
- exact governing-policy `Caid`;
- authoritative policy state;
- authoritative admission context; and
- policy-evaluation semantics;

Gate 6 MUST produce the same logical authorization result.

Policy evaluation SHALL NOT depend on uncontrolled nondeterministic inputs.

A conforming implementation MUST NOT make the Gate 6 result depend on:

- current wall-clock time;
- random values;
- current network state;
- external service responses not fixed by the authoritative admission context;
- model-generated judgment;
- mutable process-local state;
- scheduler state;
- workload placement;
- current machine identity;
- filesystem contents outside an authoritative governed input;
- runtime environment variation.

Any external or contextual input required by a future governing-policy semantics specification MUST itself be represented through deterministic authoritative context before it can affect Gate 6.

---

## 11. Replay Semantics

Historical replay MUST evaluate Gate 6 using the original authoritative admission-context relationship.

A later policy state SHALL NOT retroactively alter the Gate 6 result for a previously admitted candidate when replaying that historical admission.

Replay SHALL NOT substitute:

- the current governing policy;
- the latest version of the governing policy;
- current Registry state;
- current issuer authority;
- current wall-clock time;
- current policy-engine configuration.

Given the same authoritative historical inputs, replay MUST reproduce the same Gate 6 logical result.

This requirement does not prohibit a separately specified revocation or supersession mechanism from affecting future operational use.

No such mechanism is allocated by this specification.

---

## 12. Policy Object versus Policy Evaluator

An admitted `ObjectClass::Policy` object and the mechanism that evaluates its semantics are distinct concepts.

This specification requires authoritative resolution of the referenced Policy object and deterministic evaluation of its authorization semantics.

It does not require that:

- policy evaluation occur inside `sovereign-registry`;
- policy evaluation occur inside `sovereign-policy`;
- the Registry object itself be executable;
- the Policy payload contain bytecode;
- the Policy payload contain a rule DSL;
- the Policy payload contain serialized Rust data;
- the Policy payload contain JSON;
- the Policy payload contain an allowlist.

A later specification MUST define any concrete Policy payload schema or executable policy semantics before such a representation becomes canonical.

---

## 13. Existing `sovereign-policy` Boundary

The current `sovereign-policy` crate contains deterministic directive-admission primitives including:

- `DirectiveRequest`;
- `DirectivePolicy`;
- `PolicyDecision`;
- `PolicyDenial`; and
- `EventTypeAllowlist`.

Those primitives evaluate directive admission in terms of ledger `EventType` and directive payload bytes.

This specification does not define those existing primitives as the canonical representation or evaluator for Capability V1 `governing_policy`.

A conforming Gate 6 implementation SHALL NOT assume that:

```text
DirectivePolicy == Capability V1 governing-policy evaluator
```

unless a later specification explicitly establishes such integration.

Similarly:

```text
EventTypeAllowlist != Capability V1 governing-policy schema
```

under this specification.

Existing directive-admission policy behavior remains unchanged.

---

## 14. Policy Identity and Policy Class

Gate 6 resolves the `governing_policy` by `Caid`.

A Registry `ObjectClass::Policy` object and an `IdentityKind::Policy` identity are separate concepts.

This specification does not require a governing Policy object to possess an `IdentityId`.

It does not define:

- Policy-object ownership;
- Policy identities;
- Policy signing identities;
- Policy authorship;
- Policy controllers;
- Policy delegates;
- Policy revocation identities.

The authoritative object-class requirement is specifically:

```text
governing_policy Caid -> admitted ObjectClass::Policy
```

No substitution through `IdentityKind::Policy` is permitted.

---

## 15. Failure Mapping

All Gate 6 admission failures MUST map at the public Registry admission boundary to:

```text
RegistryError::InvalidGoverningPolicy
```

This includes, at minimum:

- governing-policy object not found;
- governing-policy object unavailable in authoritative context;
- governing-policy resolution failure;
- governing-policy object is not `ObjectClass::Policy`;
- policy payload or semantic representation cannot be evaluated;
- policy evaluator failure;
- policy returns `NotAuthorized`;
- policy authorization is indeterminate;
- policy authorization cannot be established;
- exact-grant comparison cannot be completed deterministically.

Implementations MAY internally distinguish these conditions.

They SHALL NOT expose those distinctions as alternative successful admission paths.

Gate 6 MUST NOT map governing-policy authorization failures to:

- `UnresolvedCapabilityReference`;
- `UnauthorizedCapabilityIssuer`;
- `CapabilitySemanticViolation`;
- `IdentityNotFound`;
- `IdentityStateUnavailable`;

at the final Gate 6 public admission boundary.

The canonical Gate 6 failure is:

```text
InvalidGoverningPolicy
```

---

## 16. Gate Ordering

The Capability V1 admission sequence remains:

```text
Gate 1  Structural decoding
Gate 2  Internal semantic coherence
Gate 3A Governed-reference validation
Gate 3B Authoritative identity resolution
Gate 4  Temporal validation
Gate 5  Issuer operational eligibility and issuer competency
Gate 6  Governing-policy exact-grant authorization
```

Gate 6 MUST NOT be used to bypass or replace an earlier gate.

Likewise, an earlier gate MUST NOT perform Gate 6 authorization implicitly.

The separation is normative.

---

## 17. No Implicit Root-Policy Override

Registry Genesis MAY contain a `root_policy_caid`.

This specification does not define the Genesis root policy as an automatic replacement for a Capability V1 candidate's explicit `governing_policy`.

Gate 6 SHALL NOT silently authorize a candidate against the Genesis root policy when the candidate references another policy.

Any hierarchy, delegation, inheritance, chaining, parent-policy, root-policy, or constitutional-policy semantics require a separate specification.

Until such semantics exist:

```text
candidate.governing_policy
```

is the exact Policy object Gate 6 must resolve and evaluate.

---

## 18. No Implicit Policy Lineage Semantics

Registry provenance parents and graph lineage do not by themselves establish policy inheritance or authorization inheritance.

Gate 6 SHALL NOT infer that:

- a parent Policy authorizes its child;
- a child Policy inherits its parent's authority;
- a newer Policy supersedes an older Policy;
- a Policy lineage forms an override chain;
- a Policy provenance edge constitutes delegation.

Registry provenance establishes provenance according to the Registry specifications.

It does not establish authorization inheritance unless a later specification explicitly allocates such semantics.

---

## 19. Policy Payload Schema Remains Unallocated

This specification intentionally does not allocate a canonical persistent payload schema for `ObjectClass::Policy`.

No canonical meaning is assigned here to any arbitrary existing Policy payload bytes.

In particular, this specification does not define:

- a Policy V1 binary layout;
- minimum or maximum Policy payload length;
- Policy field tags;
- rule ordering;
- policy rule precedence;
- condition encoding;
- wildcard syntax;
- capability-pattern syntax;
- subject selectors;
- issuer selectors;
- target selectors;
- resource selectors;
- executable selectors;
- budget selectors;
- time-window selectors;
- delegation encoding;
- signature encoding.

A subsequent specification MUST define such semantics before arbitrary Policy payload bytes can be interpreted canonically for Gate 6 authorization.

---

## 20. Implementation-Facing Resolution Contract

A subsequent specification in the SPEC-REG-001 family SHOULD define the minimum implementation-facing contract required to connect Gate 6 with authoritative Policy resolution and evaluation.

That contract should define, without prematurely fixing storage representation:

- authoritative policy-state or policy-context reference requirements;
- deterministic resolution inputs;
- exact candidate grant input;
- minimum logical authorization result;
- resolver/evaluator failure handling;
- replay requirements;
- public Gate 6 failure mapping;
- interaction boundary with `RegistryGraph`;
- interaction boundary with any future Policy payload decoder or policy engine.

This specification does not allocate concrete Rust trait names or concrete implementation types for that contract.

---

## 21. Non-Goals

This specification does not define:

- a concrete Policy payload encoding;
- Policy V1;
- a policy DSL;
- a rule engine;
- an expression language;
- policy bytecode;
- policy scripting;
- policy compilation;
- policy signing;
- policy ownership;
- policy delegation;
- policy revocation;
- policy supersession;
- policy inheritance;
- policy ancestry semantics;
- policy priority;
- conflict resolution between multiple policies;
- multi-policy composition;
- threshold authorization;
- quorum authorization;
- external identity-provider integration;
- role-based access control;
- attribute-based access control;
- discretionary access control;
- mandatory access control;
- trust scoring;
- model-based authorization;
- runtime capability execution;
- capability revocation after admission;
- subject operational-state validation;
- executable runtime integrity;
- runtime budget enforcement.

Such features require separate specifications.

---

## 22. Conformance Requirements

A Gate 6 implementation conforms to this specification only if it preserves all of the following:

1. The exact candidate `governing_policy` `Caid` is used.
2. The referenced object must be authoritatively resolvable.
3. The referenced object must be `ObjectClass::Policy`.
4. Passing Gate 5 is not treated as exact-grant authorization.
5. The exact Capability V1 grant is evaluated.
6. Candidate fields are not silently widened, substituted, removed, or rewritten for authorization.
7. Gate 4 remains authoritative for expiry validity.
8. Gate 6 uses no ambient or current-policy fallback.
9. Evaluation is deterministic.
10. Replay uses the original authoritative context.
11. Existing `sovereign-policy` directive primitives are not implicitly treated as the Capability V1 policy schema.
12. `IdentityKind::Policy` is not substituted for `ObjectClass::Policy`.
13. Genesis root policy is not an implicit fallback.
14. Registry lineage does not imply authorization inheritance.
15. Unverifiable authorization fails closed.
16. All public Gate 6 failures map to `RegistryError::InvalidGoverningPolicy`.
17. No canonical Policy payload schema is invented by the Gate 6 implementation absent a subsequent specification.

---

## 23. Summary

Gate 6 is the authoritative final authorization boundary for Capability V1 admission.

Its responsibility is narrowly defined:

```text
exact governing_policy Caid
        +
authoritative admission context
        +
exact Capability V1 candidate grant
        ↓
resolve admitted ObjectClass::Policy
        ↓
deterministically evaluate exact grant
        ↓
Authorized
or
InvalidGoverningPolicy
```

Gate 6 SHALL remain fail-closed, deterministic, replayable, and separate from issuer competency, directive admission, runtime execution, and any unallocated future policy-language semantics.
