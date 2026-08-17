# SOL-REG-INTEGRATION-002 — Authoritative Admission Context Persistence and Replay Contract

**Baseline:** `main@e470b25`
**Status:** DRAFT / SPECIFICATION ALLOCATION
**Domain:** Registry v2 Ledger Projection and Replay Architecture
**Document Authority:** NON-AUTHORITATIVE UNTIL LIFECYCLE PROMOTION

---

## 1. Purpose and Scope

This specification defines the canonical persistent mechanisms required to durably identify historical admission context, bind Capability V1 candidates to that context, reconstruct exact historical inputs during deterministic replay, and ensure policy-evaluation semantics remain replay-stable.

While structural validation (Gates 1 through 3A) operates on canonical candidate and graph information, Gates 3B through 6 explicitly depend on external, authoritative state references. This document serves as the foundational contract guaranteeing that those external dependencies are durably recoverable to support deterministic historical reconstruction.

### 1.1. Explicit Non-Goals (The Isolation Firewall)

This contract allocates persistence and replay mechanics **only**. It explicitly prohibits the allocation, design, or inference of:

- Runtime capability exercise, execution, or runtime tool invocation.
- Capability revocation or capability supersession semantics.
- State freshness, time-to-live (TTL), or "latest-state" evaluation overrides.
- Delegation, confused-deputy handling, or authority propagation.
- Requester, caller, or identity-binding mechanics for active sessions.
- Runtime execution budget or resource limit enforcement.
- Any mechanisms previously parked under Issue #174.

The governing invariant remains:

> **Historical admission evidence reconstruction does not grant or establish current runtime execution authority.**

---

## 2. The Authoritative Admission Context

The Authoritative Admission Context is the logically unified set of external authoritative dependencies required to evaluate a Capability V1 candidate whose identity and association MUST remain durably recoverable.

A raw `CapabilityPayloadV1` is insufficient to reconstruct historical admission for Gates 3B through 6.

The Authoritative Admission Context MUST constitute the following discrete components:

1. **`IdentityStateRef`:** The exact historical reference used by Gate 3B to resolve the `issuer_identity` and `subject_identity`.
2. **`admission_context_time`:** The exact authoritative temporal value provided by the admission environment for Gate 4 `expiry` evaluation.
3. **Issuer-State Reference:** The exact historical reference used by Gate 5 to establish operational eligibility and Capability V1 issuer authority. This may share a representation with `IdentityStateRef` or remain distinct, but its association MUST be explicitly recoverable.
4. **Policy-State Reference:** The exact historical reference used by Gate 6 to resolve the exact `governing_policy` `Caid` to a valid `ObjectClass::Policy` object.
5. **Policy-Evaluator Identity:** A deterministic identifier establishing the exact evaluation semantics applied during Gate 6 authorization.

These components MUST belong to, or be deterministically proven to belong to, the same authoritative admission context used for the original admission decision.

The persistence mechanism MUST NOT reconstruct an admission context by independently selecting otherwise-valid identity, issuer, policy, temporal, or evaluator inputs from different historical contexts.

---

## 3. Durably Recoverable Historical Inputs

The durable representation MUST contain, reference, or deterministically reconstruct all information required to reproduce the original authoritative admission decision.

It MUST NOT require substitution from mutable present-state authority sources.

During historical replay, the reconstruction mechanism MUST recover:

- The exact canonical bytes of the `CapabilityPayloadV1`.
- The exact authoritative `admission_context_time`.
- Sufficient authoritative reference information to deterministically recover the exact historical state required for identity, issuer, and policy evaluation, without requiring duplication of the complete referenced state body.
- The exact `PolicyEvaluatorIdentity`.

Replay logic MUST NOT attempt to substitute:

- current identity keys or identity state;
- current issuer state;
- current or latest policy state;
- current Registry head;
- current wall-clock time;
- current policy-engine configuration;
- present evaluator semantics; or
- any mutable ambient authority source

for the historical inputs used by the original admission decision.

---

## 4. Deterministic Context Binding

The components of the Authoritative Admission Context MUST NOT be persisted as disconnected variables.

They MUST be deterministically bound to the specific candidate they authorize.

This specification allocates the required binding but leaves the physical storage mechanism open to implementation.

Potential implementation families may include a ledger envelope, immutable referenced object, admission receipt, hybrid representation, or another mechanism that satisfies this contract.

No particular representation is selected here.

The normative model is:

