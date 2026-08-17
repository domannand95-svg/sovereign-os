# SPEC-REG-001-A06B

**Title:** Authoritative Governing-Policy Resolution and Evaluation Contract

**Document Status:** Approved / Locked

**Domain:** SOL Registry / Admission Pipeline / Gate 6

## 1. Scope

This specification defines the minimum implementation-facing contract for resolving and evaluating authoritative governing policies during Gate 6 admission. It bridges the normative Gate 6 requirements defined by SPEC-REG-001-A06 and the concrete implementation interfaces used by `sovereign-registry`.

This specification defines:

- the authoritative inputs required for Gate 6 policy resolution and evaluation;
- the strict contextual binding between the candidate's `governing_policy` CAID, the resolved policy object, the complete candidate grant, and the admission context;
- deterministic, storage-neutral policy resolution restricted strictly to `ObjectClass::Policy` records;
- the presentation of every semantically relevant field of the complete Capability V1 grant to the evaluator without projection, normalization, or widening;
- the logical binary authorization outcomes (`Authorized` / `NotAuthorized`);
- fail-closed handling for resolution and evaluation failures;
- deterministic replay semantics tied to the original admission context;
- the canonical mapping of all policy resolution, evaluation, and unverifiable authorization failures to `RegistryError::InvalidGoverningPolicy`.

This specification does not allocate a Policy V1 wire format, define a policy authoring DSL or rule execution language, introduce fallback policy mechanisms, or define inheritance, delegation, revocation, or supersession models.

---

## 2. Authority Boundary and Pipeline Context

Gate 6 operates under the invariant:

> **Identity existence ≠ identity activity ≠ issuer authority ≠ governing-policy authorization.**

The final admission boundary is strictly demarcated:

- **Gate 5:** establishes operational eligibility and general Capability V1 issuer competency.
- **Gate 6:** establishes exact governing-policy authorization for the specific Capability V1 candidate grant.

Passing Gate 5 SHALL NOT imply Gate 6 authorization.

Gate 6 is the final allocated authorization boundary of the Capability V1 admission pipeline.

---

## 3. Exact Governing-Policy Binding

An authorization result is valid for Gate 6 only if it is deterministically bound to all of the following:

1. the exact candidate `governing_policy` `Caid`;
2. the exact registry object authoritatively resolved for that `Caid`;
3. the fact that the resolved object is admitted as `ObjectClass::Policy`;
4. the exact Capability V1 candidate grant;
5. the authoritative admission context under which resolution and evaluation occur.

The policy object presented for evaluation MUST be the exact object authoritatively resolved from the candidate's `governing_policy` `Caid`.

Resolution or evaluation against any substituted object is non-conforming.

The resolver and evaluator SHALL NOT:

- substitute an alternative policy `Caid`;
- consult a default policy;
- consult an ambient policy;
- consult a root policy as fallback;
- select a newer or latest policy;
- infer a policy through ancestry, provenance, ownership, or identity relationships;
- resolve policy state from an unversioned or mutable current-state lookup;
- accept an authorization decision from an evaluator or external oracle that is not deterministically bound to the candidate's exact declared `governing_policy`.

The content-addressed identity represented by the candidate's `governing_policy` `Caid` remains authoritative. This specification does not introduce an additional cryptographic verification protocol.

---

## 4. Authoritative Policy-State Input

Gate 6 resolution and evaluation MUST operate using an explicit authoritative policy-state reference supplied by the governing admission environment.

That reference MUST be deterministically bound to the same authoritative admission context used by the preceding admission gates.

The policy-state reference SHALL NOT be obtained from:

- current process state;
- mutable singleton state;
- ambient registry state;
- an unversioned database head;
- the current ledger head;
- a latest-policy lookup;
- wall-clock time;
- network time;
- environment variables;
- filesystem state;
- process defaults;
- discovery-based fallback.

