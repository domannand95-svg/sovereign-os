# A05-BETA-001 — Capability Adversarial Corpus & Acceptance Harness

**Document ID:** A05-BETA-001

**Artifact Type:** Normative Acceptance Contract

**Document Status:** RECONSTRUCTED AND RE-RATIFIED

**Original Contract Status:** Verbatim Stage 1 text unavailable

**Reconstruction Basis:** Preserved engineering record, implemented harness behavior,
accepted Stage 3–5 dispositions, registered gaps A05B-GAP-001 through A05B-GAP-014,
and A05-BETA-001 Stage 6 Acceptance Report

**Baseline Commit:** `1d57f79e759c60c5f60d1e2fb5b01f26844d86d7`

**Engineering Checkpoint:** `5735ea8`

**Branch:** `agent/a05-adversarial-capability-harness`

**Issue #174:** PARKED — no production orchestration authorized

**Normative Boundary:**
> Adversarial evaluation of authority ≠ production orchestration of authority.

---

## 1. Reconstruction Notice

An original Stage 1 contract existed and was treated as frozen during the design phase of this branch. The verbatim text could not be recovered for the final documentation commit. This artifact is reconstructed from preserved architectural decisions, the empirical implementation, and the final Acceptance Report. It is being re-ratified now as the governing contract for the completed branch. It does not retroactively claim to reproduce the lost text word-for-word.

## 2. Purpose

A05-BETA-001 defines a deterministic, read-only adversarial evaluation layer for existing Capability V1 validation semantics. It evaluates already-allocated authority boundaries and does not create new ones.

## 3. Governing Principle

The Sovereign Operating Layer adheres to:

> **Probabilistic intelligence; deterministic authority.**

The specific invariant governing this phase is:

> **Adversarial evaluation of authority ≠ production orchestration of authority.**

## 4. Scope

The following are explicitly allowed within this phase:

* Test-only Rust implementation.
* Canonical Capability V1 decoding.
* Utilization of existing public validators.
* Injection of explicit state references.
* Deterministic adversarial fixtures.
* Deterministic replay.
* Error-to-test-oracle classification.
* Read-only resolver instrumentation.

## 5. Explicit Non-Goals

This phase shall NOT implement or execute:

* Capability exercise.
* Tool invocation.
* Ledger publication.
* Runtime delegation.
* Requester-binding implementation.
* Scope-comparison engine.
* State freshness engine.
* Revocation model.
* Composite orchestration pipeline.
* New `RecordKind` allocations.
* New Capability V1 fields.
* New production authority enums.

## 6. Harness Oracle

The harness utilizes exactly three classifications:

1. `APPROVED`
2. `FORBIDDEN`
3. `UNAVAILABLE`

These are harness-only classifications. They are not production authority enums and do not allocate runtime behavior.

## 7. Harness Gap

A `HarnessGap` is defined as:

> A failure of the harness to map an existing production result without inventing semantics.

`HarnessGap` is a test infrastructure error classification; it is NOT a fourth authority outcome.

## 8. Fail-Closed Mapping Rule

Unmapped production errors become `HarnessGap`, never inferred permission. The harness preserves the existing abstraction boundary:

* Issuer resolver failures collapsed by production to `UnauthorizedCapabilityIssuer` remain `FORBIDDEN`.
* Policy resolution/evaluation failures collapsed to `InvalidGoverningPolicy` remain `FORBIDDEN`.
* The harness must not reconstruct hidden causes behind those public errors.

## 9. Threat Classes

The required adversarial corpus consists of six classes:

1. Single-Agent Circumvention
2. Privilege Propagation
3. Collusive Bypass
4. Confused Deputy / Capability Laundering
5. Identity Substitution
6. State Replay / State Substitution

## 10. Actor Metadata Rule

Actor labels may describe an adversarial scenario, but they do not imply identity, authority, delegation, provenance, or runtime relationship unless production semantics consume equivalent context.

## 11. Fixture Requirements

Every deterministic fixture should identify, where applicable:

* Fixture ID
* Threat class
* Actors
* Requester metadata
* Governed identities
* Capability or capability reference
* Requested operation metadata
* Requested target/resources metadata
* Authoritative state reference
* Expected harness classification
* Expected production result/error
* Rationale
* Invariant exercised

Metadata that production APIs do not consume may document a scenario but may not participate in the authority decision.

## 12. State Reference Rule

> State references select authoritative evaluation context; they do not themselves establish freshness, ordering, or revocation.

`IdentityStateRef`, `IssuerStateRef`, and `PolicyStateRef` provide equality only. No latest, epoch, revision, generation, or ordering rule may be invented by the harness.

## 13. Replay Rule

Identical input combined with identical explicit state context must produce a deterministic result. The harness shall utilize:

* No ambient clock.
* No ambient "current" state.
* No fallback state.
* No implicit latest-state resolution.

## 14. Temporal Rule

