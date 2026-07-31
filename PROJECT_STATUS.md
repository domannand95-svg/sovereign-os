# Project Status

This document identifies the authoritative build boundary for Sovereign OS. It
prevents implemented, prototype, scaffolded, and research capabilities from
being conflated.

## Authoritative workspace

The root `Cargo.toml` and `Cargo.lock` define the active build:

| Crate | Status | Responsibility |
|---|---|---|
| `sovereign-core-asm` | Implemented | Deterministic state vectors, transitions, receipts, and snapshots |
| `sovereign-ledger` | Implemented | Append, replay, recovery, snapshots, checksums, and state roots |
| `sovereign-registry` | Implemented | Content-addressed registry graph and ledger projection |
| `sovereign-policy` | Implemented | Deterministic directive admission |
| `sovereign-engine` | Implemented | Single-node boot, directive orchestration, and restart reconstruction |
| `sovereign-audit` | Scaffold | Reserved for a governed audit capability |
| `sovereign-discovery` | Scaffold | Reserved for a governed discovery capability |

Only these crates are verified by root workspace commands and CI.

## Preserved prototypes

The following crates contain earlier service-layer experiments and are not
members of the authoritative workspace:

- `active-memory`
- `event-log`
- `registry-service`
- `network-service`
- `governance-wrapper`

They are retained as design and implementation references. Their presence does
not mean their APIs or documented capabilities are part of the current build.
Do not add them to the workspace as a bulk change. Any reuse should proceed
through an explicit architecture decision that resolves duplicated
responsibilities and dependency direction.

## Research boundary

Research documents describe objectives and candidate architectures. They do
not establish implemented production capabilities. Distributed networking,
governance, audit, discovery, agent lifecycle management, active memory,
neuromorphic hardware, and ternary runtime work remain outside the active
production baseline unless promoted through code, tests, and an architecture
decision.

## Verification

The active workspace uses Rust `1.97.1`. Run:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

GitHub Actions runs the same checks on Linux and Windows.

## Promotion rule

A capability may be described as implemented only when:

1. its crate is an authoritative workspace member;
2. it compiles under the pinned toolchain;
3. its observable behavior is covered by tests;
4. the relevant architecture documentation matches the implementation; and
5. CI verifies it on every pull request.