The policy-state reference MAY share a concrete representation with another admission-context reference, including `IdentityStateRef`, or MAY use a distinct storage-neutral deterministic representation.

Sharing a concrete representation SHALL NOT imply semantic equivalence between identity state and policy state.

This specification does not require the policy-state reference to encode or expose:

- a ledger height;
- block height;
- state root;
- database revision;
- timestamp;
- storage location;
- backend identifier.

It requires only that the reference be deterministic, authoritative, replayable, and bound to the same governing admission context.

---

## 5. Exact Capability V1 Grant Input

Gate 6 MUST evaluate the exact Capability V1 candidate grant.

The evaluator MUST receive, directly or through an equivalently lossless representation, every semantically relevant field of the candidate, including:

- `issuer_identity`;
- `subject_identity`;
- `operation`;
- `target`;
- `authorized_executable`, including its absence or presence;
- `resource_constraints`;
- `execution_budget`, including its absence or presence;
- `expiry`, including its absence or presence;
- the exact `governing_policy` `Caid`.

Gate 6 SHALL NOT authorize:

- a projection of the candidate;
- a reduced subset of candidate fields;
- a normalized substitute that changes candidate semantics;
- a widened target;
- a narrowed or widened resource grant;
- a substituted executable;
- a substituted issuer;
- a substituted subject;
- a substituted operation;
- a substituted governing policy;
- a different budget;
- a different expiry;
- an omitted optional field whose absence or presence is semantically relevant.

Gate 6 authorizes the candidate as presented after successful completion of the preceding gates.

---

## 6. Storage Neutrality and Policy Object Constraint

The underlying persistence model for governing policies is unallocated by this specification.

Gate 6 policy resolution MUST target canonical admitted registry objects classified as:

```text
ObjectClass::Policy
```

`ObjectClass::Policy` is the authoritative class constraint for Gate 6 policy resolution.

The implementation SHALL NOT substitute:

- `IdentityKind::Policy`;
- legacy `RegistryNodeType` values;
- arbitrary registry payloads;
- identity records;
- genesis records;
- provenance parents;
- unrelated policy-like objects.

The existence of a policy-related identity, node, record, or provenance relationship does not establish that the candidate's governing policy resolves to an admitted `ObjectClass::Policy`.

---

## 7. Logical Separation of Resolution and Evaluation

Gate 6 logically comprises two distinct determinations:

1. authoritative policy resolution;
2. exact-grant policy evaluation.

These operations are logically distinct for conformance purposes.

This specification does not require them to be implemented as separate:

- Rust traits;
- Rust types;
- modules;
- crates;
- processes;
- services;
- storage layers;
- runtime components.

Concrete implementation names such as `PolicyResolver`, `PolicyEvaluator`, `ResolvedPolicy`, or equivalent remain unallocated.

---

## 8. Policy Resolution Outcome

Policy resolution succeeds only if the implementation can deterministically establish, within the authoritative admission context, that:

1. the candidate's exact `governing_policy` `Caid` resolves authoritatively;
2. the resolved object is the exact object associated with that `Caid`;
3. the resolved object is admitted as `ObjectClass::Policy`.

Resolution fails if:

- the exact `Caid` cannot be authoritatively resolved;
- the authoritative state required for resolution is unavailable;
- the resolved object's class cannot be established;
- the resolved object is not `ObjectClass::Policy`;
- resolution requires ambient, current, inferred, or fallback state;
- the result cannot be deterministically reproduced.

This specification does not define a Policy payload schema and therefore does not allocate generic Policy-payload structural validity rules.

Any later evaluator-specific inability to interpret or evaluate an admitted Policy object is an evaluation failure, not a Policy V1 parsing rule defined by A06B.

---

## 9. Policy Authorization Outcome

A successfully completed policy evaluation yields exactly one logical authorization result:

- `Authorized`
- `NotAuthorized`

These are normative logical outcomes only.

