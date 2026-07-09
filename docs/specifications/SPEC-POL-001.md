# SPEC-POL-001: Policy Engine

**Subsystem:** `sovereign-policy`  
**Title:** Policy Engine  
**Document ID:** `SPEC-POL-001`  
**Version:** 1.0  
**Status:** Approved / Locked  
**Classification:** Technical Subsystem Contract  
**Owner:** Architecture Review Board  
**Last Updated:** 2026-07-09

---

## 1. Purpose

This specification defines the functional, structural, and behavioral interfaces for the `sovereign-policy` crate.

The Policy Engine is the deterministic authorization layer of Sovereign OS. It evaluates capability requests against the active state vector produced by `SPEC-ASM-001`, registry identities defined by `SPEC-REG-001`, and immutable historical context anchored by `SPEC-EVT-001`.

The engine SHALL emit explicit authorization decisions without panic paths, hidden mutation, or nondeterministic fallback behavior. Its purpose is to enforce capability boundaries through constant-time, fail-closed policy evaluation before any privileged operation can cross into an execution boundary.

---

## 2. Scope

This specification governs:

- Capability authorization evaluation.
- Policy object interpretation.
- Decision vector emission.
- Constant-time access control checks.
- State-bound policy evaluation.
- Capability Firewall integration.
- Fail-closed authorization behavior.
- Policy replay and audit compatibility.

This specification explicitly excludes:

- Event persistence, deferred to `SPEC-EVT-001`.
- Registry identity generation, deferred to `SPEC-REG-001`.
- State transition execution, deferred to `SPEC-ASM-001`.
- Historical forensic replay, deferred to `SPEC-AUD-001`.
- Hostile sandbox execution, deferred to `SPEC-DSC-001`.

---

## 3. Requirements Matrix

| Requirement ID | Domain Area | Operational Constraint / Behavioral Requirement | Cross-Reference |
|---|---|---|---|
| POL-REQ-001 | Authorization Decision | The policy engine SHALL emit exactly one explicit authorization vector for every evaluated capability request. | SPEC-ASM-001 |
| POL-REQ-002 | Decision Taxonomy | Authorization output MUST be one of four Version 1.0 decision classes: `ALLOW`, `DENY`, `THROTTLE`, or `QUARANTINE`. | SPEC-CF-001 |
| POL-REQ-003 | Constant-Time Evaluation | Policy decision evaluation SHALL avoid secret-dependent branching, timing leakage, and variable-time authorization paths where capability privilege data is involved. | ARCH-008 |
| POL-REQ-004 | Registry Binding | All capability, policy, workflow, and evidence identifiers consumed during evaluation MUST resolve to valid `Caid` entries in `SPEC-REG-001`. | SPEC-REG-001 |
| POL-REQ-005 | State Binding | Policy evaluation SHALL be performed against an explicit `StateVector` produced by `SPEC-ASM-001`. | SPEC-ASM-001 |
| POL-REQ-006 | Fail-Closed Default | Missing, malformed, expired, unsupported, or ambiguous policies MUST resolve to `DENY` or `QUARANTINE`; they SHALL NOT resolve to `ALLOW`. | SPEC-CF-001 |
| POL-REQ-007 | No Hidden Mutation | Policy evaluation SHALL NOT mutate registry state, ledger history, or active state vectors. | SPEC-ASM-001 |
| POL-REQ-008 | Auditability | Every policy decision MUST be serializable into an audit-compatible evidence record for later validation by `SPEC-AUD-001`. | SPEC-AUD-001 |


---

## 4. Architecture

### 4.1 Crate Architecture and Directory Topography

The `sovereign-policy` crate implements the constant-time, stateless evaluation engine for access control and capability authorization. It acts as the algorithmic enforcement mechanism for the Capability Firewall, consuming immutable state configurations to yield explicit authorization vectors.

The directory layout SHALL isolate the policy abstract syntax tree (AST) from the hot evaluation loops.

```text
crates/sovereign-policy/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── engine.rs
│   ├── ast.rs
│   ├── eval.rs
│   ├── cache.rs
│   └── error.rs
├── tests/
│   └── policy_enforcement_tests.rs
└── benches/
    └── evaluation_latency.rs
```

