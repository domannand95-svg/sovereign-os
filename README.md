# Sovereign OS

> Building resilient autonomous computing through deterministic state, event sourcing, and digital sovereignty.

Sovereign OS is an experimental operating system written in Rust. It explores
event-sourced state, deterministic recovery, cryptographically verifiable
projections, and governed autonomous computing.

Rather than treating persistence as an implementation detail, the active core
records admitted transitions in an append-only ledger. State and registry
projections can then be reconstructed deterministically, accelerated through
validated snapshots, and checked against the same ledger tail.

> [!IMPORTANT]
> The authoritative build is the seven-crate `sovereign-*` Cargo workspace.
> Earlier service-layer prototypes remain in `crates/` for reference but are
> not workspace members. Read [PROJECT_STATUS.md](PROJECT_STATUS.md) before
> changing crate membership or treating prototype features as production.

## Vision

Modern systems accumulate mutable state that becomes difficult to reason about.
Sovereign OS takes the opposite approach:

- important transitions become immutable events;
- snapshots are validated caches rather than sources of truth;
- recovery is deterministic;
- governance decisions can become auditable;
- failures close authority rather than silently publishing uncertain state.

The long-term objective is an operating environment that coordinates autonomous
services while preserving integrity, fault tolerance, and digital sovereignty.

## Implemented baseline

The active workspace currently provides:

- append-only ledger records with checksums and strict LSN sequencing;
- deterministic replay and state reconstruction;
- versioned snapshots with integrity and state-root validation;
- automatic fallback from invalid snapshots to genesis replay;
- content-addressed registry nodes and deterministic ledger projection;
- deterministic directive admission policy;
- single-node engine boot, directive submission, and restart reconstruction;
- fail-closed handling of ambiguous persistence outcomes;
- forensic tests for corruption and interrupted publication.

Distributed networking, governance, audit, discovery, agent lifecycle
management, active memory, neuromorphic hardware, and ternary runtime work are
not part of the active production baseline.

## Architecture

```text
Directive
    |
    v
Deterministic policy
    |
    v
Append-only ledger
    |
    +------------------+
    |                  |
    v                  v
Authoritative state    Registry projection
    |                  |
    +---------+--------+
              |
              v
      Single-node engine
```

Snapshots accelerate state restoration. The ledger remains canonical.

## Workspace structure

```text
crates/
|-- sovereign-core-asm/   deterministic state and snapshot representation
|-- sovereign-ledger/     append, replay, restore, integrity, and snapshots
|-- sovereign-registry/   content-addressed derived registry graph
|-- sovereign-policy/     deterministic directive admission
|-- sovereign-engine/     single-node boot and command orchestration
|-- sovereign-audit/      scaffold; not implemented
|-- sovereign-discovery/  scaffold; not implemented
`-- service prototypes/   excluded; classified in PROJECT_STATUS.md
```

## Verification

The repository pins its Rust toolchain. Run:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

GitHub Actions runs these checks on Linux and Windows.

## Roadmap

### Current engineering priorities

1. Keep the active workspace green on Linux and Windows.
2. Prove the complete boot, admit, persist, restart, and reconstruct path.
3. Keep implemented, scaffolded, prototype, and research claims separate.
4. Select the next capability through an explicit architecture decision.

### Candidate next capabilities

- governed audit evidence;
- discovery and multi-node identity;
- snapshot schema migration;
- distributed consensus and networking;
- governance and agent lifecycle management.

## Design principles

- Event sourcing over opaque mutable state
- Deterministic recovery
- Cryptographic integrity
- Fail-closed authority
- Modular Rust architecture
- Explicit governance
- Evidence-backed capability claims
- Long-term maintainability

## Project status

Sovereign OS is active research and engineering work. The single-node core is
implemented and tested; broader distributed and autonomous-system capabilities
remain staged work.

See [PROJECT_STATUS.md](PROJECT_STATUS.md) for the exact build boundary and
[ARCHITECTURE.md](ARCHITECTURE.md) for the top-level architecture classification.

## Contributing

Contributions are welcome. Create a focused feature branch, keep changes
reviewable, and include the verification performed. See
[CONTRIBUTING.md](CONTRIBUTING.md).

## License

License information will be published with the first stable release. Until a
license is added, no open-source reuse rights are granted by default.

---

*"Deterministic systems are easier to reason about. Immutable history is easier to trust."*
