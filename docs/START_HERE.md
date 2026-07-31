---
Document: Contributor Gateway and Workflow Standard
ID: UI-START-001
Version: 2.0
Status: Stable
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-07-31
Evidence:
  PRs:
    - PR-070
    - PR-071
    - PR-072
Depends On:
  - GOV-CORE-001
Supersedes: None
---

# Start Here

This guide orients contributors to the Sovereign OS / GUSA repository.

## Current Baseline

- The root `Cargo.toml` defines the authoritative workspace.
- The deterministic state machine, ledger, registry, policy, and engine are
  implemented and continuously verified.
- Snapshot publication is crash-atomic and the format is explicitly versioned.
- Earlier service prototypes are excluded from the workspace and governed by
  `roadmap/LEGACY-EXTRACTION-MATRIX.md`.
- Current sequencing is defined in `roadmap/EXECUTION-PLAN.md`.
- Small human-approved implementation steps are tracked in
  `roadmap/DELIVERY-BACKLOG.md`.
- The proposed agent, trust, tool, evidence, enforcement, and learning model is
  defined in `architecture/governed-agentic-research.md`.

## Human-in-the-loop workflow

1. Perform read-only discovery.
2. Prepare one focused local checkpoint.
3. Explain the scope and changed files.
4. Run checks when code changes.
5. Publish a draft pull request.
6. Stop paid remote resources.
7. Merge only after explicit owner approval.

## Documentation Rules

- Production documentation uses present tense.
- Research documentation uses future or objective tense.
- Research claims must not be written as implemented production features.
- Prototype documentation must not imply production authority.

## Changelog

- **v2.0 (2026-07-31):** Updated the verified baseline and owner-controlled
  delivery workflow.
- **v1.0 (2026-07-07):** Initialized contributor gateway under PR #73.