This specification does not require a specific internal Rust enum or representation.

Gate 6 succeeds only when:

1. authoritative resolution succeeds;
2. the resolved object is `ObjectClass::Policy`;
3. the evaluator is deterministically bound to the exact resolved policy object, exact candidate grant, and authoritative admission context;
4. evaluation completes successfully;
5. the result is explicitly `Authorized`.

`NotAuthorized` fails Gate 6.

Failure to obtain a valid authorization result is not a third authorization state.

An evaluator error, unavailable evaluator, unsupported condition, indeterminate result, or otherwise unverifiable result is an evaluation failure and fails closed.

---

## 10. No Subsumption or Grant Approximation

A06B SHALL NOT assume that authorization of any different grant implies authorization of the candidate grant.

This includes authorization of a grant that is:

- broader;
- narrower;
- ancestral;
- descendant;
- related through provenance;
- related through ownership;
- related through identity;
- similar in operation;
- similar in target;
- similar in resource constraints;
- otherwise semantically adjacent.

No wildcard, subset, superset, inheritance, implication, delegation, ancestry, precedence, or substitution semantics are allocated by this specification.

Any such relationship requires separately specified policy semantics.

---

## 11. Determinism Requirements

For identical:

- Capability V1 candidate input;
- `governing_policy` `Caid`;
- authoritative policy-state reference;
- authoritative admission context;
- resolved Policy object;
- governing policy semantics;

Gate 6 MUST produce the same resolution and authorization result.

Gate 6 SHALL NOT depend on:

- wall-clock time;
- randomness;
- network responses;
- machine-local state;
- scheduler timing;
- workload;
- model judgment;
- mutable process memory;
- ambient filesystem state;
- current database state;
- current registry head;
- current policy discovery.

Any future external input required by policy semantics MUST itself be explicitly incorporated into a deterministic authoritative admission context before it may participate in Gate 6.

---

## 12. Replay Requirements

Historical replay MUST use the original authoritative admission context associated with the original admission decision.

Replay MUST use the exact authoritative policy-state reference associated with that context.

Replay SHALL NOT substitute:

- current registry state;
- current policy state;
- latest policy state;
- newly available policy objects;
- newer resolution results;
- ambient policy state;
- default policy state.

Later registry state, later policy resolution results, or any subsequently available policy object or policy-related state MUST NOT replace the original authoritative context during historical replay.

For identical original inputs and authoritative context, replay MUST reproduce the original Gate 6 authorization result.

---

## 13. Separation from Gate 4 Temporal Validation

Gate 4 remains authoritative for Capability V1 expiry validity.

Gate 6 SHALL NOT redefine:

- current admission time;
- expiry comparison rules;
- clock semantics;
- grace periods;
- skew tolerance;
- temporal sentinels.

The exact `expiry` field, including its absence or presence, remains part of the exact Capability V1 candidate presented to Gate 6 because it is a semantically relevant component of the candidate grant.

A06B does not allocate additional temporal semantics for governing policies.

---

## 14. Separation from Gate 5 Issuer Authorization

Gate 5 establishes issuer operational eligibility and general Capability V1 issuer competency.

Gate 6 establishes exact governing-policy authorization for the specific candidate grant.

The following implication is invalid:

```text
Gate 5 passed
    ≠
exact candidate grant authorized
```

Gate 6 SHALL NOT treat issuer competency as governing-policy authorization.

Gate 6 SHALL NOT silently bypass policy evaluation because an issuer is otherwise authorized to issue Capability V1 objects.

---

## 15. Existing `sovereign-policy` Boundary

Existing `sovereign-policy` directive-admission primitives are not automatically the canonical Gate 6 governing-policy evaluator.

In particular, the existence of types or traits such as:

- `DirectiveRequest`;
- `DirectivePolicy`;
- `PolicyDecision`;
- `PolicyDenial`;
- `EventTypeAllowlist`;

