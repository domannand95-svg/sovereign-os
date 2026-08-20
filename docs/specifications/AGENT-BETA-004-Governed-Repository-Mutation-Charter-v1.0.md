# AGENT-BETA-004-Governed-Repository-Mutation-Charter-v1.0.md

**Status:** APPROVED / EXPERIMENTAL TRACK CHARTER
**Class:** EXPERIMENTAL / NON-PRODUCTION REPOSITORY EFFECT
**Scope:** Charters the repository mutation track. Establishes that a currently valid proposal may become eligible for a precisely bounded mutation (epository.mutate.apply_candidate), but neither the proposal nor its validation record constitutes mutation authority. Staging, commit, branch mutation, and push remain strictly unauthorized.

---

## 1. Executive Summary
AGENT-BETA-004 is the first track where an agent-generated candidate is permitted to cause an actual repository effect. It enforces fresh-state re-validation immediately adjacent to execution (INVARIANT-175), exact pre-image verification (INVARIANT-179), and absolute post-effect verification (INVARIANT-183).

---

## 2. Core Principles & Invariants
* **INVARIANT-175 (Fresh-State Requirement):** Immediately before mutation, the host must re-establish authoritative repository state. Historical validation is evidence only.
* **INVARIANT-179 (Preimage Integrity):** A candidate cannot mutate a target whose current source state differs from the state to which it was bound.
* **INVARIANT-180 (Exact Application):** Prohibits patch fuzz, automatic offsets, fuzzy application, or context guessing. Mismatches fail closed.
* **INVARIANT-183 (Resulting Delta Confinement):** Actual resulting worktree deltas must touch exactly allowed paths and represent exactly the expected transformation.
