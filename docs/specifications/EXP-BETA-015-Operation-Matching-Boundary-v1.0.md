# EXP-BETA-015-Operation-Matching-Boundary-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** NON-PRODUCTION / RUNTIME GOVERNANCE
**Scope:** Defines normative schemas (OPERATION_MATCH_REQUEST-v1 and OPERATION_BOUNDARY_RESULT-v1) and strict operation containment constraints.

---

## 1. Executive Summary
EXP-BETA-015 establishes the exact operation matching boundary (OPERATION_MATCH_REQUEST-v1 and OPERATION_BOUNDARY_RESULT-v1). This subsystem verifies whether a requested operation and resource pair is strictly contained within an admitted capability's granted boundary without permitting privilege expansion or ambiguous interpretation.

---

## 2. Normative Invariants
* **INVARIANT-064 (Exact Operation Containment):** A capability authorizing operation A does not automatically authorize operation B.
* **INVARIANT-065 (No Scope Expansion Through Matching):** The matcher confirms containment only and can never produce a requested scope exceeding the granted scope.
* **INVARIANT-066 (Deterministic Boundary Resolution):** Equivalent capability state, operation requests, and resource contexts produce identical matching outcomes.
* **INVARIANT-067 (Ambiguity Fails Closed):** Wildcard failures, unknown resource classes, or incomplete parameter constraints result in strict denial.
