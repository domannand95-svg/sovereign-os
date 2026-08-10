# Sovereign OS Completion Plan

> **Historical planning baseline:** This document records the plan used before
> the current single-node foundation and snapshot-hardening work was merged.
> Some branch names, test totals, and "remaining work" items below are stale.
> Use [EXECUTION-PLAN.md](EXECUTION-PLAN.md) for current sequencing and
> [LEGACY-EXTRACTION-MATRIX.md](LEGACY-EXTRACTION-MATRIX.md) for prototype
> retirement decisions.

| Field | Value |
|---|---|
| Document ID | SOV-PLAN-001 |
| Version | 0.1-draft |
| Status | Draft for owner and reviewer approval |
| Owner | Sovereign OS project owner |
| Implementation authority | Codex |
| Strengthening reviewer | Kimi |
| Adversarial reviewer | Fable review team |
| Supporting reviewers | DeepSeek (filesystem/concurrency), Gemini (architecture/traceability) |
| Last updated | 2026-07-17 |
| Depends on | Constitutional invariants, ADR 0001, Gate 8 restoration, v0.1 acceptance contract |

## 1. Mission

Complete Sovereign OS as a governed, deterministic runtime in which every canonical state transition is policy-authorized, durably recorded, reproducibly replayable, independently auditable, and incapable of being promoted by the agent that proposed it.

The program advances through runnable releases rather than isolated modules. Every release ends with an executable acceptance command and evidence that can be reproduced by a reviewer from a clean checkout.

## 2. Definition of Complete

Sovereign OS v1.0 is complete when a clean single-node installation can:

1. Boot from an empty ledger, a valid snapshot, or a full replay.
2. Accept a typed external directive through a stable service or CLI boundary.
3. Authenticate and authorize the directive through policy and capability checks.
4. Apply the transition to canonical state and registry projections without divergence.
5. Persist the event atomically and reconstruct identical state after process or host restart.
6. Isolate discovery and experimental artifacts from production authority.
7. Record provenance, policy decisions, execution evidence, and promotion decisions in an immutable audit trail.
8. Operate with documented configuration, observability, backup, recovery, and upgrade procedures.
9. Pass the security, resilience, compatibility, and performance release gates in this plan.

Multi-node consensus is a separate release track. It is required for the distributed edition, but it does not block a truthful single-node v1.0 designation.

## 3. Evidence Status

### Verified baseline

- PR #119 merged Gate 8 restoration into `main`.
- Ledger restoration, snapshot discovery, tail scanning, checkpoint replay, and failure-atomic restoration are merged.
- The authoritative `main` branch implements boot, registry reconstruction,
  policy admission, atomic append publication, restart reconstruction, and
  crash-boundary tests.
- Current validation evidence at commit `014a748`: formatting and strict Clippy
  pass; 180 tests pass with one intentionally ignored subprocess helper; Rust
  and security/coverage workflows pass on the current Windows/Linux baseline.

### Inspected non-production components

The following crates exist but their production maturity must be established by inspection rather than percentage estimates:

- `sovereign-discovery`
- `sovereign-audit`
- `active-memory`
- `event-log`
- `governance-wrapper`
- `registry-service`
- `network-service`

`sovereign-audit` and `sovereign-discovery` are authoritative workspace
scaffolds without implemented runtime behavior. The five preserved prototype
crates remain outside the authoritative workspace and do not establish current
capabilities.

## 4. Delivery Model

Each sprint must define:

- one runnable outcome;
- an explicit in-scope and out-of-scope boundary;
- an acceptance command known before implementation;
- positive, negative, corruption, and restart cases where applicable;
- a review bundle or pushed branch containing identical evidence for all reviewers;
- a reversible commit sequence;
- a merge decision made by the project owner.

The sequence for every material sprint is:

1. Codex inspects and implements against the live repository.
2. Codex runs targeted tests, full workspace tests, formatting, strict Clippy, and diff audit.
3. The branch is pushed and a PR is opened without merging.
4. Kimi performs a maintainability and API-strengthening review against the actual PR.
5. Codex reproduces and reconciles Kimi's findings, then updates the branch.
6. Fable performs an adversarial review against the strengthened revision.
7. Codex reproduces every claimed defect, fixes confirmed defects, and documents rejected claims with evidence.
8. The complete acceptance suite is rerun.
9. The owner authorizes merge.

Reviewers never modify the authoritative checkout directly. Findings require file-and-line evidence or a reproducible failing test.

## 5. Release Roadmap

## Release v0.1 — Deterministic Engine Bootstrap

### Outcome

A bootable single-node engine restores canonical state, reconstructs the registry, applies policy to a directive, persists an admitted event atomically, restarts, and reconstructs the identical state root.

### Closure state

The deterministic engine outcome and exit gate are implemented. Remaining work
before a `v0.1.0` release is repository and release governance rather than core
engine behavior:

1. merge the v0.1 closure record after independent review;
2. select and declare the repository licence;
3. exercise decoder fuzz targets;
4. reproduce a packaged-node artifact from a clean environment; and
5. approve and exercise artifact signing before creating the release tag.

### Exit gate

```bash
cargo test --workspace --test boot_restart
```

Plus:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

### Non-negotiable invariants

- Policy denial occurs before persistence.
- A crash exposes either the previous state or the complete new state, never a partial record.
- Publication order is record write, file sync, atomic rename, directory sync.
- Bootstrap removes only strictly recognized stale pending artifacts.
- Registry projection and restored state agree on the replayed final LSN.

## Release v0.2 — Capability Firewall and Governed Artifacts

### Outcome

Discovery and experimental artifacts can be proposed and evaluated without acquiring canonical authority. Promotion is an explicit, independently authorized governance transition.

### Workstreams

1. Inspect `sovereign-discovery` and document its trust boundary.
2. Define content-addressed `HarnessCandidate` and `HarnessManifest` schemas as data, not privileged executable configuration.
3. Define artifact lifecycle states: proposed, admitted-to-sandbox, evaluated, rejected, approved, promoted, revoked.
4. Implement a sandbox execution boundary with no direct canonical ledger, registry, policy, credential, network, or filesystem mutation capability.
5. Reset every evaluation from a declared snapshot and deterministic fixture set.
6. Record inputs, outputs, resource limits, environment identity, state roots, and replay trace.
7. Require a distinct promotion authority and signed promotion event.
8. Add negative tests proving a candidate cannot self-promote, rewrite policy, modify canonical state, or suppress audit evidence.

### Exit gate

A dedicated integration suite must prove:

- two evaluations from the same snapshot and inputs produce the same evidence root;
- candidate execution cannot mutate canonical state;
- a candidate cannot promote itself;
- promotion requires a separately authorized governance event;
- rejected artifacts remain traceable but inactive.

Proposed command:

`cargo test --workspace --test capability_firewall`

## Release v0.3 — Audit, Provenance, and Active Memory

### Outcome

Operators and governance can explain every canonical state transition, its policy basis, its originating artifact, and its replay evidence without trusting volatile process memory.

### Workstreams

1. Inspect `sovereign-audit`, `active-memory`, `event-log`, and `governance-wrapper` for overlap and legacy duplication.
2. Select one authoritative event path and formally deprecate or adapt duplicate paths.
3. Define provenance identifiers linking directive, policy decision, ledger record, state transition, registry mutation, snapshot, and audit entry.
4. Implement queryable audit projections rebuilt from the canonical ledger.
5. Define retention, redaction, and disclosure rules without altering canonical hashes.
6. Add consistency checks between ledger state, audit projection, and active-memory projection.
7. Add tamper, omission, reordering, and replay-equivalence tests.

### Exit gate

`cargo test --workspace --test provenance_replay`

The suite must reconstruct an explanation for every state-changing event and detect missing, reordered, or mismatched provenance.

## Release v0.4 — Operable Single-Node Services

### Outcome

A user can install, configure, run, inspect, back up, restore, and safely stop Sovereign OS through supported interfaces rather than test-only APIs.

### Workstreams

1. Inspect and integrate `registry-service`.
2. Define a versioned CLI or local service API for boot, status, directive submission, snapshot, audit query, and controlled shutdown.
3. Add authentication and authorization at the external boundary.
4. Define configuration precedence, validation, secure defaults, and secret handling.
5. Add structured logs, metrics, health/readiness signals, and stable error codes.
6. Implement backup, restore, compaction policy, disk-space handling, and operator runbooks.
7. Add clean shutdown and forced termination tests.
8. Add schema/version migration with rollback or explicit fail-closed behavior.
9. Package a reproducible binary or container and generate an SBOM.

### Exit gate

A black-box test must start a packaged node, submit allowed and denied directives, query state and audit evidence, terminate it, restore it from backup, and verify identical roots.

Proposed command:

`cargo test --workspace --test packaged_node_lifecycle`

## Release v0.5 — Distributed Node Foundations

### Outcome

Nodes communicate through authenticated, bounded protocols without weakening single-node authority or pretending that transport equals consensus.

### Workstreams

1. Inspect `network-service` and define protocol/version boundaries.
2. Add peer identity, mutual authentication, replay protection, message size limits, timeouts, and backpressure.
3. Separate discovery, transport, replication, and consensus responsibilities.
4. Define canonical conflict behavior. No last-writer-wins shortcut may be introduced for constitutional state.
5. Replicate content-addressed immutable artifacts before attempting canonical-state agreement.
6. Add partition, duplication, reordering, Byzantine-input, and downgrade tests.
7. Select or explicitly defer a consensus protocol through an ADR.

### Exit gate

A deterministic network simulation must prove bounded behavior under loss, duplication, reordering, partition, and hostile messages. Distributed canonical writes remain disabled until a consensus ADR and safety proof are accepted.

Proposed command:

`cargo test --workspace --test network_adversarial`

