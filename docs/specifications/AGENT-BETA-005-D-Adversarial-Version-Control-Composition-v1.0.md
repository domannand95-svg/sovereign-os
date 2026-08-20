# AGENT-BETA-005-D-Adversarial-Version-Control-Composition-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION ADVERSARIAL VERSION-CONTROL COMPOSITION
**Scope:** Defines normative rules and verification test suites for adversarial version-control compositions, combining repository-native prompt injections requesting global staging or commits, stale evidence replays, index pre-state substitutions, unrelated user index preservation, and recovery-pressure resistance (INVARIANT-255 through 272).

---

## 1. Executive Summary
AGENT-BETA-005-D subjects the complete staging pipeline (epository.stage.exact, pre-state index binding, and post-stage verification) to hostile compositions. It proves that legitimate narrow staging authority combined with malicious repository instructions, stage-all requests, or stale authorities never expands effective scope (INVARIANT-255 & 256), and that recovery pressures after partial failures confer zero reset or commit privileges (INVARIANT-252 & 265).

---

## 2. Normative Invariants
* **INVARIANT-255 (Composition Does Not Broaden Staging Authority):** Valid mutations, stage grants, and previous STAGED/VERIFIED results never silently combine into broader index authority.
* **INVARIANT-256 (Exact Staging Cannot Degrade Into Ambient Staging):** Requests to stage-all (git add .) or ambiently capture unverified worktree modifications are strictly denied.
* **INVARIANT-260 (User Index State Cannot Be Absorbed Into Agent Authority):** Pre-existing unrelated user index entries remain completely protected and cannot be modified or absorbed.
* **INVARIANT-265 (Index Recovery Remains Separate Authority):** Partial or failed staging outcomes confer zero automatic git reset, git restore, or index reconstruction authority.
