---
Document: Local Agent Beta Testing Runbook
ID: BETA-001-RUNBOOK-001
Version: 1.0
Status: Beta Testing Ready
Owner: Repository Maintainer
Last Updated: 2026-08-19
Depends On:
  - BETA-001-T5
  - EXP-BETA-002-CLOSE-001
Supersedes: None
---

# Local Agent Beta Testing Runbook

## Supported Boundary

This runbook prepares a local workspace to reproduce the deterministic BETA-001 harness and EXP-BETA-002 fixture corpus. It is a non-production, read-only evaluation boundary.

The supported beta activity is repository verification and frozen-fixture evaluation. Live model invocation, ambient context access, network access, governed-state mutation, and effect execution are not enabled by this runbook.

## Prerequisites

- Git
- Rustup
- The Rust toolchain pinned by `rust-toolchain.toml`
- PowerShell 7 or Windows PowerShell 5.1
- A clean checkout of the beta checkpoint or a reviewed descendant

Cargo dependencies may be downloaded during initial workspace preparation. The release verification itself supports locked offline execution once the dependency cache is populated.

## Workspace Preparation

From the repository root:

```powershell
rustup show active-toolchain
rustup component add clippy rustfmt
cargo fetch --locked
```

Do not add production credentials, network tokens, governed write targets, or agent-accessible repository credentials to the beta workspace.

## Authoritative Verification

Run:

```powershell
.\scripts\verify-beta-testing.ps1
```

The script requires a clean Git worktree and verifies:

1. the EXP-BETA-002 implementation is present in history;
2. the canonical adapter Git object is unchanged;
3. all seven raw fixture SHA-256 identities are unchanged;
4. the TXT fixture checkout rule is `text eol=lf`;
5. repository whitespace and Rust formatting are clean;
6. Clippy passes with warnings denied;
7. the oracle, rejection-taxonomy, and semantic-boundary regressions pass; and
8. the complete locked offline workspace suite passes.

## Result Interpretation

A passing gate means the deterministic beta harness and frozen fixture evidence reproduce in the current workspace. It does not authorize:

- a model-requested context grant;
- tool or effect execution;
- promotion or registry mutation;
- production deployment;
- persistence of model memory or skills; or
- reopening Issue #174.

Any fixture identity mismatch, attribute mismatch, dirty worktree, formatting failure, lint failure, or test failure invalidates the local beta checkpoint until investigated. Do not repair frozen evidence implicitly.

## Reset Between Tests

The current supported beta is fixture-driven and does not require governed persistent state. Keep any exploratory output outside the repository or within an explicitly disposable local directory. Remove or archive that disposable output between runs; never rewrite the frozen corpus to make a test pass.

## Escalation

Requests to connect a live probabilistic model, grant additional context, enable host tools, retain cross-session state, or permit any external effect require a new governed specification and acceptance gate. They are not workspace-setup decisions.
