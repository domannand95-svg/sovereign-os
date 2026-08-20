# EXP-BETA-013-Temporary-Capability-Grant-Boundary-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** NON-PRODUCTION / PRE-AUTHORITY TO AUTHORITY BRIDGE
**Scope:** Defines the normative schema and security boundaries for TEMPORARY_CAPABILITY_GRANT_RESULT-v1.

---

## 1. Executive Summary
EXP-BETA-013 establishes the controlled bridge between completed pre-authority negotiation (BETA-002) and the core admission/lifecycle pipeline (BETA-001). The TEMPORARY_CAPABILITY_GRANT_RESULT-v1 artifact represents a bounded activation proposal derived from negotiation without holding or bypassing admission authority.

---

## 2. Normative Invariants
* **INVARIANT-053 (Negotiation Dependency):** A temporary grant cannot exist without valid request, evaluation, and scope negotiation artifacts.
* **INVARIANT-054 (Grant Non-Self-Activation):** A temporary grant object cannot self-activate or produce a CapabilityPermission directly.
* **INVARIANT-055 & 056 (Scope & Duration Bounding):** GrantedScope ⊆ NegotiatedScope and GrantedDuration ≤ NegotiatedDuration.
* **INVARIANT-057 & 058 (Admission & Lifecycle Separation):** Only CAPABILITY_ADMISSION_RESULT-v1 and CAPABILITY_LIFECYCLE_EVENT-v1 control state transitions.
* **INVARIANT-059 (Audit Completeness):** Lineage back through the entire BETA-002 and BETA-001 chain must remain unbroken.
