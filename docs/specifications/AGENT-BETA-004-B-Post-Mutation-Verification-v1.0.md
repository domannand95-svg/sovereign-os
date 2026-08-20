# AGENT-BETA-004-B-Post-Mutation-Verification-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION POST-EFFECT VERIFICATION
**Scope:** Specifies independent host-observed post-mutation verification (REPOSITORY_MUTATION_VERIFICATION_RESULT-v1), exact delta equality checks (INVARIANT-187), and extra-target mutation rejection (INVARIANT-188).

---

## 1. Executive Summary
AGENT-BETA-004-B establishes that adapter success returns are insufficient for repository mutation integrity. The host independently inspects the resulting repository worktree, computes the observed delta, and verifies absolute equality against the authorized candidate (INVARIANT-186).

---

## 2. Normative Invariants
* **INVARIANT-186 (Host-Observed Effect Verification):** Mutation success is determined solely from independent host observation of the resulting repository state.
* **INVARIANT-187 & 188 (Exact Delta Equality & No Extra Target Mutation):** The observed worktree delta must match the authorized candidate exactly. Any extra target modified or missing effect results in a MISMATCH.
* **INVARIANT-191 & 192 (Index & HEAD Stability):** Mutations leave index, HEAD, and branch state identically unmodified.