does not establish that those primitives define:

- the Capability V1 governing-policy payload schema;
- the authoritative Gate 6 evaluator interface;
- Capability V1 exact-grant semantics;
- Policy V1;
- a canonical policy rule language.

Any future integration between `sovereign-policy` and Gate 6 requires explicit specification.

A06B SHALL NOT infer that integration.

---

## 16. No Implicit Root-Policy Fallback

The existence of a genesis or registry field such as `root_policy_caid` does not authorize Gate 6 to replace the candidate's declared `governing_policy`.

If the candidate's exact governing-policy `Caid` cannot be authoritatively resolved and evaluated, Gate 6 fails closed.

A root policy SHALL NOT be used as:

- fallback;
- default;
- ancestor;
- override;
- implicit governing policy.

Any future root-policy semantics require separate specification.

---

## 17. No Implicit Lineage or Provenance Authorization

Registry provenance or lineage SHALL NOT imply:

- policy inheritance;
- policy supersession;
- authorization inheritance;
- delegation;
- policy substitution;
- policy ancestry authorization;
- policy priority.

A provenance relationship may exist independently of authorization semantics.

A06B allocates no policy-lineage behavior.

---

## 18. Failure and Unavailability Semantics

Any failure to establish explicit authoritative Gate 6 authorization MUST fail closed.

This includes:

- the referenced `governing_policy` `Caid` cannot be authoritatively resolved;
- the required authoritative policy-state reference is unavailable;
- the policy-state reference is not bound to the governing admission context;
- the resolved object is not `ObjectClass::Policy`;
- the resolved object's class cannot be established;
- resolution is ambiguous or unverifiable;
- resolution depends on prohibited ambient or current state;
- the evaluator cannot be deterministically bound to the exact resolved policy object;
- the evaluator cannot be deterministically bound to the exact Capability V1 candidate;
- evaluation fails;
- evaluation is unavailable;
- evaluation is indeterminate;
- evaluation is unverifiable;
- the logical result is `NotAuthorized`.

At the public Gate 6 admission boundary, all such failures MUST map to:

```rust
RegistryError::InvalidGoverningPolicy
```

Gate 6 SHALL NOT expose these conditions publicly as:

- `UnresolvedCapabilityReference`;
- `UnauthorizedCapabilityIssuer`;
- `CapabilitySemanticViolation`;
- `IdentityNotFound`;
- `IdentityStateUnavailable`;
- a new policy-denial error variant;
- a new policy-resolution error variant;
- a new policy-evaluation error variant.

Internal implementation code MAY distinguish failure causes for diagnostics, testing, or internal control flow, provided those distinctions do not alter the canonical public Gate 6 failure mapping.

---

## 19. Unverifiable Authorization

The governing invariant is:

> **Unverifiable or denied policy authorization = no authorization.**

Only an explicit, deterministic, authoritative `Authorized` result permits Gate 6 to succeed.

Absence of evidence of denial is not authorization.

Resolver success alone is not authorization.

Policy existence alone is not authorization.

Correct Policy object class alone is not authorization.

Evaluator availability alone is not authorization.

Gate 5 issuer competency is not authorization.

---

## 20. Minimum Implementation-Facing Contract

A conforming Gate 6 implementation MUST provide sufficient implementation-facing machinery to accomplish all of the following without violating A06 or A06B:

1. receive the exact candidate `governing_policy` `Caid`;
2. receive or derive from an explicitly supplied governing admission context an authoritative policy-state reference;
3. authoritatively resolve the exact referenced registry object;
4. establish that the object is admitted as `ObjectClass::Policy`;
5. bind evaluation to that exact resolved policy object;
6. present the exact Capability V1 candidate grant without semantic loss;
7. bind evaluation to the authoritative admission context;
8. obtain a deterministic logical authorization result;
9. distinguish successful authorization from resolution or evaluation failure internally as needed;
10. collapse all non-authorized Gate 6 outcomes to `RegistryError::InvalidGoverningPolicy`;
11. preserve deterministic historical replay.

