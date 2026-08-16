# Sovereign Operating Layer (SOL)

**Probabilistic intelligence; deterministic authority.**

Sovereign Operating Layer (SOL) is an experimental Rust control plane for governed agentic systems, reproducible evidence, bounded authority, deterministic admission, and progressively earned trust.

SOL does not attempt to make probabilistic models deterministic. Instead, it places deterministic, auditable, fail-closed authority around probabilistic components.

Models may reason, propose, classify, search, and request capabilities. The surrounding control plane determines what authority exists, which state is authoritative, what may be admitted, what may cause external effects, and what must be retained for audit and replay.

> **Intelligence may explore beyond the boundary. Effects may not.**

---

## Programme Scope and Naming

- **Repository / long-term programme:** `sovereign-os`
- **Currently implemented architecture:** Sovereign Operating Layer (SOL)
- **Future aspiration:** Sovereign Operating System, retained strictly as a
  future architectural horizon contingent upon owning sufficient execution,
  lifecycle, resource, isolation, recovery, and hardware-facing
  responsibilities

> The repository retains the historical `sovereign-os` name as the home of the
> broader programme. The currently implemented system is the Sovereign
> Operating Layer; "Sovereign Operating System" remains a possible future
> architectural horizon, not a claim about present functionality.

---

## Current Status

The currently implemented authority work comprises the allocated **Capability
V1 validation contracts for Gates 1-6**.

The implemented validation components are:

1. **Gate 1 - Structural Decoding**
2. **Gate 2 - Internal Coherence**
3. **Gate 3A - Registry Reference Resolution**
4. **Gate 3B - Authoritative Identity Resolution**
5. **Gate 4 - Deterministic Temporal Validation**
6. **Gate 5 - Issuer Operational Eligibility and Competency**
7. **Gate 6 - Authoritative Governing-Policy Authorization**

Each gate has deterministic, fail-closed implementation and tests. The
repository does not yet expose one production admission orchestrator that
invokes the complete sequence end to end. Until that integration exists, these
components must not be described as an active governed admission pipeline.

The broader runtime, orchestration, policy language, multi-agent ecosystem,
institutional interfaces, and distributed architecture remain under
development. Completion of these validation contracts must not be interpreted
as completion of an end-to-end admission path or of the entire Sovereign
Operating Layer.

---

## Core Architectural Principle

SOL separates intelligence from authority.

External or local models may remain probabilistic, adaptive, and provider-neutral. Authority is instead constrained through deterministic infrastructure:

- explicit schemas;
- canonical encoding;
- content-addressed objects;
- authoritative state references;
- fail-closed validation;
- bounded capabilities;
- deterministic replay;
- recorded provenance; and
- independently reviewable policy decisions.

The governing rule is:

> **Proposal is not authorization. Capability is not permission. Intelligence is not authority.**

Ambient authority is not inferred.

Missing, unresolved, substituted, stale, malformed, unauthorized, or context-disconnected inputs fail closed.

---

## Capability V1 Validation Boundary

Capability V1 is the currently implemented set of governed validation
contracts. The sections below describe their allocated order and individual
responsibilities, not a claim that the complete sequence is already wired into
one production admission path.

### Gate 1 - Structural Decoding

Candidate bytes must decode according to the allocated Capability V1 schema.

Malformed structure is rejected before semantic interpretation.

### Gate 2 - Internal Coherence

Decoded fields must satisfy their allocated semantic relationships and constraints.

Structurally valid but internally invalid candidates are rejected.

### Gate 3A - Registry Reference Resolution

Registry-backed references must resolve to admitted objects through the authoritative registry context.

Unresolved references fail closed.

The governing-policy reference is deliberately reserved for Gate 6 rather than treated as an ordinary Gate 3A reference.

### Gate 3B - Authoritative Identity Resolution

Issuer and subject identities are resolved against an explicit authoritative identity-state reference.

Ambient or implicitly current identity state is not substituted.

### Gate 4 - Deterministic Temporal Validation

Temporal admission uses explicit admission-context time rather than ambient wall-clock time.

