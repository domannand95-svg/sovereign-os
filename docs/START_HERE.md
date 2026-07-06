---
Document: Contributor Gateway and Workflow Standard
ID: UI-START-001
Version: 1.0
Status: Stable
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-07-07
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

- **PR #70:** Hybrid snapshot compaction merged.
- **PR #71:** Registry open benchmark baseline merged.
- **PR #72:** Snapshot integrity, versioning, checksum validation, and fallback recovery merged.
- **PR #73:** Engineering Handbook Foundation is the current documentation-only target.

## Mobile-Safe Workflow

1. Perform read-only discovery.
2. Apply one focused pasteable script.
3. Run checks when code changes.
4. Inspect `git status`.
5. Commit and push only after verification.

## Documentation Rules

- Production documentation uses present tense.
- Research documentation uses future or objective tense.
- Research claims must not be written as implemented production features.
- Rust code is not modified in PR #73.

## Changelog

- **v1.0 (2026-07-07):** Initialized contributor gateway under PR #73.
