# AGENT-BETA-003-C-Repository-Proposal-Binding-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION PROPOSAL BINDING & STALENESS DETECTION
**Scope:** Defines normative rules for evaluating candidate proposal validity relative to host-observed repository states, enforcing baseline advancement detection (MATCHED vs STALE), and guaranteeing that validation results grant zero mutation authority (INVARIANT-160).

---

## 1. Executive Summary
AGENT-BETA-003-C introduces semantic proposal binding and staleness detection. It guarantees that candidate diffs generated against past repository states cannot be silently reinterpreted or treated as valid when repository HEAD or worktree state advances (INVARIANT-153 & 155). Furthermore, successful validation (MATCHED) confers zero repository mutation authority (INVARIANT-160).

---

## 2. Normative Invariants
* **INVARIANT-153 (Proposal State Relativity):** A candidate is valid only relative to the exact repository state against which it was generated.
* **INVARIANT-154 (Host-Observed Baseline Authority):** Host-observed repository state exclusively dictates validation; model-supplied or candidate-embedded baselines cannot establish authority.
* **INVARIANT-155 (Baseline Advancement Fails Closed):** Candidate baseline  \neq$ Observed baseline $ results strictly in STALE. No automatic rebasing or fuzzy application is permitted.
* **INVARIANT-160 (Validation Non-Authority):** REPOSITORY_PROPOSAL_VALIDATION_RESULT-v1 $\neq$ epository.mutate. Validation proves binding coherence only and grants zero write or mutation rights.