### 4.2 Computational Boundedness and Memory Constraints

The evaluation hot path MUST execute with deterministic memory consumption and deterministic execution latency.

Dynamic heap allocation, unbounded recursion, and variable-length iterative execution are prohibited within the authorization hot path.

---

## 5. Public Interfaces

### 5.1 Public Interface Primitives

```rust
use sovereign_core_asm::StateVector;
use sovereign_ledger::Lsn;
use sovereign_registry::Caid;

/// Authorization vectors emitted by the policy engine.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny,
    Throttle { microsecond_delay: u32 },
    Quarantine,
}

/// Snapshot context used during authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyContext {
    pub actor_caid: Caid,
    pub target_capability: Caid,
    pub operation_verb: u32,
    pub active_state: StateVector,
}

/// Policy evaluation failures.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PolicyError {
    RuleSyntaxInvalid(Caid),
    ContextResolutionFailure,
    EvaluationTimeout,
    CacheSynchronizationStall,
    EvaluationDepthExceeded,
}

/// Core authorization interface.
pub trait PolicyEvaluationEngine {
    fn refresh_policy_set(
        &mut self,
        state: &StateVector,
    ) -> Result<(), PolicyError>;

    fn evaluate_authorization(
        &self,
        context: &PolicyContext,
    ) -> Result<PolicyDecision, PolicyError>;

    fn purge_evaluation_cache(
        &mut self,
    ) -> Result<u32, PolicyError>;
}
```

---

## 6. Runtime Invariants

### POL-INV-001: Algorithmic Complexity Bound

The authorization engine SHALL execute within a statically bounded worst-case execution path whose upper bound is independent of runtime state growth.

Evaluation depth limits MUST prevent cyclic or recursively expanding authorization graphs.

### POL-INV-002: Fail-Closed Default Routing

If policy evaluation encounters an unmapped rule, schema violation, resource exhaustion event, or ambiguous authorization condition, evaluation SHALL immediately terminate and return:

```text
PolicyDecision::Deny
```

No partially evaluated authorization context SHALL resolve to a permissive decision.

### POL-INV-003: I/O and Side-Effect Isolation

The authorization engine SHALL behave as a pure deterministic projection.

It SHALL NOT perform network I/O, filesystem access, hardware inspection, or any external side effects during evaluation.

All authorization inputs SHALL originate exclusively from the immutable `PolicyContext`.

### POL-INV-004: Point-in-Time Snapshot Locking

Authorization SHALL execute against exactly one immutable `StateVector`.

Concurrent state updates SHALL NOT influence an evaluation already in progress.

### POL-INV-005: Zero Dynamic Allocation

The authorization hot path SHALL perform zero dynamic heap allocations after initialization.

Lookup tables, memoization structures, and decision trees SHALL reside within preallocated memory regions established during initialization or synchronization.


---

## 7. Failure and Boundary Management Models

When a runtime security boundary, structural constraint, or resource ceiling is breached, the `sovereign-policy` engine SHALL reject arbitrary fallback configurations or soft mitigation pathways.

It MUST execute a deterministic transition to a fail-closed containment state to ensure that the integrity of the Capability Firewall remains uncompromised.

### 7.1 Deterministic Exception Handling

#### POL-FAIL-001: Syntax Trapping and Schema Violations

If a policy rule or access control declaration ingested from the active `StateVector` exhibits invalid syntax, unresolved identifiers, or schema mismatches, the engine SHALL emit `PolicyError::RuleSyntaxInvalid`.

The affected capability namespace MUST resolve through default fail-closed routing.

#### POL-FAIL-002: Context Resolution Exhaustion

If an incoming `PolicyContext` contains unresolvable entity identifiers, mismatched cryptographic signatures, or missing provenance data, the engine SHALL abort processing and emit `PolicyError::ContextResolutionFailure`.

The system MUST NOT speculate on the intended permissions of anonymous or detached actors.

#### POL-FAIL-003: Evaluation Bound Control

If AST traversal exceeds configured execution limits or maximum evaluation step count, the evaluation supervisor SHALL terminate the active evaluation before any permissive decision is emitted.

The engine SHALL emit `PolicyError::EvaluationDepthExceeded`.