Where an expiry is present, a candidate is valid only while:

```text
admission_context_time < expiry
```

Historical replay therefore evaluates against the original admission context rather than whatever time happens to be current during replay.

### Gate 5 - Issuer Operational Eligibility and Competency

The issuer must be operationally eligible and possess the authority required to issue Capability V1 grants.

Gate 5 establishes issuer competency. It does not itself authorize the exact requested grant.

### Gate 6 - Authoritative Governing-Policy Authorization

Gate 6 resolves the exact `governing_policy` CAID against an explicit authoritative policy-state reference.

The resolved object must:

- match the exact requested CAID;
- be an admitted Registry v2 `ObjectClass::Policy`; and
- authorize the complete, unchanged Capability V1 candidate.

Resolution and evaluation remain bound to the same authoritative policy-state reference.

Only an explicit authorized result succeeds.

Resolution failures, evaluator failures, policy substitution, wrong object class, or explicit non-authorization fail closed as:

```text
RegistryError::InvalidGoverningPolicy
```

No implicit root policy, current policy, latest policy, default policy, inherited grant, wildcard expansion, or ambient fallback is inferred by this boundary.

---

## Deterministic Replay

Replay is a core architectural requirement.

SOL does not define replay as "run the same model again and expect identical
prose." Probabilistic inference may remain nondeterministic.

Instead, deterministic authority requires that admission decisions be reproducible when supplied with the same:

- candidate bytes;
- authoritative registry state;
- identity-state reference;
- admission-context time;
- issuer-state reference;
- policy-state reference; and
- allocated validation rules.

The authority path should therefore reproduce the same admission result without silently consulting ambient current state.

---

## Governed Research and Evidence

SOL also contains the foundations of a governed research record layer.

Implemented work includes:

- the canonical A04 governed-evidence envelope;
- bounded Objective payload encoding;
- bounded Claim payload encoding;
- bounded Source payload encoding;
- bounded Method payload encoding;
- bounded Uncertainty payload encoding;
- bounded Failed Attempt payload encoding;
- bounded Reviewer Finding payload encoding;
- content-addressed records;
- deterministic encoding and decoding;
- kind-confusion protection;
- provenance-preserving structures; and
- audit-oriented validation boundaries.

The long-term research flow remains:

```text
Research objective
    -> capability-scoped work
    -> recorded sources, methods, tools, budgets, and uncertainty
    -> independent and adversarial review
    -> verified | disputed | rejected
    -> independently authorized promotion, remediation, or revocation
    -> reproducible evidence and audit history
```

Verified evidence may eventually support institution-controlled model development, but training does not confer trust or authority. Any resulting model re-enters the governed system as an untrusted candidate.

---

## Persistence and Canonical State

The repository also provides a deterministic single-node persistence baseline, including:

- append-only ledger records;
- checksums and strict sequence ordering;
- deterministic replay and state reconstruction;
- crash-atomic, explicitly versioned snapshots;
- snapshot integrity and state-root validation;
- legacy-format recovery;
- safe fallback from invalid or unsupported snapshots to ledger replay;
- content-addressed registry nodes;
- deterministic registry projection;
- deterministic directive admission;
- single-node boot and restart reconstruction;
- fail-closed handling of ambiguous persistence outcomes; and
- corruption and interrupted-publication tests.

Snapshots accelerate restoration but do not replace canonical ledger authority.

---

## Workspace

The authoritative root Cargo workspace currently contains seven crates:

