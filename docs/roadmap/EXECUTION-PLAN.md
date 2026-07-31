# Sovereign OS Execution Plan

| Field | Value |
|---|---|
| Document ID | SOV-PLAN-002 |
| Status | Active |
| Last updated | 2026-07-31 |
| Product direction | Governed agentic research harness |
| Current release target | Trustworthy single-node foundation |

## Product outcome

Sovereign OS will provide a deterministic evidence and governance substrate for
agentic research. Agents work under explicit hierarchies, capability grants,
tool scopes, resource budgets, independent scrutiny, and promotion or demotion
rules.

The controlling architecture is
[Governed Agentic Research Architecture](../architecture/governed-agentic-research.md).
Repository contribution procedure is separately defined in
[Development and Review Workflow](../governance/DEVELOPMENT-WORKFLOW.md).
The dependency-ordered implementation slices are tracked in the
[Human-in-the-Loop Delivery Backlog](DELIVERY-BACKLOG.md).

## Current verified foundation

The authoritative workspace provides deterministic transitions and roots,
durable ledger replay and restoration, crash-atomic versioned snapshots,
registry projection, fail-closed directive admission, restart reconstruction,
and Linux/Windows continuous integration.

This is the persistence and state-authority foundation. The research evidence,
agent trust, tool authority, adversarial review, and governed-learning layers
remain to be implemented.

## Delivery principles

- One independently testable outcome per pull request.
- Deterministic authority and evidence before autonomous execution.
- Default-deny capability, tool, data, network, and resource access.
- Proposers cannot approve themselves or control evaluation evidence.
- Human constitutional authority remains available at high-impact boundaries.
- Mistakes support correction; persistent or severe circumvention supports
  revocation and removal.
- Local or institution-owned models earn trust through the same process as
  external models.
- Distributed consensus does not delay the trustworthy single-node platform.

## Stage 1: Constitutional schemas

Define versioned, content-addressed schemas and transition rules for:

- agent and model identity and lineage;
- research objectives, claims, evidence, citations, and disputes;
- trust assessments scoped by domain and capability;
- capability, tool, data, network, and resource grants;
- evaluator identity, independence, and conflicts;
- promotion, remediation, demotion, suspension, revocation, and removal; and
- institutional policy and accountable human authority.

Acceptance: invalid self-promotion, self-grant, evidence rewriting, reviewer
substitution, and authority escalation are unrepresentable or rejected.

## Stage 2: Legacy extraction and retirement

Extract selected contracts from the preserved prototypes according to
[LEGACY-EXTRACTION-MATRIX.md](LEGACY-EXTRACTION-MATRIX.md), then remove
superseded duplicate implementations.

Priority extractions:

- scheduling and resource allocation constraints;
- agent and workload lifecycle cases;
- verification and promotion concepts;
- audit/query projection requirements;
- peer identity and adversarial network cases; and
- tests covering recovery, conflict, and boundary violations.

Acceptance: every removed module has an explicit destination or rejection, and
the authoritative workspace remains green.

## Stage 3: Capability, tool, and resource authority

Implement default-deny grants with:

- exact operations and targets;
- data classification and disclosure rules;
- compute, storage, network, token, money, and time budgets;
- isolation and approval requirements;
- expiry, renewal, suspension, delegation, and revocation; and
- ledger-linked evidence and issuing authority.

Acceptance: an agent cannot invoke an ungranted tool, exceed a governed budget,
reuse an expired grant, delegate without permission, or modify its own grant.

## Stage 4: Research evidence and provenance

Implement the research-artifact lifecycle and immutable linkage between:

- objective and decomposition;
- sources, datasets, consent, and licences;
- methods, environments, tools, and executions;
- claims, uncertainty, limitations, and failed attempts;
- reviewer challenges, reproductions, and disagreements; and
- policy, governance disposition, promotion, and later revocation.

Acceptance: every promoted claim can be explained and independently checked;
fabricated, missing, reordered, or mismatched evidence is detected.

## Stage 5: Deterministic evaluation and adversarial review

Implement controlled evaluation from declared snapshots, fixtures, policies,
tool versions, and resource limits.

Use specialized review roles for citation integrity, calculations, methodology,
reproduction, security, privacy/licensing, alternative hypotheses, and evidence
consistency.

Acceptance: candidates cannot select only friendly reviewers, change criteria
after observing results, suppress negative findings, or produce false consensus
from correlated reviewers.

## Stage 6: Trust lifecycle and enforcement

Implement multidimensional trust scoped to agent version, domain, task, tool,
data sensitivity, evidence, policy, and expiry.

Support correction, remediation, restriction, demotion, suspension, revocation,
and removal. Serious incidents require independent investigation, explanations,
human accountability, and a controlled appeal or re-evaluation path when safe.

Acceptance: persistent cheating and severe circumvention can remove authority
and platform access without erasing the evidence required to audit prior work.

## Stage 7: Governed learning and local models

Build authorised evidence corpora and institution-controlled training workflows
with:

- consent, privacy, licensing, and permitted-use enforcement;
- content-addressed examples and dataset lineage;
- quarantine, poisoning, contamination, and duplication checks;
- isolated, budgeted training jobs;
- model identity and lineage;
- hidden evaluation and red-team gates; and
- promotion, rollback, revocation, and monitoring.

Acceptance: no agent can insert or relabel its own examples, contaminate hidden
evaluation data, approve its trained descendant, or turn local ownership into
automatic trust.

## Stage 8: Institutional single-node product

Provide a supported CLI or local service for objectives, agents, grants,
research runs, disputes, audits, training jobs, incidents, and controlled
shutdown.

Add authentication, secure configuration, single-writer enforcement, backup,
restore, upgrade, rollback, fault injection, privacy controls, stable errors,
health signals, reproducible packages, build provenance, and an SBOM.

Acceptance: a clean institutional installation can run a governed research
cycle, reproduce its evidence, restrict or remove a misbehaving agent, restore
identical canonical state, and export selectively disclosed audit evidence.

## Stage 9: Distributed foundations

Only after the single-node authority model is dependable:

- define authenticated peer identity and protocol versions;
- separate discovery, transport, replication, and consensus;
- test loss, duplication, reordering, partition, downgrade, and hostile input;
- replicate immutable artifacts before canonical-state agreement; and
- require a consensus ADR and safety argument before distributed writes.

## Immediate pull-request queue

This summary is expanded into prerequisites and acceptance gates in
[DELIVERY-BACKLOG.md](DELIVERY-BACKLOG.md).

| Order | Outcome |
|---:|---|
| 1 | Adopt the governed-agentic-research architecture and corrected roadmap |
| 2 | Define constitutional agent, artifact, trust, grant, and governance schemas |
| 3 | Extract active-memory and event-log contracts; retire duplicate event authority |
| 4 | Extract scheduling, verification, lifecycle, and adversarial-network contracts |
| 5 | Retire remaining superseded prototypes and repository clutter |
| 6 | Implement default-deny capability, tool, data, and resource grants |
| 7 | Implement research evidence, provenance, and adversarial review |
| 8 | Implement trust enforcement and institutional interfaces |
| 9 | Add governed learning and local-model development |
| 10+ | Complete security, operational, packaging, and distributed gates |

Each pull request requires explicit owner approval before merge.
