# Human-in-the-Loop Delivery Backlog

| Field | Value |
|---|---|
| Document ID | SOV-BACKLOG-001 |
| Status | Active |
| Last updated | 2026-07-31 |
| Governing plan | `EXECUTION-PLAN.md` |

## How to use this backlog

This backlog converts the architecture into small pull requests. Each item has
one acceptance outcome and must be approved independently by the project owner.

Before starting an item:

1. confirm its prerequisites are merged;
2. inspect the live repository for changed assumptions;
3. restate scope, non-goals, and acceptance tests; and
4. split the item again if its review surface becomes too large.

Documentation-only items do not require artificial runtime tests. Runtime items
must pass formatting, strict linting, focused tests, the full authoritative
workspace, and Linux/Windows CI.

## Track A: Architecture and constitutional contracts

### A01 — Adopt the corrected mission and delivery controls

- **Outcome:** Merge the governed research architecture, README, execution plan,
  legacy matrix, backlog, and contributor workflow.
- **Runtime change:** None.
- **Acceptance:** Links resolve, implemented and proposed features are clearly
  separated, and owner approval is recorded.
- **Prerequisites:** None.

### A02 — Authority separation ADR

- **Outcome:** Decide the proposer, executor, evaluator, reviewer, policy,
  promotion, incident, and human constitutional authority boundaries.
- **Runtime change:** None.
- **Acceptance:** Self-promotion, self-review, silent evidence mutation, and
  unilateral constitutional changes are explicitly prohibited.
- **Prerequisites:** A01.
- **Checkpoint:** Proposed in
  [`ADR-0002`](../adr/0002-authority-separation.md); acceptance requires
  owner-approved merge.

### A03 — Identity and lineage types

- **Outcome:** Add deterministic identifiers for agents, model versions, tools,
  datasets, policies, evaluators, and institutions.
- **Acceptance:** Canonical encoding and content roots are stable; ambiguous or
  incomplete identities are rejected.
- **Prerequisites:** A02.

### A04 — Research artifact and evidence schemas

- **Outcome:** Define objectives, claims, sources, methods, uncertainty, failed
  attempts, reviewer findings, disputes, and dispositions.
- **Acceptance:** Round-trip and malformed-input tests; evidence cannot silently
  lose provenance fields.
- **Prerequisites:** A03.

### A05 — Capability and resource grant schemas

- **Outcome:** Define operations, targets, tools, data scope, network scope,
  budgets, approvals, expiry, delegation, and revocation.
- **Acceptance:** Default deny; self-issued, expired, overbroad, and malformed
  grants fail validation.
- **Prerequisites:** A03.

### A06 — Trust and enforcement schemas

- **Outcome:** Define scoped trust assessments and correction, remediation,
  restriction, demotion, suspension, revocation, removal, and appeal records.
- **Acceptance:** No universal permanent trust score; authority changes require
  issuer, policy, reason, evidence, scope, and time.
- **Prerequisites:** A03.

### A07 — Governance transition policy

- **Outcome:** Admit constitutional artifact, grant, trust, and enforcement
  transitions through deterministic policy.
- **Acceptance:** Negative tests cover self-promotion, reviewer substitution,
  evidence suppression, self-grants, and unauthorized removal.
- **Prerequisites:** A04–A06.

## Track B: Legacy extraction and repository simplification

### B01 — Event-history contract extraction

- **Outcome:** Map useful `active-memory` and `event-log` behavior onto canonical
  ledger and planned audit projection contracts.
- **Runtime change:** Tests/specifications first.
- **Acceptance:** Ordering, malformed-record, query, and replay cases have active
  destinations.
- **Prerequisites:** A01.

### B02 — Remove duplicate event authority

- **Outcome:** Delete `event-log` and superseded persistence portions of
  `active-memory`.
- **Acceptance:** No active dependency or documentation claim remains; active
  workspace stays green.
- **Prerequisites:** B01.

### B03 — Scheduling and resource contract extraction

- **Outcome:** Extract deterministic placement, capacity, budget, lifecycle, and
  recovery cases from `governance-wrapper` and `registry-service`.
- **Acceptance:** Scheduler proposes work but cannot persist, grant itself
  resources, or become governance authority.
- **Prerequisites:** A05.

### B04 — Verification and promotion contract extraction

- **Outcome:** Extract verification, task lifecycle, promotion, rejection, and
  recovery cases from `registry-service`.
- **Acceptance:** Contracts align with independent evaluation and do not inherit
  prototype self-authority.
- **Prerequisites:** A04, A06.

### B05 — Distributed research archive

- **Outcome:** Preserve useful peer identity, heartbeat, conflict, snapshot
  installation, and hostile-network cases as specifications.
