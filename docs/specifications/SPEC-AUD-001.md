# SPEC-AUD-001: Audit & Replay Engine

**Subsystem:** `sovereign-audit`  
**Title:** Audit & Replay Engine  
**Document ID:** `SPEC-AUD-001`  
**Version:** 1.0  
**Status:** Approved / Locked  
**Classification:** Technical Subsystem Contract  
**Owner:** Architecture Review Board  
**Last Updated:** 2026-07-09

---

## 1. Purpose

This specification defines the functional, structural, and behavioral interfaces for the `sovereign-audit` crate.

The Audit & Replay Engine is the deterministic forensic verification layer of Sovereign OS. Operating strictly out-of-band from the production execution path, it reconstructs historical system state by replaying authoritative event streams through the deterministic execution substrate defined by `SPEC-EVT-001`, `SPEC-REG-001`, `SPEC-ASM-001`, and `SPEC-POL-001`.

Its purpose is to mathematically verify that recorded execution history is internally consistent, policy compliant, and free from unauthorized state drift, corruption, or privilege escalation.

The audit subsystem SHALL observe system history without modifying it.

---

## 2. Scope

This specification governs:

- Deterministic historical replay.
- State reconstruction.
- Differential replay verification.
- State drift detection.
- Policy compliance verification.
- Snapshot verification.
- Forensic evidence generation.
- Immutable audit reporting.

This specification explicitly excludes:

- Production event ingestion, delegated to `SPEC-EVT-001`.
- Registry identity generation, delegated to `SPEC-REG-001`.
- Runtime state evolution, delegated to `SPEC-ASM-001`.
- Active authorization enforcement, delegated to `SPEC-POL-001`.
- Discovery sandbox execution, delegated to `SPEC-DSC-001`.

---

## 3. Requirements Matrix

| Requirement ID | Domain Area | Operational Constraint / Behavioral Requirement | Cross-Reference |
|---|---|---|---|
| AUD-REQ-001 | Deterministic Replay | Replay of an identical validated ledger SHALL always reconstruct an identical runtime state. | SPEC-ASM-001 |
| AUD-REQ-002 | Drift Detection | Any divergence between reconstructed and expected state SHALL be reported as explicit audit evidence. | SPEC-ASM-001 |
| AUD-REQ-003 | Policy Verification | Replay SHALL verify that every historical authorization decision remained compliant with `SPEC-POL-001`. | SPEC-POL-001 |
| AUD-REQ-004 | Snapshot Verification | Snapshot checkpoints SHALL be validated against deterministic replay before acceptance. | SPEC-ASM-001 |
| AUD-REQ-005 | Read-Only Operation | The audit engine SHALL operate without mutating the ledger, registry, state machine, or policy engine. | ARCH-008 |
| AUD-REQ-006 | Evidence Production | Every detected anomaly SHALL produce an immutable audit evidence record suitable for archival and review. | HB-CH06 |
| AUD-REQ-007 | Independent Verification | Replay verification SHALL execute independently of production execution threads. | SPEC-EVT-001 |
| AUD-REQ-008 | Fail-Closed Reporting | If verification cannot establish correctness, the audit result SHALL explicitly report verification failure rather than infer correctness. | ARCH-008 |


---

## 4. Architecture

### 4.1 Crate Architecture and Directory Topography

The `sovereign-audit` crate implements the out-of-band forensic verification and differential replay engine.

It operates with strict read-only access to the system substrate, pulling data sequentially from the event ledger, registry, state machine, and policy engine to validate runtime compliance. The directory layout SHALL isolate log parsing and differential state recalculation from forensic report compilation.

```text
crates/sovereign-audit/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── verifier.rs
│   ├── differential.rs
│   ├── history.rs
│   ├── report.rs
│   └── error.rs
├── tests/
│   └── forensic_audit_tests.rs
└── benches/
    └── audit_throughput.rs
```

### 4.2 Observational Isolation and Memory Constraints

The auditing engine MUST operate as a detached, zero-side-effect subsystem.

It SHALL NOT acquire write handles to production ledgers, active registry allocations, or current state machine registers.

Differential replay actions MUST execute inside isolated memory contexts so verification cannot pollute active production state.

---

## 5. Public Interfaces

### 5.1 Public Interface Primitives

```rust
use sovereign_core_asm::{AsmError, StateVector};
use sovereign_ledger::{EventRecord, LedgerError, Lsn};
use sovereign_policy::{PolicyContext, PolicyDecision, PolicyError};
use sovereign_registry::Caid;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ComplianceStatus {
    Verified,
    StateDriftDetected,
    PolicyViolationDetected,
    LineageBreak,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AuditWindow {
    pub start_lsn: Lsn,
    pub end_lsn: Lsn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    pub window: AuditWindow,
    pub status: ComplianceStatus,
    pub last_valid_lsn: Lsn,
    pub detected_divergence_hash: Option<[u8; 32]>,
    pub violated_capabilities: Vec<Caid>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AuditError {
    WindowOutOfBounds,
    LedgerReadFailure(LedgerError),
    AsmReductionFailure(AsmError),
    PolicyEvaluationFailure(PolicyError),
    ReportSerializationStall,
}

pub trait ForensicAuditEngine {
    fn verify_state_consistency(
        &self,
        window: AuditWindow,
    ) -> Result<AuditReport, AuditError>;

    fn audit_policy_compliance(
        &self,
        context: &PolicyContext,
        expected_decision: PolicyDecision,
    ) -> Result<bool, AuditError>;

    fn compile_forensic_package(
        &self,
        report: &AuditReport,
    ) -> Result<Vec<u8>, AuditError>;
}
```

---

## 6. Runtime Invariants

