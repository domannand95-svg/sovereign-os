# EXP-BETA-010-Capability-Request-Boundary-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** NON-PRODUCTION / PRE-AUTHORITY GOVERNANCE
**Scope:** Defines the normative schema and security boundaries for CAPABILITY_REQUEST-v1.

---

## 1. Executive Summary
EXP-BETA-010 defines the initial pre-authority primitive (CAPABILITY_REQUEST-v1). This object permits agents or runtime components to express structured intent for capability access without holding, implying, or generating execution authority.

---

## 2. Normative Invariants
* **INVARIANT-037 (Request Non-Authority):** A capability request can never directly authorize execution or produce a CapabilityPermission.
* **INVARIANT-038 (Request Immutability):** Once instantiated, request identifiers and intent parameters are immutable.
* **INVARIANT-039 (Scope Separation):** Requested scopes are strictly distinct from granted scopes.
* **INVARIANT-040 (Credential Exclusion):** Schemas reject execution tokens, API keys, or permission grants.
* **INVARIANT-041 (Evidence Separation):** Supporting evidence references provide context only and cannot elevate request authority.