- **Runtime change:** No distributed production code.
- **Acceptance:** Consensus experiments are labelled non-authoritative and have
  named future tests or ADR questions.
- **Prerequisites:** A01.

### B06 — Remove remaining prototype implementations

- **Outcome:** Delete retired prototype crates and placeholder root executable.
- **Acceptance:** Every deleted module has a recorded destination or rejection;
  docs and workspace boundaries are simpler; CI stays green.
- **Prerequisites:** B02–B05.

## Track C: Governed authority runtime

### C01 — Canonical governance events

- **Outcome:** Persist admitted identity, artifact, grant, trust, and governance
  transitions in the canonical ledger.
- **Acceptance:** Replay reconstructs identical governance state and roots.
- **Prerequisites:** A07.

### C02 — Capability decision engine

- **Outcome:** Evaluate agent, operation, tool, target, data scope, approval,
  expiry, and delegation before execution.
- **Acceptance:** Ungranted, expired, delegated-without-authority, and
  out-of-scope operations are rejected before effects.
- **Prerequisites:** C01.

### C03 — Resource accounting

- **Outcome:** Reserve, consume, release, and audit compute, storage, network,
  token, money, time, rate, and concurrency budgets.
- **Acceptance:** Concurrent reservations cannot overspend; crash recovery
  reconstructs the same remaining allocation.
- **Prerequisites:** C02.

### C04 — Capability-scoped tool adapter

- **Outcome:** Provide one minimal tool adapter that accepts only validated
  handles rather than ambient credentials.
- **Acceptance:** Scope, target, budget, expiry, result, and failure evidence are
  recorded; bypass tests fail.
- **Prerequisites:** C02, C03.

### C05 — Grant suspension and revocation

- **Outcome:** Stop new operations and safely handle in-flight work when a grant
  is suspended or revoked.
- **Acceptance:** Revoked authority cannot be reused after restart; race cases
  are deterministic and fail closed.
- **Prerequisites:** C04.

## Track D: Evidence and adversarial research

### D01 — Provenance projection

- **Outcome:** Rebuild queryable links between objectives, agents, sources,
  methods, tool executions, claims, reviews, policy, and disposition.
- **Acceptance:** Missing, reordered, mismatched, or fabricated links are
  detected.
- **Prerequisites:** C01, A04.

### D02 — Source and citation verification

- **Outcome:** Record retrieval evidence and verify that cited material exists
  and supports the associated claim.
- **Acceptance:** Missing, invented, changed, and non-supporting sources produce
  distinct findings rather than silent acceptance.
- **Prerequisites:** D01, C04.

### D03 — Reproducibility runner

- **Outcome:** Repeat declared methods from identified inputs, fixtures,
  environment, tools, policy, and budgets.
- **Acceptance:** Exact replay, tolerance-based reproduction, and
  irreproducibility are clearly distinguished.
- **Prerequisites:** D01, C04.

### D04 — Specialized reviewer assignments

- **Outcome:** Assign citation, calculation, methodology, reproduction,
  security, privacy/licensing, and counterargument reviews.
- **Acceptance:** Proposers cannot choose only favorable reviewers; conflicts
  and shared model lineage are recorded.
- **Prerequisites:** D01, A02.

### D05 — Dispute and false-consensus handling

- **Outcome:** Preserve disagreement and detect correlated agreement.
- **Acceptance:** Unresolved disputes block configured promotions without
  rewriting minority evidence.
- **Prerequisites:** D02–D04.

### D06 — Governed promotion gate

- **Outcome:** Promote, reject, supersede, or revoke research artifacts through
  independent authority.
- **Acceptance:** The candidate and its proposer cannot approve promotion;
  negative evidence remains linked.
- **Prerequisites:** D05, A07.

## Track E: Trust, conduct, and institutional control

### E01 — Scoped reliability assessments

- **Outcome:** Derive reviewable assessments by agent version, domain, task,
  tool, data sensitivity, evidence, policy, and expiry.
- **Acceptance:** Performance in one scope cannot silently grant authority in
  another.
- **Prerequisites:** D06, A06.

### E02 — Correction and remediation

- **Outcome:** Run failed work again under restricted capabilities with recorded
  learning objectives.
- **Acceptance:** Remediation preserves the original failure and cannot rewrite
  the assessment evidence.
- **Prerequisites:** E01, C05.

### E03 — Demotion, suspension, revocation, and removal

- **Outcome:** Enforce proportionate consequences, including immediate response
  to severe incidents.
- **Acceptance:** Removed agents lose platform authority while historical
  evidence remains auditable.
- **Prerequisites:** E01, C05.

### E04 — Incident investigation and appeal

- **Outcome:** Separate subject, investigator, decision authority, and controlled
  re-evaluation.