## Release v1.0 — Production-Hardened Single Node

### Outcome

The single-node edition is supportable, secure by default, upgradeable, observable, and backed by reproducible release evidence.

### Release gates

1. Threat model covers assets, trust boundaries, attacker capabilities, and residual risk.
2. Fuzzing covers record, snapshot, manifest, policy, registry, and protocol decoders.
3. Property tests cover replay equivalence, state-root determinism, policy determinism, and projection consistency.
4. Fault injection covers short writes, I/O errors, full disk, permission changes, rename failures, directory-sync failures, and process termination.
5. Supported filesystem and platform guarantees are documented and tested.
6. A single-writer lock or equivalent invariant is enforced rather than assumed.
7. Symlink, path traversal, ownership, and permission attacks are tested.
8. Performance budgets exist for boot, replay, snapshot, append, audit query, and storage growth.
9. Upgrade and rollback are tested across every supported on-disk format version.
10. CI builds from a clean checkout, runs the complete suite, produces signed artifacts and an SBOM, and records provenance.
11. Documentation includes installation, configuration, operations, incident response, backup, recovery, upgrade, and security limitations.
12. Independent reviewers reproduce the release evidence from the tagged commit.

### Exit gate

`./scripts/release-gate.sh v1.0.0`

The script must run all unit, integration, adversarial, fuzz smoke, migration, packaging, and documentation checks from a clean checkout.

## 6. Cross-Cutting Technical Debt Register

The following items must be scheduled before the release that depends on them:

| Item | Required by | Current risk |
|---|---|---|
| Split snapshot unreadable I/O from malformed data | v0.2 diagnostics | Retryable failures are classified as corruption |
| Mark or remove non-seeking replay bootstrap footgun | v0.2 | Restore callers may choose the wrong constructor |
| Enforce the single-writer invariant | v1.0 | Atomic rename alone does not serialize competing writers |
| Test real filesystem/power-loss guarantees | v1.0 | Process-abort tests do not model storage-controller loss |
| Harden pending cleanup against live-writer and symlink races | v1.0 | Cleanup currently relies on naming and single-writer assumptions |
| Restore segment packing without weakening atomicity | Performance gate | Current correctness-first publication uses one segment per record |
| Define snapshot trust/fast-restart policy | Performance gate | Validate-through seeking scales with history length |
| Resolve legacy event-log versus sovereign-ledger authority | v0.3 | Duplicate event paths can create ambiguous authority |
| Define on-disk schema migration policy | v0.4 | Format changes can strand or misread state |

## 7. Meta-Harness Compatibility Constraints

Current and future APIs must preserve these constraints:

1. A candidate is immutable, content-addressed data.
2. Candidate generation grants no execution or promotion authority.
3. Evaluation begins from an identified snapshot and declared environment.
4. Candidate execution receives capability-scoped handles, never ambient authority.
5. Experimental writes target disposable state only.
6. Evaluation evidence is appended by infrastructure outside the candidate's control.
7. Promotion requires an independent policy/governance decision recorded in the canonical ledger.
8. The candidate cannot alter its score, baseline, evaluator, policy, or audit trace.
9. Every promoted harness remains linked to the candidate, evidence, reviewers, policy version, and promotion event.
10. Rollback selects a previously admitted content-addressed harness; it never mutates history.

Meta-Harness implementation begins only after v0.2 proves the capability firewall. It does not redirect the v0.1 completion path.

## 8. Program Controls

### Branch and merge discipline

- One outcome branch per release or tightly bounded sprint.
- No direct writes to `main`.
- No `git add -A` while transfer/review archives are present.
- Explicit staging paths only.
- Every commit must compile; every PR must satisfy its declared exit gate.
- Temporary review archives are deleted before staging.

### Finding reconciliation

A reported issue is confirmed only when Codex can point to the relevant source or reproduce it with a failing test. Severity is based on impact and reachability, not the reviewing model's confidence. Conflicting reviewer conclusions are resolved through code, platform documentation, tests, or an ADR.

### Change control

Any change to constitutional authority, canonical identity, state-root computation, ledger format, promotion authority, or consensus requires an ADR before implementation.

## 9. Immediate Next Actions

1. Merge the v0.1 closure record after review and passing gates.
2. Protect `main` with required review and all current CI/security checks.
3. Record the owner-selected repository licence.
4. Implement and exercise decoder fuzz targets.
5. Define A04 research-artifact and evidence schemas.
6. Continue A05–A07 before implementing canonical governance events.
7. Add packaged-node and release-signing evidence before tagging `v0.1.0`.

## 10. Goal-Setting Recommendation

After owner approval, set the persistent goal to:

> Complete Sovereign OS through the verified v1.0 single-node release defined by SOV-PLAN-001, advancing one executable acceptance gate at a time, preserving constitutional invariants, and requiring independent strengthening and adversarial review before each milestone merge.

The distributed-node track remains planned through v0.5 but is not allowed to delay or weaken the production-hardened single-node v1.0 release.