```text
CapabilityPayloadV1
        +
AuthoritativeAdmissionContext
        |
        v
candidate/context binding
        |
        v
A03-A06B admission
        |
        v
durable historical admission representation
        |
        +-------------------------------+
        |                               |
        v                               v
live authoritative projection      later replay
                                        |
                                        v
                         recover exact candidate
                         recover exact historical refs
                         recover original context time
                         reproduce policy semantics
                         verify candidate/context binding
                                        |
                                        v
                           deterministic projection
```

The integration layer MUST guarantee that an admission context cannot be detached from one candidate and applied to another.

During replay, a candidate lacking a deterministically verified context binding MUST be rejected from authoritative projection.

The binding mechanism MUST remain deterministic and authoritative.

This contract does not require that the binding be specifically:

- cryptographic;
- structural;
- reference-based;
- receipt-based;
- state-root-based; or
- represented by any particular storage primitive.

---

## 5. Replay-Stable Governing-Policy Semantics

Gate 6 exact-grant authorization introduces a distinct historical dependency: the interpretation of the governing Policy object itself.

The same Policy object evaluated under different semantics may produce a different authorization result.

Such behavior would violate deterministic historical reconstruction.

### 5.1. Policy Evaluation Semantics Identity

A canonical immutable identifier MUST identify the deterministic Gate 6 evaluation semantics applicable to the historical admission.

The identifier MUST resolve, directly or indirectly, to semantics sufficient to reproduce the original evaluation.

Its concrete representation is not allocated by this specification.

A future implementation may choose an evaluator identifier, semantic profile identifier, schema identity, code identity, bytecode identity, specification identity, or another deterministic representation, provided that the resulting semantics are stable and replayable.

### 5.2. Historical Evaluator Availability

Historical evaluator semantics MAY be obsolete or deprecated for new live admissions.

Deprecation alone MUST NOT invalidate deterministic historical replay.

Replay fails only if the historically required semantics are unavailable or cannot be deterministically reproduced.

### 5.3. No Evaluator Fallback

Historical replay MUST NOT substitute:

- the current evaluator;
- the latest evaluator;
- a default evaluator;
- a semantically similar evaluator;
- an inferred evaluator; or
- an evaluator selected from ambient runtime configuration

for the exact historically applicable policy-evaluation semantics.

---

## 6. Historical Reconstruction Requirements

Historical replay MUST reconstruct the original admission relationship rather than perform a new admission decision against present state.

At minimum, replay MUST be able to establish:

1. the exact candidate being reconstructed;
2. the exact historical authoritative context used during admission;
3. the required historical identity-state dependency;
4. the original authoritative `admission_context_time`;
5. the required historical issuer-state dependency;
6. the required historical policy-state dependency;
7. the historically applicable policy-evaluation semantics; and
8. the deterministic binding proving that these inputs belong to the original admission decision.

Replay MUST NOT reinterpret historical admission through later:

- identity changes;
- issuer-state transitions;
- policy changes;
- evaluator changes;
- key rotation;
- revocation events;
- restorations;
- authority grants or removals;
- passage of time; or
- storage migration.

Those later changes may affect separate present-state or runtime semantics if such semantics are allocated in the future.

They MUST NOT rewrite the historical admission result.

---

## 7. Historical Admission Is Not Present-State Validity

This specification preserves a strict distinction between:

```text
Was this Capability valid and authorized under its original authoritative admission context?
```

and:

```text
Would this Capability be valid or exercisable under current state?
```

The first question belongs to deterministic historical reconstruction.

The second belongs to future lifecycle, revocation, freshness, runtime authority, or capability-exercise semantics and is explicitly outside this contract.

Therefore:

```text
historical admission
        !=
current validity
        !=
runtime exercise authority
```

---

## 8. Projection Is Not Admission Proof

A structurally valid Registry v2 record does not, by itself, prove completion of the Capability V1 authority pipeline.

Likewise, deterministic graph insertion does not independently establish authoritative admission.

The following remain distinct:

```text
wire decoding
        !=
historical admission proof

graph projection
        !=
Capability V1 authorization

stored capability
        !=
runtime permission
```

A future integration MUST preserve evidence sufficient to establish the historically authoritative admission relationship before projecting a Capability V1 record as authoritative state.

---

## 9. Failure and Unavailability Semantics

Historical reconstruction MUST remain fail-closed.

Failure to reconstruct or verify required historical admission context MUST prevent projection of the affected record and MUST prevent successful publication of an authoritative reconstructed engine state.

Replay reconstruction MUST fail if any required condition cannot be established, including:

- the candidate cannot be recovered exactly;
- the authoritative admission context is missing, malformed, incomplete, or ambiguous;
- the candidate/context binding cannot be deterministically verified;
- the original `admission_context_time` cannot be recovered;
- a required historical identity-state dependency cannot be deterministically resolved;
- a required historical issuer-state dependency cannot be deterministically resolved;
- a required historical policy-state dependency cannot be deterministically resolved;
- the historically applicable policy-evaluation semantics cannot be identified;
- the required historical evaluator semantics are unavailable or cannot be deterministically reproduced; or
- otherwise-valid context components cannot be proven to belong to the same original authoritative admission context.

Such a failure is a replay/reconstruction failure.

It MUST NOT be classified as durable-append ambiguity merely because historical reconstruction failed.

`CommitAmbiguous` remains reserved for uncertainty concerning whether a live durable append occurred.

Concrete `RegistryError` variants for historical admission-context restoration are deferred to implementation/API design unless a later lifecycle decision explicitly allocates them.

---

## 10. Durable Representation Requirements

The concrete persistence format remains unallocated, but any future representation MUST satisfy all of the following properties:

1. **Deterministic:** Identical durable evidence MUST reconstruct identically.
2. **Candidate-Bound:** Context evidence MUST NOT be reusable for a different candidate.
3. **Historically Exact:** Required historical references and temporal values MUST identify the original admission inputs.
4. **Same-Context Verifiable:** Cross-gate dependencies MUST be proven to belong to the same authoritative admission context.
5. **Evaluator-Stable:** The historically applicable Gate 6 semantics MUST remain deterministically reproducible.
6. **Replay-Independent:** Reconstruction MUST NOT depend on present mutable authority state.
7. **Fail-Closed:** Missing or unverifiable historical evidence MUST prevent authoritative projection.
8. **Execution-Neutral:** The durable representation MUST NOT itself constitute runtime capability exercise authority.

---

## 11. Unallocated Physical Representation

This specification intentionally does not yet require that historical admission evidence be represented as:

- fields embedded directly in the Registry ledger event;
- a separate immutable Registry object;
- an admission receipt;
- a state root;
- a snapshot identifier;
- a block identifier;
- a ledger position;
- a cryptographic commitment;
- a structural composite;
- a dedicated event type; or
- any particular hybrid of these mechanisms.

A later implementation design MUST choose a representation that satisfies this specification without weakening the locked A03–A06B contracts.

---

## 12. Implementation Boundary

This specification allocates persistence and replay requirements only.

It does not authorize production modifications to:

- `sovereign-registry`;
- `sovereign-engine`;
- `sovereign-policy`;
- `sovereign-ledger`; or
- any runtime execution subsystem.

Production integration MUST remain a separate engineering ticket with explicit acceptance criteria.

---

## 13. Explicit Non-Allocation of Runtime Exercise

Nothing in the Authoritative Admission Context grants permission to exercise a stored Capability.

Nothing in the durable historical admission representation grants permission to:

- invoke a tool;
- execute a process;
- use a credential;
- access a filesystem resource;
- perform a network operation;
- dispatch an agent;
- mutate external state; or
- perform another governed side effect.

The runtime exercise boundary remains independently unallocated.

---

## 14. Explicit Issue #174 Isolation

This specification MUST NOT be interpreted to allocate, resolve, partially implement, or establish precedent for semantics isolated under Issue #174.

Any Issue #174 acceptance contract remains a separate future phase.

---

## 15. Acceptance Properties for Lifecycle Promotion

Before this specification may be promoted from draft status, review MUST establish that the contract:

1. preserves all locked A03–A06B historical replay requirements;
2. does not require substitution from present mutable authority state;
3. preserves the same-authoritative-context relationship across Gates 3B–6;
4. preserves exact `admission_context_time`;
5. requires deterministic recovery of historical identity, issuer, and policy dependencies;
6. requires replay-stable governing-policy evaluation semantics;
7. does not equate evaluator deprecation with historical invalidity;
8. keeps replay failure distinct from `CommitAmbiguous`;
9. does not presuppose a physical persistence representation;
10. does not allocate runtime capability exercise;
11. does not allocate revocation, supersession, freshness, delegation, or requester/caller semantics; and
12. preserves Issue #174 isolation.

---

## 16. Governing Invariants

> **Historical admission evidence reconstruction does not establish current runtime authority.**

> **Historical admission is not present-state validity.**

> **Projection is not authorization.**

> **A stored capability is not a runtime permission check.**

> **Replay must reproduce original authority context, not substitute present authority state.**

> **Deprecated historical semantics remain valid when required for deterministic replay.**

> **Probabilistic intelligence; deterministic authority.**