```text
crates/
|-- sovereign-core-asm/   deterministic state and snapshot representation
|-- sovereign-ledger/     append, replay, restore, integrity, and snapshots
|-- sovereign-registry/   registry graph, governed references, identity and Capability V1 admission
|-- sovereign-policy/     deterministic directive-admission primitives
|-- sovereign-engine/     single-node boot and command orchestration
|-- sovereign-audit/      governed research and evidence records
`-- sovereign-discovery/  scaffold; not yet implemented
```

Older service prototypes and retired architectural experiments may remain in the repository for extraction, historical comparison, or adversarial test cases. They are not part of the active authority boundary unless included by the root workspace and current normative architecture.

---

## Important Policy Boundary

The existing `sovereign-policy` crate contains deterministic directive-admission primitives.

It is **not** currently the canonical Capability V1 governing-policy evaluator.

Gate 6 deliberately exposes a storage-neutral governing-policy resolution and evaluation boundary without inventing a Policy V1 payload schema, DSL, bytecode format, rule engine, delegation system, inheritance model, or implicit policy hierarchy.

Those semantics must be explicitly allocated before they can become authority.

---

## Verification

The repository pins its Rust toolchain.

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

Boundary-affecting changes should additionally be checked for deterministic replay, fail-closed behavior, substitution resistance, and authoritative-context binding.

GitHub Actions provides the repository CI boundary across supported environments.

---

## Private Beta Readiness

The deterministic Capability V1 authority core is substantially implemented.

The remaining work before promotion to a formal private-beta baseline is primarily integration and empirical verification rather than foundational admission design.

The current beta exit criteria are:

1. **Integrated commissioning path**
   Exercise the Knowledge Infrastructure Bootstrap Kit and Sovereign Operating Layer together in the actual beta workflow, with failed commissioning preventing promotion.

2. **Adversarial admission suite**
   Exercise malformed inputs, unresolved references, substituted objects, stale or mismatched context, unauthorized issuers, denied policy outcomes, evaluator failures, and other fail-closed paths across the complete admission chain.

3. **Replay equivalence**
   Demonstrate that historical admission decisions reproduce the same result when supplied with the original authoritative admission context and state references.

4. **Clean-environment commissioning**
   Demonstrate clean clone, build, test, validation, and beta execution on the intended target environment without undocumented local assumptions.

Completion of those criteria will establish:

> **Private Beta Baseline 1 - Governed Capability Admission & Commissioning**

This milestone means the governed Capability V1 admission and commissioning baseline has been validated for private beta. It does not mean the wider SOL ecosystem is complete.

---

## Knowledge Infrastructure Bootstrap Kit

The Knowledge Infrastructure Bootstrap Kit (BKI) is being developed as a commissioning and validation boundary around the repository and its governed knowledge artifacts.

Its role is to support deterministic validation, normalization, compatibility checks, quarantine, and commissioning evidence without silently acquiring authority to promote or mutate governed state.

Joint BKI-SOL integration and adversarial commissioning remain part of the
private-beta path.

---

## SOL Identity

SOL also has a non-normative identity layer derived from the architecture.

Primary identity axiom:

> **Probabilistic intelligence; deterministic authority.**

Secondary axiom:

> **Intelligence may explore beyond the boundary. Effects may not.**

The project identity uses the language of core, boundary, state, admission, replay, proof, orbit, corona, and solar illumination to describe the architecture without redefining it.

See:

```text
docs/identity/SOL_Identity_Canon_v0.1.md
```

The Identity Canon is explicitly subordinate to normative specifications and merged architecture decisions.

Identity language must not introduce or imply technical authority absent from the normative architecture.

---

## Development and Review

Development proceeds through small, independently reviewable pull requests.

Important rules include:

- implemented behavior must remain distinguishable from proposals;
- specifications and architecture decisions precede authority-bearing implementation;
- no plan or discussion pre-authorizes later merges;
- authority-bearing changes require explicit review;
- failures should be classified before production behavior is modified;
- deterministic and adversarial tests are preferred over optimistic assumptions; and
- implementation must fail closed where authoritative context is unavailable.

The project owner retains merge authority.

---

## Institutional Boundary

SOL aims to provide inspectable technical authority, reproducible evidence, and locally controlled governance.

It does not by itself certify:

- research truth;
- legal compliance;
- regulatory approval;
- model safety;
- institutional accreditation; or
- fitness for a particular deployment.

Those remain separate human, institutional, scientific, legal, and regulatory responsibilities.

---

## Contributing

Keep changes focused, preserve the distinction between normative architecture and exploratory proposals, and document the verification performed.

See `CONTRIBUTING.md`.

---

## License

Licensed under the Apache License 2.0.
