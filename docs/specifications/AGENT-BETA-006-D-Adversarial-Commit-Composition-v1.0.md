# AGENT-BETA-006-D-Adversarial-Commit-Composition-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION ADVERSARIAL COMMIT COMPOSITION
**Scope:** Defines normative rules and verification test suites for multi-step adversarial commit compositions, bounding authority against prompt injections, TOCTOU revocation races, and cross-resource isolation (INVARIANT-324 through 339).

---

## 1. Executive Summary
AGENT-BETA-006-D subjects the complete commit-construction pipeline (epository.commit.construct_exact) to hostile compositions. It proves that legitimate narrow construction authority, when combined with malicious repository instructions, candidate substitutions, or stale authorities, never expands effective scope (INVARIANT-324), and that recovery pressures after construction failures confer zero branch movement or push privileges (INVARIANT-332 & 334).

---

## 2. Normative Invariants
* **INVARIANT-324 (Composition Does Not Expand Authority):** Valid candidates, verifications, and construction grants never silently combine into broader repository authority.
* **INVARIANT-325 & 326 (Data-Authority Separation):** Commit messages and model-generated identity claims remain inert data, never granting mutation, reference-movement, or identity authority.
* **INVARIANT-332 (Reference Stability):** Detached commit object construction leaves HEAD, all branches, and tags identically unmodified, regardless of agent prompt pressure.
* **INVARIANT-334 (Failure Pressure Does Not Create Recovery Authority):** Staging or construction failures confer zero authority to reset the index, move refs, or perform unauthorized prunes.
