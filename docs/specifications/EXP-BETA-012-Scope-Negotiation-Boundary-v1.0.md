# EXP-BETA-012-Scope-Negotiation-Boundary-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** NON-PRODUCTION / PRE-AUTHORITY GOVERNANCE
**Scope:** Defines the normative schema and security boundaries for SCOPE_NEGOTIATION_RESULT-v1.

---

## 1. Executive Summary
EXP-BETA-012 defines the Minimum Necessary Authority (MNA) scope negotiation boundary (SCOPE_NEGOTIATION_RESULT-v1). This artifact records the deterministic down-scoping of a broad capability request into the narrowest sufficient operational resource path and duration. Negotiation can reduce or equal requested scope; it can never expand privilege.

---

## 2. Normative Invariants
* **INVARIANT-047 (Scope Non-Expansion):** NegotiatedScope ⊆ RequestedScope. Privilege expansion is structurally prohibited.
* **INVARIANT-048 (Minimum Necessary Authority):** The negotiated result represents the smallest sufficient authority boundary for the declared purpose.
* **INVARIANT-049 (Negotiation Non-Authority):** A negotiated scope cannot directly create capability permissions or execution authority.
* **INVARIANT-050 (Evaluation Dependency):** A scope negotiation must reference a valid REQUEST_EVALUATION_RESULT-v1.
* **INVARIANT-051 (Deterministic Reduction):** Equivalent inputs under equivalent constraints produce equivalent negotiated scopes.
* **INVARIANT-052 (Immutable Negotiation Record):** Negotiation outcomes are append-only evidence.