#### POL-FAIL-004: Cache Coherence Corruption

If the lock-free memoization cache encounters synchronization stalls, unexpected hash collisions, or byte-level memory degradation, the engine SHALL emit `PolicyError::CacheSynchronizationStall`.

The subsystem MUST bypass the cache layer, execute authorization through the cold evaluation path, and schedule an out-of-band cache flush.

---

## 8. Validation Requirements

Conforming implementations of the `sovereign-policy` crate SHALL validate all core functional behavior, algorithmic limits, and performance invariants before code promotion and integration testing.

### 8.1 Traceability and Verification Matrix

| Requirement | Verification Method | Expected Result |
|---|---|---|
| POL-INV-001 | Property-based fuzz testing | Synthetic policy AST structures remain within configured bounded execution limits. |
| POL-INV-002 | Negative fault injection | Corrupted, zeroed, or fragmented contexts resolve to `PolicyDecision::Deny`. |
| POL-INV-003 | Static boundary audit | No network socket binds, file descriptor acquisitions, or host environment lookups are present in the hot path. |
| POL-INV-004 | Concurrent mutation testing | Parallel event writes do not alter an in-flight decision vector. |
| POL-INV-005 | Heap allocation profiling | Continuous `evaluate_authorization` cycles produce zero dynamic heap allocations after initialization. |
| POL-FAIL-001 | Negative integration testing | Malformed policy schemas trigger clean fail-closed containment without panic. |

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
SPEC-DSC-001
```

### Direct Upstream Dependencies

- `SPEC-EVT-001` — Imposes monotonic log structures and foundational serializability properties.
- `SPEC-REG-001` — Provides immutable `Caid` identifiers and object definitions used to build authorization context.
- `SPEC-ASM-001` — Establishes the authoritative point-in-time `StateVector` snapshot that anchors rule evaluation.

### Direct Downstream Consuming Contracts

- `SPEC-DSC-001` — Consumes emitted `PolicyDecision` vectors to isolate, throttle, or quarantine speculative agent tool execution inside the sandbox runtime.
- `SPEC-AUD-001` — Consumes serialized policy decisions for historical compliance verification.

---

## 10. Future Extension Points

### POL-EXT-001: Hierarchical Attribute Delegation

The `PolicyContext` layout MAY be extended to support cryptographically signed delegated capability tokens.

Such extensions MUST preserve fail-closed evaluation, deterministic decision emission, and point-in-time state binding.

### POL-EXT-002: Sandboxed Bytecode Evaluation

The declarative AST parser MAY be replaced or supplemented by a deterministic sandboxed bytecode evaluation runner.

Permitted candidates MAY include deterministic WebAssembly profiles or localized eBPF-style micro-kernels, provided they preserve bounded execution, zero post-initialization allocation, and side-effect isolation.

---

## Appendix A: Glossary

| Term | Definition |
|---|---|
| Abstract Syntax Tree | Tree representation of policy rules where nodes represent operands, operators, predicates, or structural tokens. |
| Authorization Vector | Definitive execution command emitted by the policy engine, such as `Allow`, `Deny`, `Throttle`, or `Quarantine`. |
| Capability Firewall | Enforcement boundary that consumes policy decisions before privileged execution. |
| Memoization Cache | Preallocated in-memory index used to reuse deterministic evaluation outcomes for identical input contexts. |
| Policy Context | Complete authorization input containing the actor, target capability, operation verb, and point-in-time state vector. |

---

## Appendix B: Normative References

- BCP 14 / RFC 2119 — Requirement terminology.
- `SPEC-EVT-001` — Deterministic Append-Only Event Ledger.
- `SPEC-REG-001` — Content-Addressable Registry Service.
- `SPEC-ASM-001` — Deterministic State Machine Runtime.
- `SPEC-CF-001` — Capability Firewall execution contract.
- `SPEC-AUD-001` — Audit and Replay Engine.
- `SPEC-DSC-001` — Discovery Layer Runtime.

---

## Implementation Lock

This specification is approved and locked as the Phase 16 policy enforcement contract.

Downstream implementations MUST NOT weaken, bypass, or reinterpret the policy invariants defined herein.

