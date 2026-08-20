# EXP-BETA-Integration-Decision-Record-v1.0.md

**Status:** APPROVED / ARCHITECTURAL DECISION RECORD
**Class:** NON-PRODUCTION / INTEGRATION DISPOSITION
**Scope:** Final architectural ownership disposition, promotion decisions, and migration constraints for the EXP-BETA-004 through EXP-BETA-009 governance architecture.

---

## 1. Review Scope

This decision record formalizes the architectural review of the experimental governance sequence (`EXP-BETA-004` through `EXP-BETA-009`). It evaluates how the frozen non-production evidence contracts and the read-only audit replay engine map onto existing Sovereign OS crates, ensuring that no experimental artifact compromises core authority paths or introduces ambient privilege.

---

## 2. Existing Architecture Baseline

A preliminary survey of the Sovereign OS repository structure indicates the following baseline subsystems:
* sovereign-intent / proposal handling
* sovereign-policy / rule enforcement
* sovereign-compiler / payload translation
* sovereign-registry / capability admission (Registry v2)
* sovereign-ledger / state persistence
* sovereign-audit / observation and verification
* beta001-harness / experimental verification and fuzzing

---

## 3. Contract Ownership Decisions

The mapping between experimental evidence contracts and prospective production ownership crates is established as follows:

| Experimental Contract | Proposed Owner Crate | Ownership & Integration Rationale |
| :--- | :--- | :--- |
| `EFFECT_PROPOSAL-v1` | `sovereign-intent` | Encapsulates unverified model output into a structured, non-authoritative intent object. |
| `EFFECT_EVALUATION_RESULT-v1` | `sovereign-policy` | Records host evaluation outcomes, separating policy inspection from privilege grant. |
| `CAPABILITY_COMPILATION_RESULT-v1` | `sovereign-compiler` | Manages the deterministic translation of approved evaluations into `CapabilityPayloadV1` candidates. |
| `CAPABILITY_ADMISSION_RESULT-v1` | `sovereign-registry` | Interfaces with Registry v2 gates (1–6) to record admission outcomes without embedding runtime tokens. |
| `CAPABILITY_LIFECYCLE_EVENT-v1` | `sovereign-ledger` | Enforces append-only persistence of operational status transitions (active, suspended, expired, revoked, inert). |
| EXP-BETA-009 Replay Engine | `sovereign-audit` | Provides read-only graph traversal, cryptographic ancestry hashing, and point-in-time state reconstruction. |

---

## 4. Production Integration Boundaries

To maintain the absolute separation of concerns established during the experimental phase, production integration must adhere to three rigid boundaries:

1. **Strict Audit Isolation:** The audit replay engine in `sovereign-audit` must operate as a read-only query service. It cannot be called inline during high-performance execution paths or invoked to bypass registry admission checks.
2. **Explicit Lineage Validation:** Every state mutation recorded in `sovereign-ledger` must carry cryptographically verified ancestry linking back to its origin proposal. Orphan records are rejected at the storage layer.
3. **Zero-Authority Evidence:** Evidence objects (`EFFECT_PROPOSAL-v1`, `CAPABILITY_ADMISSION_RESULT-v1`, `CAPABILITY_LIFECYCLE_EVENT-v1`, etc.) remain inert audit structures and can never be coerced into acting as direct execution credentials.

---

## 5. Promotion Decisions

Each experimental artifact is assigned a definitive promotion disposition:

| Artifact / Subsystem | Promotion Disposition | Action Required Prior to Core Integration |
| :--- | :--- | :--- |
| `EFFECT_PROPOSAL-v1` schema | **PROMOTE** | Wrap in strongly typed Rust structs with zero-copy deserialization. |
| `EFFECT_EVALUATION_RESULT-v1` schema | **PROMOTE** | Integrate with `sovereign-policy` evaluation pipelines. |
| `CAPABILITY_COMPILATION_RESULT-v1` schema | **PROMOTE** | Bind cryptographic hash pointers to compiler output structs. |
| `CAPABILITY_ADMISSION_RESULT-v1` schema | **ADAPT** | Harmonize field definitions with existing Registry v2 gate error structures. |
| `CAPABILITY_LIFECYCLE_EVENT-v1` schema | **PROMOTE** | Implement as append-only log entries in `sovereign-ledger`. |
| EXP-BETA-009 Replay Engine | **PROMOTE** | Port traversal algorithms into `sovereign-audit` as a standalone verification service. |
| Experimental JSON Fixtures | **RETAIN-EXPERIMENTAL** | Keep inside `beta001-harness/tests/fixtures/` for ongoing regression testing. |

---

## 6. Deferred Research Areas

The following advanced topics are explicitly deferred to post-integration research phases:
* Distributed consensus hashing across federated Sovereign OS nodes.
* Automated zero-knowledge proof generation for privacy-preserving audit traversal.
* Dynamic runtime garbage collection of historical evidence trees.

---

## 7. Migration Constraints

* **No Retroactive Mutation:** Existing ledger records cannot be retroactively modified to fit new schema versions. Multi-version decoders must be maintained in `sovereign-audit`.
* **Feature Flag Gating:** All evidence-chain verification logic must be deployed behind cargo feature flags during initial canary integration phases.

---

## 8. Rollback Strategy

In the event of a validation anomaly or performance degradation during core crate integration:
* Disable the `governance-evidence-chain` feature flag to revert to baseline direct-dispatch logic.
* Retain the append-only evidence log for post-mortem analysis without disrupting host operational continuity.

---

## 9. Final Approval Gate

* **EXP-BETA Experimental Track:** COMPLETED & FROZEN
* **Integration Decision Record:** FORMALIZED
* **Core Code Modification:** AUTHORIZED SOLELY FOR SCAFFOLDING IN ISOLATED BRANCHES (`spec/exp-beta-004-effect-proposal-boundary`)
