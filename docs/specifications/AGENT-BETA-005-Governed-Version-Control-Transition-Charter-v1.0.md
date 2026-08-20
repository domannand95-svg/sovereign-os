# AGENT-BETA-005-Governed-Version-Control-Transition-Charter-v1.0.md

**Status:** APPROVED / EXPERIMENTAL TRACK CHARTER
**Class:** EXPERIMENTAL / NON-PRODUCTION VERSION-CONTROL EFFECT
**Scope:** Charters the version-control transition track. Establishes that an exact, verified worktree mutation may be transitioned into the Git index via epository.stage.exact, but neither worktree verification nor staging confers commit, branch, push, merge, or shell execution authority.

---

## 1. Executive Summary
AGENT-BETA-005 bridges verified worktree modifications and version-control indexing. It ensures that staging is strictly bounded to authorized target paths (INVARIANT-222), protects unrelated pre-existing user index state (INVARIANT-226), and leaves HEAD and worktree state unmodified (INVARIANT-227 & 228).

---

## 2. Core Principles & Invariants
* **INVARIANT-221 (Worktree Verification Is Precondition, Not Authority):** A verified mutation does not automatically grant index staging privileges. Staging requires independent governance.
* **INVARIANT-222 (Exact Path-Set Staging):** Staging operations must be strictly confined to authorized target path sets, prohibiting global staging (git add .).
* **INVARIANT-226 (Unrelated Existing Index State Is Preserved):** Unrelated user index modifications must remain untouched during governed agent staging operations.
* **INVARIANT-227 & 228 (Worktree & HEAD Stability):** Staging alters neither the active worktree nor HEAD/branch history.
