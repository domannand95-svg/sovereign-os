---
Document: Master Software Engineering Roadmap
ID: ROADMAP-CORE-001
Version: 2.0
Status: Active
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-07-31
Evidence:
  - PR #119
  - PR #123
  - PR #124
  - PR #125
Depends On:
  - GOV-CORE-001
Supersedes: None
---

# Sovereign OS Master Software Roadmap

The active delivery sequence and human-in-the-loop workflow are defined in
[EXECUTION-PLAN.md](EXECUTION-PLAN.md). Prototype extraction and retirement
decisions are tracked in
[LEGACY-EXTRACTION-MATRIX.md](LEGACY-EXTRACTION-MATRIX.md).
Small implementation slices and their prerequisites are tracked in
[DELIVERY-BACKLOG.md](DELIVERY-BACKLOG.md).
The product authority and evidence model is defined in
[Governed Agentic Research Architecture](../architecture/governed-agentic-research.md).

## Completed foundation

- Engineering handbook and architecture boundaries
- Deterministic state machine and state roots
- Durable ledger append, replay, and restoration
- Crash-atomic, versioned snapshots
- Deterministic registry projection and policy admission
- Single-node engine boot and restart reconstruction
- Linux and Windows continuous integration

## Current

- Adopt the governed-agentic-research constitutional architecture.
- Define versioned agent, evidence, trust, tool, resource, and governance
  schemas.
- Classify and extract useful prototype contracts without inheriting duplicate
  authority.

## Next

- Default-deny capability, tool, data, and resource authority
- Research evidence and provenance
- Deterministic evaluation and adversarial review
- Trust enforcement, demotion, revocation, and removal
- Governed learning and institution-controlled models
- Supported institutional single-node interface
- Operational and security hardening
- Reproducible single-node release

## Deferred track

Authenticated networking and distributed consensus remain deferred until the
single-node release is dependable.

## Changelog

- **v2.0 (2026-07-31):** Rebased the roadmap on the verified authoritative
  workspace and linked the execution and legacy-retirement plans.
- **v1.0 (2026-07-07):** Initialized roadmap under PR #73.
