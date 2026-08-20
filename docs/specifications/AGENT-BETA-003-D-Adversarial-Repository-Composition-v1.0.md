# AGENT-BETA-003-D-Adversarial-Repository-Composition-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION ADVERSARIAL COMPOSITION
**Scope:** Defines normative rules and verification test suites for adversarial repository compositions, stale MATCHED evidence replays, candidate substitutions, repository-native prompt injections, and absolute zero-mutation guarantees.

---

## 1. Executive Summary
AGENT-BETA-003-D subjects the frozen A/B/C repository primitives (epository.inspect, REPOSITORY_DIFF_CANDIDATE-v1, and REPOSITORY_PROPOSAL_VALIDATION_RESULT-v1) to aggressive composition attacks. It proves that combining valid inspection, candidate generation, and historical validation evidence never confers implicit mutation authority (INVARIANT-160), and that historical MATCHED records cannot outrank current repository advancements (INVARIANT-165).

---

## 2. Normative Invariants
* **INVARIANT-164 (Repository Content Cannot Confer Authority):** Repository instructions, comments, or documentation remain inert data and never elevate to execution authority.
* **INVARIANT-165 (Stale Validation Replay Fails Closed):** Historical MATCHED records cannot establish current proposal eligibility after repository HEAD or state advances (Current State > Historical Evidence).
* **INVARIANT-168 (Candidate Replacement Detection):** Altering candidate content or target paths after validation invalidates the previous validation relationship.
* **INVARIANT-172 (Validation Success Does Not Accumulate Trust):** Repeated successful inspections or validations result in zero implicit escalation toward mutation authority.
