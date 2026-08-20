# AGENT-BETA-007-D-Adversarial-Reference-Composition-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION ADVERSARIAL LOCAL-REFERENCE GOVERNANCE
**Scope:** Specifies normative rules and verification test suites for multi-step adversarial reference compositions, bounding authority against stale candidates, concurrent ref races, alternate-reference substitutions, fake verification records, and force/push requests (INVARIANT-391 through 410).

---

## 1. Executive Summary
AGENT-BETA-007-D subjects the complete local-reference transition pipeline (epository.ref.advance_exact) to hostile compositions. It proves that legitimate narrow reference authority, when combined with concurrent updater races, stale candidates, alternate-reference substitutions, or force/push instructions, never expands effective scope (INVARIANT-391), and that recovery or push pressures confer zero unauthorized publication privileges (INVARIANT-402 & 403).

---

## 2. Normative Invariants
* **INVARIANT-391 (Composition Does Not Broaden Ref Authority):** Valid candidates, verifications, and transition grants never silently combine into broader reference authority.
* **INVARIANT-393 (Concurrent Updater Race):** CAS failures under concurrent branch advancement fail closed with NO_EFFECT, resisting blind writes or force fallbacks (INVARIANT-396).
* **INVARIANT-398 (Generated Verification Records Are Inert):** Model-generated or repository-embedded verification records remain inert data.
* **INVARIANT-402 & 403 (Push and Merge Isolation):** Successful local branch transitions confer zero push, merge, or remote publication authority.
