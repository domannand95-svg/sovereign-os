# EXP-BETA-016-Expiry-Revocation-Boundary-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** NON-PRODUCTION / RUNTIME GOVERNANCE
**Scope:** Defines normative schemas (CAPABILITY_STATE_QUERY-v1 and CAPABILITY_REVOCATION_CHECK_RESULT-v1) and temporal/revocation interception rules.

---

## 1. Executive Summary
EXP-BETA-016 establishes the real-time state validation boundary (CAPABILITY_STATE_QUERY-v1 and CAPABILITY_REVOCATION_CHECK_RESULT-v1). This subsystem intercepts runtime execution attempts to ensure that granted capabilities are active, unexpired, and unrevoked at the exact moment of dispatch.

---

## 2. Normative Invariants
* **INVARIANT-068 (Expired Authority Is Non-Executable):** A capability past its lifetime cannot be revived by request repetition.
* **INVARIANT-069 (Revocation Dominates Previous Approval):** If a capability has been revoked in the lifecycle history, execution is strictly denied regardless of prior admission.
* **INVARIANT-070 (No Runtime Self-Renewal):** Executing agents cannot extend duration or request implicit renewals via runtime execution hooks.
* **INVARIANT-071 (Temporal Determinism):** Given a capability history, a requested timestamp, and lifecycle events, the state evaluation result must be completely deterministic.
