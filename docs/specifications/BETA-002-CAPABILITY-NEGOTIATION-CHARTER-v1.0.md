# BETA-002-CAPABILITY-NEGOTIATION-CHARTER-v1.0.md

**Status:** APPROVED / EXPERIMENTAL TRACK CHARTER
**Class:** ARCHITECTURAL SPECIFICATION & GOVERNANCE BOUNDARY
**Scope:** Chartering the pre-authority capability request, evaluation, scope reduction, and trust negotiation layer (`BETA-002`).

---

## 1. Executive Summary

BETA-001 established the **Authority Governance Spine** (provenance, traversal, cryptographic verification, and historical audit reconstruction). BETA-002 establishes the **Authority Emergence and Consent Spine**. 

The fundamental architectural question of BETA-002 is:
> *"Before authority exists, how does a system request, justify, constrain, and receive capability access safely?"*

---

## 2. Core Invariants & Boundaries

1. **Request $\neq$ Authorization:** Emitting a request object creates zero execution capability.
2. **Minimum Necessary Authority (MNA):** The system evaluation engine is structurally obligated to down-scope requested privileges to the narrowest viable functional bounds.
3. **Upstream Integration:** Negotiated grants feed directly into the BETA-001 evidence chain (`EFFECT_PROPOSAL-v1` -> `CAPABILITY_ADMISSION_RESULT-v1`), ensuring seamless cryptographic lineage from initial request to final audit record.

---

## 3. Proposed Experimental Sequence

* **EXP-BETA-010 (Capability Request Object Boundary):** Define the inert representation of an agent requesting a capability (`CAPABILITY_REQUEST-v1`).
* **EXP-BETA-011 (Request Evaluation Boundary):** Determine eligibility, necessity, and policy compliance (`REQUEST_EVALUATION_RESULT-v1`).
* **EXP-BETA-012 (Scope Reduction & Negotiation):** Automatically down-scope overly broad resource paths or durations to meet MNA bounds.
* **EXP-BETA-013 (Temporary Capability Grant Boundary):** Bind negotiated grants into the BETA-001 admission pipeline.

---

## 4. Non-Goals

* Unconstrained dynamic privilege elevation.
* Ambient agent self-authorization.
* Direct runtime bypass of the evidence ledger.
