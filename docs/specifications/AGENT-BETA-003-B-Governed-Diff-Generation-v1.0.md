# AGENT-BETA-003-B-Governed-Diff-Generation-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION GOVERNED DIFF GENERATION
**Scope:** Defines normative rules for candidate-only diff generation (REPOSITORY_DIFF_CANDIDATE-v1), absolute worktree isolation (INVARIANT-150), and baseline binding (INVARIANT-144).

---

## 1. Executive Summary
AGENT-BETA-003-B extends repository operations to allow a probabilistic agent to propose patch candidates against inspected repository baselines. It guarantees absolute worktree isolation: generated diffs remain inert data structures and carry zero capability to mutate live repositories (INVARIANT-143).

---

## 2. Normative Invariants
* **INVARIANT-143 (Diff Non-Execution):** $\text{Generated Diff} \neq \text{Applied Diff}$. Syntactically valid candidate patches have zero mutation authority.
* **INVARIANT-144 & 145 (Baseline & Repository Identity Binding):** Candidates must bind strictly to host-observed repository identifiers and baseline commits, rejecting model-supplied false baselines.
* **INVARIANT-146 (Target-Path Confinement):** Changed paths in candidates must remain within governed proposal scopes.
* **INVARIANT-150 (No Worktree Mutation):** Successful candidate generation leaves HEAD, index, and working tree identically unmodified.
