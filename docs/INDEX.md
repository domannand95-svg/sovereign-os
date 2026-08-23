---
Document: Engineering Handbook Index
ID: DOC-INDEX-001
Version: 2.6
Status: Stable
Owner: Repository Maintainer
Reviewers:

- Core Reviewers

Last Updated: 2026-08-23

Depends On:
- GOV-CORE-001

Supersedes:
- DOC-INDEX-001 v2.5
---

# Sovereign OS Engineering Handbook

The Engineering Handbook is the authoritative documentation set describing the production architecture and documenting future engineering work.

---

# Volume I — Repository Governance

- GOVERNANCE.md
- ARCHITECTURE.md
- START_HERE.md

---

# Volume II — Production Architecture

- Registry Architecture
- Event Log Architecture
- Snapshot Architecture
- Runtime Architecture

## Governed Intelligence and Authority Separation

- `architecture/governed-agentic-research.md` - proposed agent, evidence,
  capability, trust, enforcement, and governed-learning architecture

- `architecture/authority-epistemics-separation.md` - baseline separating
  proposals and epistemic evaluation from consequential authority

- `adr/0002-authority-separation.md` - merged constitutional authority roles,
  independence rules, prohibited combinations, and negative cases

## Identity and Evidence Architecture

- `specs/SPEC-ID-001-Canonical-Identity-and-Lineage.md` - canonical
  identifiers, lineage records, validation, and stability vectors

- `specs/SPEC-EV-001-Governed-Research-Artifacts-and-Evidence.md` - proposed
  canonical research-artifact, provenance, review, dispute, and disposition
  schemas

- `specs/SPEC-EV-001-A04-Binary-Encoding.md` - approved immutable v1 limits,
  timestamps, and canonical binary-encoding rules

- `integration/BKI-SOS-PROFILE-001.md` - proposed versioned metadata and
  language boundary for read-only BKI compatibility

---

# Volume III — Execution Architecture

The execution architecture defines the deterministic authority boundary between
external proposals, governed execution, and post-execution evidence.

## BETA-025 — Execution Boundary

- `handover/SOVEREIGN_OS_BETA025_EXECUTION_BOUNDARY.md` - execution boundary
  and API translation layer definition

Validated concepts:

- GovernedExecutor boundary
- deterministic execution dispatch
- fail-closed execution errors
- capability separation

## BETA-026 — Execution API Contract

- `handover/SOVEREIGN_OS_BETA026_EXECUTION_API_CONTRACT.md` - versioned
  execution request/response contracts, sanitized execution status model,
  and execution API error taxonomy

Validated concepts:

- GovernedExecutionRequest
- GovernedExecutionResponse
- ExecutionStatus
- ExecutionApiError taxonomy
- external consumer contract boundary

## BETA-027 — Execution Evidence Pipeline

- `reports/BETA-027_CLOSURE_RECORD.md` - formal closure record documenting
  execution receipt generation, evidence projection, and authority separation

Validated concepts:

- ExecutionReceipt DTO
- deterministic evidence identity
- ExecutionProjectionAdapter
- AuditLedgerEntry projection
- AuditLedgerChain integration

---

# Volume IV — Governance Freeze

## BETA-028 — Architecture Consolidation

- `reports/BETA-028_ARCHITECTURE_FREEZE_PLAN.md` - architecture freeze scope,
  frozen boundaries, deferred capabilities, invariant registry direction,
  and integration planning

BETA-028 scope:

- documentation consolidation
- architecture preservation
- invariant formalization
- branch reconciliation planning

No new execution authority layers are introduced during freeze.

---

# Volume V — Engineering Process

- ADR Template
- RFC Template
- `governance/DEVELOPMENT-WORKFLOW.md` - repository change and approval process

---

# Volume VI — Planning

- `roadmap/ROADMAP.md` - current milestone summary
- `roadmap/EXECUTION-PLAN.md` - active delivery sequence and review workflow
- `roadmap/DELIVERY-BACKLOG.md` - dependency-ordered, owner-approved PR slices
- `roadmap/LEGACY-EXTRACTION-MATRIX.md` - prototype extraction and retirement
- `roadmap/V0.1-ACCEPTANCE-CONTRACT.md` - completed foundation contract
- `roadmap/SOVEREIGN-OS-COMPLETION-PLAN.md` - historical planning baseline

---

# Volume VII — Research Portfolio

Research documentation is maintained separately from production architecture.

Current research placeholders include:

- GUSA
- TCF-01A
- Indra's Net
- Sovereign Stack

Research documentation is informational and shall not be interpreted as implemented production functionality.

---

# Architectural Invariants

The following invariants define the Sovereign OS authority model:

```
Intelligence ≠ Authority

Proposal ≠ Permission

Permission ≠ Execution

Execution ≠ Evidence

Evidence ≠ Authority
```

Interpretation:

- Intelligence may propose but cannot authorize.
- Proposals require independent policy evaluation.
- Permission does not guarantee execution success.
- Execution events generate evidence after completion.
- Evidence records history but cannot create authority.

---

# Documentation Principles

Production documentation:

- describes implemented architecture
- uses present tense
- reflects validated repository state

Research documentation:

- describes objectives
- uses future or objective tense
- does not imply implemented production functionality

---

# Changelog

- **v2.6 (2026-08-23)** Added BETA-026 execution API contract,
  BETA-027 execution evidence pipeline, and BETA-028 architecture freeze
  documentation references.

- **v2.5 (2026-08-23)** Recorded the BETA-025 execution boundary and API
  translation layer implementation.

- **v2.4 (2026-08-16)** Added the Authority-Epistemics Separation baseline.

- **v2.3 (2026-08-11)** Recorded the owner-approved A04 v1 binary-encoding
  baseline and advanced the evidence schema to implementation.

- **v2.2 (2026-08-11)** Added the proposed BKI and Sovereign OS language
  compatibility profile and pinned schema candidate.

- **v2.1 (2026-08-11)** Added the A04 governed evidence schema contract and
  reconciled merged A02/A03 status labels.

- **v2.0 (2026-07-07)** Handbook cross-linking and verification.