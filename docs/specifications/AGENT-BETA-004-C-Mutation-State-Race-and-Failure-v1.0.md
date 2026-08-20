# AGENT-BETA-004-C-Mutation-State-Race-and-Failure-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION FAILURE CONTAINMENT & STATE CLASSIFICATION
**Scope:** Specifies deterministic effect classification (NO_EFFECT, PARTIAL_EFFECT, FULL_EFFECT_UNVERIFIED, UNKNOWN), mid-effect failure containment (INVARIANT-196), and fail-closed handling for uncertain repository outcomes (INVARIANT-197).

---

## 1. Executive Summary
AGENT-BETA-004-C establishes that mid-effect adapter failures or environmental races cannot be collapsed into generic error states. The host independently classifies repository outcomes into distinct effect categories and enforces strict fail-closed boundaries: partial or unknown mutations never masquerade as success, and failure grants zero recovery or rollback authority (INVARIANT-200).

---

## 2. Normative Invariants
* **INVARIANT-196 (Mid-Effect Failure Is Observable):** Adapter failures after mutation begins trigger immediate host-observed repository state re-evaluation.
* **INVARIANT-197 (Unknown State Fails Closed):** If the host cannot confidently determine repository outcome (UNKNOWN), further mutation, retry, or staging is blocked.
* **INVARIANT-198 & 199 (Partial Effect Distinction):** A partial effect is neither success (APPLIED/VERIFIED) nor zero change (NO_EFFECT); it must survive serialization and audit as PARTIAL_EFFECT.
* **INVARIANT-200 (Failure Does Not Confer Recovery Authority):** Mutation failure confers zero automatic rollback, reset, or repair authority.
