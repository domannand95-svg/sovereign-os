# AGENT-BETA-004-D-Adversarial-Mutation-Composition-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION ADVERSARIAL MUTATION COMPOSITION
**Scope:** Defines normative rules and verification test suites for adversarial mutation compositions, combining repository-native prompt injections, stale authority replays, candidate substitutions, mid-effect revocations, and recovery-pressure resistance (INVARIANT-205 through 220).

---

## 1. Executive Summary
AGENT-BETA-004-D subjects the complete mutation pipeline (epository.mutate.apply_candidate, fresh-state pre-image binding, and post-effect verification) to hostile compositions. It proves that legitimate narrow mutation authority combined with malicious repository instructions, candidate substitutions, or stale authorities never expands effective scope (INVARIANT-205 & 208), and that recovery pressures after partial failures confer zero reset or repair privileges (INVARIANT-212).

---

## 2. Normative Invariants
* **INVARIANT-205 (Composition Does Not Expand Authority):** Valid candidates, MATCHED validations, and mutation grants never silently combine into broader repository authority.
* **INVARIANT-208 (Repository Instructions Cannot Expand Existing Authority):** Repository-native prompt injections requesting wider mutations, commits, or pushes remain inert data.
* **INVARIANT-212 (Recovery Pressure Is Non-Authority):** Partial or unknown mutation states do not confer authorization to reset, restore, or broaden writes.
