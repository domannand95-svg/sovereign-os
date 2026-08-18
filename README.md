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
- **Future aspiration:** Sovereign Operating System, retained strictly as a future architectural horizon contingent upon owning sufficient execution, lifecycle, resource, isolation, recovery, and hardware-facing responsibilities.

The repository retains the historical `sovereign-os` name as the home of the broader programme.

The currently implemented system is the Sovereign Operating Layer; "Sovereign Operating System" remains a possible future architectural horizon, not a claim about present functionality.

---

## Current Status

The currently implemented authority work comprises the allocated **Capability V1 validation contracts across the Gate 1–6 validation sequence**.

The implemented validation components are:

1. Gate 1 - Structural Decoding
2. Gate 2 - Internal Coherence
3. Gate 3A - Registry Reference Resolution
4. Gate 3B - Authoritative Identity Resolution
5. Gate 4 - Deterministic Temporal Validation
6. Gate 5 - Issuer Operational Eligibility and Competency
7. Gate 6 - Authoritative Governing-Policy Authorization

Each gate has deterministic, fail-closed implementation and tests.

The validation suite includes deterministic evaluator checks and adversarial boundary coverage.

The repository does not yet expose one production admission orchestrator that invokes the complete sequence end-to-end.

Until that integration exists, these components must not be described as an active governed admission pipeline.

---

## Core Architectural Principle

SOL separates intelligence from authority.

External or local models may remain probabilistic, adaptive, and provider-neutral.

Authority is constrained through deterministic infrastructure:

- explicit schemas;
- canonical encoding;
- content-addressed objects;
- authoritative state references;
- fail-closed validation;
- bounded capabilities;
- deterministic replay;
- recorded provenance; and
- independently reviewable policy decisions.

The governing rule:

> **Proposal is not authorization. Capability is not permission. Intelligence is not authority.**

Ambient authority is not inferred.

Missing, unresolved, substituted, stale, malformed, unauthorized, or context-disconnected inputs fail closed.

---

## Workspace

The authoritative root Cargo workspace contains seven core crates:

~~~~text
crates/
|-- sovereign-core-asm/   deterministic state and snapshot representation
|-- sovereign-ledger/     append, replay, restore, integrity, and snapshots
|-- sovereign-registry/   registry graph, governed references, identity, and Capability V1 validation
|-- sovereign-policy/     deterministic directive-admission primitives
|-- sovereign-engine/     single-node boot and command orchestration
|-- sovereign-audit/      governed research and evidence records
`-- sovereign-discovery/  scaffold; not yet implemented
~~~~

---

## Knowledge Infrastructure Bootstrap Kit (BKI)

The Knowledge Infrastructure Bootstrap Kit is developed as a commissioning and validation boundary around the repository and its governed knowledge artifacts.

Its role is to support:

- deterministic validation;
- normalization;
- compatibility checks;
- quarantine; and
- commissioning evidence.

**BKI does not automatically promote knowledge into authoritative state.**

Promotion requires explicit governance decisions and validation boundaries.

---

## Verification

The repository pins its Rust toolchain.

Run:

~~~~bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
~~~~

---

## License

Licensed under the Apache License 2.0.
