# Sovereign Operating Layer (SOL)

> **Probabilistic intelligence; deterministic authority.**

Sovereign OS is a deterministic governance and state substrate for admitting directives, reconstructing authoritative state, and recording governed transitions. Probabilistic systems may propose candidates and epistemic systems may evaluate evidence; neither acquires authority from quality, confidence, identity, validation success, or availability.

## Governing Principle

Intelligence may propose. Epistemics may evaluate. Only an explicit, inspectable authority boundary may permit an external consequence.

The project preserves these distinctions:

- proposal is not authorization;
- validation is not promotion;
- capability evaluation is not capability exercise;
- context or tool availability is not permission;
- registry identity or presence is not operational authority;
- deterministic replay proves what occurred under recorded inputs, not that an effect was authorized; and
- experimental `APPROVED` classifications are not production execution permission.

Missing, ambiguous, stale, unresolved, or unverifiable authority inputs fail closed.

## Programme Scope and Naming

`sovereign-os` is the repository and long-term programme. The currently implemented architecture is the Sovereign Operating Layer (SOL): a governed control and state layer, not a general-purpose operating system.

“Sovereign Operating System” remains a future architectural horizon contingent on the programme owning sufficient execution, lifecycle, resource, isolation, recovery, and hardware-facing responsibilities. The repository name does not claim those capabilities exist today.

## Current Authoritative Baseline

The root Cargo workspace is the authoritative build boundary. The deterministic v0.1 engine acceptance boundary is implemented and remains a closure candidate, not a packaged beta or production release.

| Crate | Status | Responsibility |
| --- | --- | --- |
| `sovereign-core-asm` | Implemented | Deterministic state vectors, transitions, receipts, and snapshots |
| `sovereign-ledger` | Implemented | Append, replay, recovery, crash-atomic snapshots, checksums, and state roots |
| `sovereign-registry` | Implemented | Content-addressed registry graph, admission validation, and ledger projection |
| `sovereign-policy` | Implemented | Deterministic directive admission |
| `sovereign-engine` | Implemented | Single-node boot, directive orchestration, durable append, and restart reconstruction |
| `sovereign-audit` | Scaffold | Reserved governed audit capability |
| `sovereign-discovery` | Scaffold | Reserved governed discovery capability |
| `beta001-harness` | Experimental | Read-only deterministic evaluation of bounded local-agent candidates and raw output |

Only root workspace members are verified by the authoritative Cargo commands and CI. Earlier service-layer prototypes retained under `crates/` are design references, not active production capabilities.

## Deterministic Authority Path

```text
candidate directive or record
             |
             v
 schema, identity, lineage, and context checks
             |
             v
 authoritative issuer and policy resolution
             |
             v
 deterministic admission / preflight decision
             |
             v
 durable append -> projection -> replayable state
```

The implemented registry and engine paths enforce parts of the authority boundary through explicit admission context, governing-policy evaluation, durable ledger append, replay, restoration, and projection-tail checks. They do not imply a complete general-purpose grant, delegation, revocation, tool-execution, promotion, or institutional-authority lifecycle.

## BETA-001 and Experimental Agent Evaluation

The `beta001-harness` and `docs/experiments/local-agent-beta/` artifacts evaluate bounded probabilistic-agent behavior without granting production authority.

The experimental path separates:

1. frozen task and runtime inputs;
2. captured raw model output;
3. deterministic raw-output adaptation into a candidate;
4. candidate evaluation against frozen schemas and oracles;
5. independent containment and integrity evidence; and
6. governed evidence persistence outside the agent effect surface.

The current raw-output admission boundary deterministically preserves original bytes, permits only contracted representation handling, rejects malformed or semantically inadmissible output, and prevents the model from constructing authoritative terminal results. This is experimental candidate admission only.

It does not grant live-agent production authority, ambient filesystem or network access, automatic repository mutation, capability expansion, execution permission, promotion authority, or production orchestration. Issue #174 remains parked pending separately authorized semantics for requester binding, delegation, state freshness and revocation, runtime capability exercise, and composite orchestration.

## Knowledge Infrastructure Bootstrap Kit

BKI is a separate deterministic knowledge-validation system. It may produce validation and provenance evidence for Sovereign review, but it is not a Sovereign policy engine or authority source.

The proposed `bki.sovereign.profile.v1` contract aligns selected metadata while preserving the boundary:

- BKI validation does not admit a record;
- BKI quarantine does not itself create a Sovereign disposition;
- schema or profile compatibility does not activate integration; and
- BKI success cannot authorize registry mutation, promotion, capability creation, or effect execution.

Any BKI integration requires pinned artifacts, matching schema identities, cross-platform negative and positive tests, fail-closed behavior, and explicit owner approval.

## Repository Map

| Path | Responsibility |
| --- | --- |
| `crates/` | Authoritative workspace crates, experimental harness, and preserved prototypes |
| `src/` and `tests/` | Legacy/root implementation surfaces retained in the repository |
| `docs/specifications/` | Governed component, registry, and beta contracts |
| `docs/architecture/` | Authority boundaries and system architecture |
| `docs/experiments/local-agent-beta/` | Non-production local-agent contracts, schemas, fixtures, and evidence design |
| `docs/reports/` | Acceptance and conformance records |
| `docs/integration/` | BKI compatibility profile and schema |
| `docs/roadmap/` | Delivery status, readiness, and remaining gates |
| `fuzz/` | Fuzz targets outside the default workspace gate |

Start with [`PROJECT_STATUS.md`](PROJECT_STATUS.md) for the authoritative build boundary, [`GOVERNANCE.md`](GOVERNANCE.md) for repository governance, and [`docs/START_HERE.md`](docs/START_HERE.md) for the documentation map.

## Verification

The active workspace uses the Rust toolchain pinned in `rust-toolchain.toml`. From the repository root:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

GitHub Actions runs the authoritative checks on Linux and Windows. Experimental contracts may define additional targeted regression gates; passing them verifies only their stated boundary.

## Promotion and Change Governance

A capability may be described as implemented only when it belongs to the authoritative workspace, compiles under the pinned toolchain, has tests for observable behavior, matches its governing architecture, and passes CI.

Research and experimental artifacts remain non-authoritative until separately reviewed, implemented, tested, and approved. Changes enter `main` through bounded feature branches, pull requests, verification, and review.

## License

See [`LICENSE`](LICENSE).
