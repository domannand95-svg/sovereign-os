# AGENT-BETA-007-Governed-Reference-Transition-Charter-v1.0.md

**Status:** APPROVED / EXPERIMENTAL TRACK CHARTER
**Class:** EXPERIMENTAL / NON-PRODUCTION LOCAL REFERENCE EFFECT
**Scope:** Charters the reference transition track. Establishes that an independently verified detached commit may be made reachable by advancing one exact local branch reference via epository.ref.advance_exact, but remote publication, merge, force updates, and resets remain strictly unauthorized.

---

## 1. Executive Summary
AGENT-BETA-007 bridges verified detached commit objects and local branch references (efs/heads/*). It enforces atomic compare-and-swap semantics (INVARIANT-344), exact one-step fast-forward constraints (INVARIANT-343), and guarantees that remote push, merge, and tag authority remain completely absent.

---

## 2. Core Principles & Invariants
* **INVARIANT-340 (Verified Commit Is Precondition, Not Ref Authority):** A VERIFIED commit does not automatically grant reference movement privileges. Reference movement requires independent governance.
* **INVARIANT-343 (Exact One-Step Fast-Forward):** Transitions are restricted strictly to one-step fast-forwards where C.parent == ExpectedOldCommit.
* **INVARIANT-344 (Atomic Old-Value Compare-and-Swap):** Reference updates use atomic compare-and-swap semantics, failing closed if the reference advanced concurrently.
* **INVARIANT-349 & 383 (Symbolic HEAD & Isolation):** Symbolic HEAD targets remain stable while resolving to the new commit tip, and non-target refs remain untouched.