### AUD-INV-001: Absolute Observational Isolation

The `sovereign-audit` engine SHALL remain passive and observational.

No execution thread within this crate SHALL modify, repair, correct, truncate, or append records to the active production ledger, registry allocations, or live state machine vectors.

Any remediation or fault containment MUST be dispatched explicitly to external governance workflows.

### AUD-INV-002: Differential Replay Parity

Feeding an identical slice of sequential log data from the ledger into an isolated shadow instance of the state reduction function MUST yield a `StateVector::root_hash` that matches the production hash at that sequence location bit-for-bit.

Any variance SHALL be treated as structural state drift.

### AUD-INV-003: Post-Hoc Policy Alignment

Re-evaluating a historical `PolicyContext` against the state snapshot synchronized to its matching target LSN MUST yield an identical `PolicyDecision` to the vector emitted during live runtime processing.

### AUD-INV-004: Non-Interference Resource Ceiling

Forensic verification sweeps SHALL NOT degrade hard real-time execution profiles of concurrent production crates.

Audit loops MUST be throttled or bound to low-priority asynchronous processing contexts.

### AUD-INV-005: Immutable Evidence Binding

All forensic audit summaries compiled by the subsystem MUST be serialized using canonical data descriptors.

The identifier for each report payload SHALL be computed from its content hash representation, satisfying `ARCH-006`.


---

## 7. Failure and Boundary Management Models

When forensic verification detects corruption, drift, incomplete evidence, or policy divergence, the `sovereign-audit` engine SHALL report the failure explicitly and remain observational.

The audit engine MUST NOT repair, mutate, truncate, or rewrite production state.

### AUD-FAIL-001: State Drift Detection

If differential replay produces a `StateVector::root_hash` that differs from the expected production state hash, the engine SHALL emit an audit report with:

```text
ComplianceStatus::StateDriftDetected
```

### AUD-FAIL-002: Policy Violation Detection

If historical policy re-evaluation produces a `PolicyDecision` that differs from the decision recorded during live execution, the engine SHALL emit:

```text
ComplianceStatus::PolicyViolationDetected
```

### AUD-FAIL-003: Lineage Break Detection

If a historical object reference cannot be traced through valid `Caid` lineage, the audit engine SHALL emit:

```text
ComplianceStatus::LineageBreak
```

### AUD-FAIL-004: Verification Failure Reporting

If the engine cannot prove correctness because of missing records, corrupted snapshots, unsupported versions, or incomplete evidence, the audit result SHALL report verification failure rather than infer correctness.

---

## 8. Validation Requirements

| Requirement | Verification Method | Expected Result |
|---|---|---|
| AUD-INV-001 | Static boundary audit | Audit code holds no production write handles. |
| AUD-INV-002 | Differential replay test | Recomputed state hashes match expected production hashes. |
| AUD-INV-003 | Policy replay test | Historical policy decisions reproduce identical authorization vectors. |
| AUD-INV-004 | Resource isolation test | Audit sweeps do not interfere with production execution timing. |
| AUD-INV-005 | Evidence binding test | Audit reports generate deterministic content-addressed identifiers. |
| AUD-FAIL-001 | Drift injection test | Artificial state divergence produces `StateDriftDetected`. |
| AUD-FAIL-002 | Policy mismatch test | Conflicting historical authorization produces `PolicyViolationDetected`. |

---

## 9. Dependency Graph

```text
SPEC-EVT-001
        │
        ▼
SPEC-REG-001
        │
        ▼
SPEC-ASM-001
        │
        ▼
SPEC-POL-001
        │
        ▼
SPEC-AUD-001
        │
        ▼
SPEC-DSC-001
```

### Direct Upstream Dependencies

- `SPEC-EVT-001` — Provides authoritative event history.
- `SPEC-REG-001` — Provides content-addressable lineage and object identity.
- `SPEC-ASM-001` — Provides deterministic state reconstruction semantics.
- `SPEC-POL-001` — Provides authorization decision semantics.

### Direct Downstream Consuming Contracts

- `SPEC-DSC-001` — Consumes audit evidence and verification status for sandbox governance.

---

## 10. Future Extension Points

### AUD-EXT-001: Distributed Audit Workers

Future versions MAY introduce distributed audit workers, provided all replay outputs remain deterministic and content-addressed.

### AUD-EXT-002: Cryptographic Proof Packaging

Future versions MAY emit zero-knowledge or Merkle proof packages for compact external verification.

### AUD-EXT-003: Continuous Compliance Windows

Future versions MAY support rolling audit windows over long-running event streams.

---

## Appendix A: Glossary

| Term | Definition |
|---|---|
| Audit Window | Bounded LSN range selected for forensic verification. |
| Differential Replay | Independent replay used to compare recomputed state against expected production state. |
| State Drift | Divergence between expected state and replay-derived state. |
| Forensic Package | Immutable evidence payload summarizing an audit trace. |
| Observational Isolation | Rule that audit logic may inspect but never mutate production state. |

---

## Appendix B: Normative References

- BCP 14 / RFC 2119 — Requirement terminology.
- `SPEC-EVT-001` — Deterministic Append-Only Event Ledger.
- `SPEC-REG-001` — Content-Addressable Registry Service.
- `SPEC-ASM-001` — Deterministic State Machine Runtime.
- `SPEC-POL-001` — Policy Engine.
- `SPEC-DSC-001` — Discovery Layer Runtime.
- `ARCH-006` — Content-addressable architectural identity reference.
- `ARCH-008` — Deterministic execution and isolation reference.

---

## Implementation Lock

This specification is approved and locked as the Phase 16 audit and replay contract.

Downstream implementations MUST NOT weaken, bypass, or reinterpret the audit invariants defined herein.

