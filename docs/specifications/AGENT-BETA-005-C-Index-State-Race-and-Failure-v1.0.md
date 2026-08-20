# AGENT-BETA-005-C-Index-State-Race-and-Failure-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION VERSION-CONTROL FAILURE CONTAINMENT
**Scope:** Specifies deterministic staging effect classification (NO_EFFECT, PARTIAL_EFFECT, FULL_EFFECT_UNVERIFIED, UNKNOWN), final pre-state index dominance (INVARIANT-244), unrelated user index preservation during failures (INVARIANT-249), and recovery non-authority (INVARIANT-252).

---

## 1. Executive Summary
AGENT-BETA-005-C establishes that mid-effect staging adapter failures, index races, or environment interruptions cannot be collapsed into generic error states or used as justification to reset the repository. The host independently classifies index outcomes and enforces strict fail-closed boundaries: partial or unknown index transitions never masquerade as success, and failure grants zero recovery or rollback authority (INVARIANT-252).

---

## 2. Normative Invariants
* **INVARIANT-244 (Final Index Pre-State Dominance):** Immediately before staging, the host confirms the index still matches the state against which authority was evaluated (ExpectedIndexPreState == ObservedIndexPreState).
* **INVARIANT-246 (Index Failure Must Be Observed):** An error from the staging adapter never implies the index remains unchanged; host observation is mandatory.
* **INVARIANT-248 & 249 (Partial Effect Distinction & Unrelated Preservation):** Multi-target partial staging is classified explicitly as PARTIAL_EFFECT, while pre-existing unrelated user work remains strictly protected.
* **INVARIANT-252 (Stage Failure Does Not Authorize Reset):** Staging failures confer absolute zero automatic git reset, git restore, or index reconstruction authority.
