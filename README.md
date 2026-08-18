# Sovereign Operating Layer (SOL)

**Probabilistic intelligence; deterministic authority.**

Sovereign Operating Layer (SOL) is an experimental Rust control plane exploring deterministic authority boundaries around probabilistic intelligence.

SOL does not attempt to make models deterministic.

Instead, it places deterministic, auditable, fail-closed infrastructure around probabilistic components.

Models may reason, propose, classify, search, and request capabilities.

The surrounding control plane determines:

- what authority exists;
- which state is authoritative;
- what may be admitted;
- what may cause external effects; and
- what must be retained for audit and replay.

> **Intelligence may explore beyond the boundary. Effects may not.**

---

# Programme Scope and Naming

- **Repository:** `sovereign-os`
- **Implemented architecture:** Sovereign Operating Layer (SOL)
- **Future architectural horizon:** Sovereign Operating System

The repository name reflects the broader programme vision.

The currently implemented system is a governed control layer, not a complete operating system.

---

# Current Status

The current implementation focuses on deterministic authority validation foundations.

Implemented areas include:

- Capability V1 validation contracts;
- Registry v2 compatibility boundaries;
- canonical encoding and decoding;
- deterministic validation gates;
- governed evidence records;
- adversarial authority evaluation;
- deterministic replay foundations;
- persistence integrity boundaries.

The validation sequence includes:

1. Structural decoding.
2. Internal coherence checks.
3. Registry reference resolution.
4. Authoritative identity resolution.
5. Deterministic temporal validation.
6. Issuer eligibility validation.
7. Governing-policy authorization boundaries.

These components provide deterministic validation contracts.

They do not yet represent a complete production admission orchestrator.

---

# Core Architectural Principle

SOL separates intelligence from authority.

External or local models may remain:

- probabilistic;
- adaptive;
- provider-neutral; and
- exploratory.

Authority is constrained through deterministic infrastructure:

- explicit schemas;
- canonical encoding;
- content-addressed objects;
- authoritative state references;
- fail-closed validation;
- bounded capabilities;
- deterministic replay;
- provenance preservation; and
- independently reviewable decisions.

The governing rule:

> **Proposal is not authorization. Capability is not permission. Intelligence is not authority.**

Ambient authority is never inferred.

Missing, unresolved, substituted, stale, malformed, unauthorized, or disconnected context fails closed.

---

# Deterministic Validation

SOL validation boundaries enforce:

- canonical representation;
- schema correctness;
- semantic consistency;
- authoritative reference resolution;
- identity binding;
- temporal correctness;
- policy evaluation boundaries; and
- adversarial failure handling.

Validation determines whether a candidate satisfies defined authority requirements.

Validation does not create authority.

---

# Governed Evidence

SOL includes governed evidence foundations.

Implemented structures include:

- objective records;
- claim records;
- source records;
- method records;
- uncertainty records;
- failed attempt records;
- reviewer findings;
- provenance preservation;
- deterministic encoding;
- audit-oriented validation.

Evidence remains epistemic.

Evidence does not authorize execution.

---

# Knowledge Infrastructure Bootstrap Kit

The Knowledge Infrastructure Bootstrap Kit (BKI) operates as a complementary knowledge governance boundary.

BKI provides:

- intake;
- classification;
- normalization;
- validation;
- provenance preservation.

SOL provides:

- authority boundaries;
- capability admission;
- deterministic execution controls.

The relationship:

    Information
        |
        v
    BKI
    Classification / Normalization / Validation
        |
        v
    Governed Evidence
        |
        v
    SOL
    Authority Checks / Admission / Execution Boundaries
        |
        v
    Bounded Effects

Neither system silently inherits the authority of the other.

---

# Workspace

The root Cargo workspace contains:

    crates/
    |-- sovereign-core-asm/
    |-- sovereign-ledger/
    |-- sovereign-registry/
    |-- sovereign-policy/
    |-- sovereign-engine/
    |-- sovereign-audit/
    `-- sovereign-discovery/

---

# Verification

Run:

    cargo fmt --all -- --check

    cargo clippy --workspace --all-targets --locked -- -D warnings

    cargo test --workspace --all-targets --locked

Boundary-affecting changes should also verify:

- deterministic replay;
- fail-closed behaviour;
- substitution resistance;
- authoritative context binding.

---

# Development Principles

SOL development follows:

- specifications before authority-bearing implementation;
- explicit acceptance boundaries;
- small reviewable changes;
- deterministic testing;
- adversarial validation;
- separation between proposals and permissions;
- clear distinction between research and implementation.

---

# License

Licensed under the Apache License 2.0.