The implementation MAY combine resolution and evaluation behind a single interface if all required bindings remain explicit and testable.

The implementation MAY separate them across multiple internal interfaces if the exact policy object, exact candidate grant, and authoritative context remain deterministically bound across those boundaries.

A06B allocates behavior, not concrete Rust names.

---

## 21. Conformance Requirements

A Gate 6 implementation conforms to this specification only if all of the following hold:

1. The exact candidate `governing_policy` `Caid` is used.
2. No replacement policy `Caid` is selected.
3. Resolution uses an authoritative deterministic policy-state reference.
4. The policy-state reference is bound to the governing admission context.
5. Resolution does not use ambient current state.
6. The exact referenced object is resolved.
7. The resolved object is established as `ObjectClass::Policy`.
8. `IdentityKind::Policy` is not substituted for `ObjectClass::Policy`.
9. The complete semantically relevant Capability V1 candidate is presented to evaluation.
10. Optional candidate fields preserve their exact absence or presence.
11. Candidate fields are not silently widened, narrowed, removed, rewritten, or substituted.
12. Evaluation is bound to the exact resolved Policy object.
13. Evaluation is bound to the exact candidate grant.
14. Evaluation is bound to the authoritative admission context.
15. A successful evaluation yields only `Authorized` or `NotAuthorized`.
16. Evaluation failure is not treated as implicit authorization.
17. `NotAuthorized` fails Gate 6.
18. Unverifiable authorization fails Gate 6.
19. No grant-subsumption semantics are invented.
20. No default or root-policy fallback is used.
21. No policy-lineage or provenance authorization semantics are inferred.
22. Existing `sovereign-policy` directive primitives are not implicitly promoted into the Gate 6 policy system.
23. Gate 4 remains authoritative for expiry validation.
24. Gate 5 issuer competency is not treated as exact-grant authorization.
25. Historical replay uses the original authoritative context.
26. All public Gate 6 failures map to `RegistryError::InvalidGoverningPolicy`.
27. No Policy V1 payload schema, DSL, bytecode, or rule language is invented.

---

## 22. Non-Goals

This specification explicitly does not define:

- a Policy V1 wire format;
- canonical Policy payload field structure;
- policy payload decoding rules;
- a policy DSL;
- a policy expression language;
- a rule engine;
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
- wildcard matching;
- subset or superset authorization;
- policy priority;
- conflict resolution;
- multi-policy composition;
- quorum authorization;
- threshold authorization;
- role-based access control;
- attribute-based access control;
- discretionary access control;
- mandatory access control;
- model-based authorization;
- external identity-provider integration;
- runtime capability execution;
- runtime capability revocation;
- subject operational-state validation;
- executable runtime integrity;
- runtime resource enforcement;
- runtime execution-budget enforcement.

Such semantics require separate specifications.

---

## 23. Summary

A06B defines the minimum implementation-facing Gate 6 contract:

```text
exact Capability V1 candidate
        +
exact governing_policy Caid
        +
authoritative admission context
        +
authoritative policy-state reference
        ↓
resolve exact governing_policy Caid
        ↓
establish admitted ObjectClass::Policy
        ↓
bind resolved Policy object
        +
exact candidate grant
        +
authoritative admission context
        ↓
deterministic policy evaluation
        ↓
Authorized
        ↓
Gate 6 succeeds

anything else
        ↓
RegistryError::InvalidGoverningPolicy
```

A06B deliberately stops before defining what a Policy means internally.

It defines only how an authoritative Policy object, an exact Capability V1 candidate, and an authoritative deterministic admission context MUST meet at the Gate 6 authorization boundary.

No Policy V1 schema, DSL, rule engine, inheritance model, fallback mechanism, or runtime execution semantics are allocated by this specification.