Temporal evaluation uses the explicit production input (`admission_context_time: u64`) and never the ambient wall-clock time.

## 15. Identity Boundary

**Decidable:**

* Declared issuer/subject existence against explicit state.
* Identity-state unavailability.

**Undecidable (Requires Orchestration):**

* External requester matching the capability subject.
* Valid-identity substitution as a runtime caller-binding problem.

## 16. Issuer Boundary

Issuer eligibility and Capability V1 issuance authority are existing semantics. Resolver failures collapsed by production remain `UnauthorizedCapabilityIssuer`. The harness does not recover hidden unavailable or error states.

## 17. Governing Policy Boundary

* Policy resolution and evaluation are distinct.
* Both must use the same supplied `PolicyStateRef`.
* The exact declared policy CAID must be preserved.
* Policy payload language remains unallocated.
* The harness may not define new policy semantics.

## 18. Capability Coherence Boundary

The harness shall test the following existing decidable intrinsic contradictions:

* `Create` operation combined with an `ExactObject` target.
* Executable attached to a non-`Execute` operation.
* Network egress budget without network scope.
* Filesystem-write budget without write scope.

## 19. Reference Boundary

A governed reference is not a delegation edge. Capability references do not imply parent/child authority, transfer, delegation, or propagation unless separately specified.

## 20. Request-vs-Grant Boundary

**Deferred Gaps (001–003):**

* No requester binding.
* No requested-operation comparison.
* No requested-scope/resource/budget comparison.

The harness SHALL NOT implement these comparisons itself.

## 21. Multi-Agent Boundary

**Deferred Gaps (004–010): DEFERRED BY SPECIFICATION.**

No Rust fixtures should make authority decisions based on test-invented relationships such as "Agent A calls Agent B."

## 22. State Lifecycle Boundary

**Deferred Gaps (011–014):**

* No freshness ordering.
* No Capability V1 revocation lifecycle.
* Generic `Supersedes` is not authority revocation.
* No composite validator sequencing semantic.

## 23. Mutation and Side-Effect Prohibition

* Authoritative state must remain read-only.
* The candidate capability must not be mutated.
* No external side effects are permitted.
* Telemetry may record calls only.
* No capability execution is permitted.

## 24. Implemented Corpus

* `A05B-CIRC-001` through `A05B-CIRC-008` (Implemented)
* `A05B-STATE-001` through `A05B-STATE-004` (Implemented)
* Stage 4 Multi-Agent: No implementation (Deferred by specification)

## 25. Deferred Gap Registry

1. **A05B-GAP-001:** No current public API binds an external requester identity to the capability subject.
2. **A05B-GAP-002:** No current public API compares a requested operation against the granted payload.
3. **A05B-GAP-003:** No current public API compares requested scopes/resources/budgets against the granted payload.
4. **A05B-GAP-004:** No current public API binds an external requester/caller identity to `CapabilityPayloadV1::subject_identity()`.
5. **A05B-GAP-005:** Capability V1 contains no delegation field or parent-capability authority relationship.
6. **A05B-GAP-006:** No current public validator receives requester/deputy context sufficient to evaluate confused-deputy behavior.
7. **A05B-GAP-007:** No current production representation captures authority origin across a deputy or laundering chain.
8. **A05B-GAP-008:** No current public API evaluates composition of authority across multiple subjects or multiple capabilities.
9. **A05B-GAP-009:** No current representation records or validates multi-hop privilege propagation between agents.
10. **A05B-GAP-010:** Capability references are not specified as delegation edges or parent/child authority relationships.
11. **A05B-GAP-011:** `StateRef` contracts define equality only; no production freshness, ordering, epoch, or "latest state" relation exists.
12. **A05B-GAP-012:** Capability V1 defines no revocation lifecycle against authoritative state.
13. **A05B-GAP-013:** Generic Registry `Supersedes` edges are not specified as Capability V1 revocation, expiry, or authority-invalidating semantics.
14. **A05B-GAP-014:** No current production API defines a composite validation sequence whose reordering can be treated as an authority decision.

## 26. Issue #174 Isolation

Issue #174 remains outside the scope of A05-BETA-001. Findings produced by this harness do not authorize reopening, implementing, or partially implementing #174.

## 27. Reconstruction Ratification

This reconstructed contract is adopted as the authoritative repository record for the completed A05-BETA-001 phase.

It supersedes no recoverable verbatim document and makes no claim of textual identity with the lost Stage 1 artifact.

Its authority begins with this re-ratification and derives from consistency with:

1. the preserved A05-BETA-001 engineering record;
2. the implemented test-only harness;
3. the accepted Stage 3–5 dispositions;
4. A05B-GAP-001 through A05B-GAP-014;
5. the Stage 6 Acceptance Report.

Any future semantic expansion requires a separate authorized acceptance contract.

**Disposition:** CLEAR WITH DEFERRED SPECIFICATION GAPS

**Issue #174:** REMAINS PARKED