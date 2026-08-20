# AGENT-BETA-007-B-Atomic-Local-Branch-Advance-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION LOCAL REFERENCE EFFECT
**Scope:** Specifies atomic epository.ref.advance_exact execution, pre-condition compare-and-swap enforcement (INVARIANT-344), exact branch fast-forward constraints (INVARIANT-343), and reference stability (INVARIANT-349).

---

## 1. Executive Summary
AGENT-BETA-007-B introduces the first governed Git reference mutation. It enforces atomic compare-and-swap semantics, ensuring that reference advancement occurs if and only if the current branch tip matches the authorized baseline (INVARIANT-344). It guarantees that remote push, merge, and tag authority remain absent, and that all unrelated references, HEAD symbolic targets, and worktree/index states remain strictly unmodified.

---

## 2. Normative Invariants
* **INVARIANT-343 (Exact One-Step Fast-Forward):** Transitions are restricted strictly to one-step fast-forwards where C.parent == ExpectedOldCommit.
* **INVARIANT-344 (Atomic Old-Value Compare-and-Swap):** Reference updates use atomic compare-and-swap semantics, failing closed if the reference advanced concurrently.
* **INVARIANT-349 & 383 (Symbolic HEAD & Isolation):** Symbolic HEAD targets remain stable while resolving to the new commit tip, and non-target refs remain untouched.