- **Acceptance:** Serious outcomes contain evidence, policy, explanation, human
  accountability, scope, duration, and appeal status.
- **Prerequisites:** E03, A02.

### E05 — Institutional policy profiles

- **Outcome:** Support versioned local authority hierarchies, review thresholds,
  data boundaries, and mandatory human approvals.
- **Acceptance:** Policy changes are attributable, replayable, and cannot
  retroactively erase prior requirements.
- **Prerequisites:** A07, E04.

## Track F: Governed learning and local models

### F01 — Training-data governance schemas

- **Outcome:** Record source, consent, privacy class, licence, purpose, permitted
  derivatives, retention, and revocation handling.
- **Acceptance:** Unlicensed, non-consented, unknown-provenance, or incompatible
  examples cannot enter an admitted corpus.
- **Prerequisites:** A03, A04, E05.

### F02 — Evidence-corpus builder

- **Outcome:** Build versioned corpora from governed dispositions and labelled
  negative examples.
- **Acceptance:** Agents cannot insert or relabel their own output; corpus roots
  and inclusion decisions reproduce exactly.
- **Prerequisites:** F01, D06.

### F03 — Poisoning and contamination defenses

- **Outcome:** Add quarantine, source-correlation, duplicate, near-duplicate,
  anomaly, distribution-shift, and train/evaluation separation checks.
- **Acceptance:** Seeded poisoning and hidden-evaluation contamination cases are
  detected.
- **Prerequisites:** F02.

### F04 — Isolated training jobs

- **Outcome:** Run capability-scoped, resource-budgeted training with complete
  code, configuration, environment, base-model, dataset, and output lineage.
- **Acceptance:** Training cannot access unauthorized data, tools, credentials,
  or canonical mutation paths.
- **Prerequisites:** F03, C04.

### F05 — Model evaluation and promotion

- **Outcome:** Evaluate a trained model as a new candidate using hidden,
  rotating, adversarial, safety, privacy, memorisation, and leakage tests.
- **Acceptance:** The trainer and model cannot approve deployment; rollback and
  revocation are tested.
- **Prerequisites:** F04, D06, E03.

## Track G: Institutional product and release hardening

### G01 — Supported command boundary

- **Outcome:** Replace the placeholder executable with versioned commands for
  status, objectives, grants, runs, reviews, incidents, snapshots, and shutdown.
- **Acceptance:** Stable errors, authentication boundary, allowed/denied cases,
  and restart behavior.
- **Prerequisites:** C05, D06.

### G02 — Single-writer and filesystem security

- **Outcome:** Enforce writer ownership and defend paths, links, permissions,
  pending cleanup, and concurrent writers.
- **Acceptance:** Adversarial filesystem and lock tests fail closed.
- **Prerequisites:** G01.

### G03 — Fault injection and recovery

- **Outcome:** Test short writes, I/O failures, full disk, permission changes,
  rename failures, directory-sync failures, and process termination.
- **Acceptance:** No partial authority publication; recovery result is explicit.
- **Prerequisites:** G02.

### G04 — Backup, restore, upgrade, and rollback

- **Outcome:** Support version-aware operational lifecycle with tested recovery.
- **Acceptance:** Restored canonical roots match and unsupported migrations fail
  safely.
- **Prerequisites:** G03.

### G05 — Privacy-aware audit export

- **Outcome:** Export selected evidence without altering canonical hashes or
  disclosing unauthorized data.
- **Acceptance:** Redaction decisions are policy-controlled and auditable.
- **Prerequisites:** D01, E05.

### G06 — Observability and incident operations

- **Outcome:** Stable logs, metrics, health/readiness, alerts, and incident
  runbooks.
- **Acceptance:** Authority failures and resource exhaustion are visible without
  leaking protected content.
- **Prerequisites:** G01, G05.

### G07 — Reproducible release

- **Outcome:** Produce Windows and Linux artifacts, build provenance, SBOM,
  installation, operations, recovery, upgrade, security, and limitation docs.
- **Acceptance:** Independent clean-machine verification and owner-approved
  release tag.
- **Prerequisites:** G02–G06.

## Track H: Distributed foundations

This track is blocked until G07.

It begins with peer identity and authenticated bounded transport, then
deterministic network simulation, immutable artifact replication, and a separate
consensus ADR. Distributed canonical writes remain disabled until their safety
case is accepted.

## Human approval checkpoints

For each backlog item:

```text
inspect
  -> agree scope
  -> prepare locally
  -> explain changed files
  -> verify
  -> draft PR
  -> independent checks/review
  -> explicit owner merge approval
  -> merge
  -> update backlog evidence
```

No arrow may be skipped merely because later work depends on the item.
