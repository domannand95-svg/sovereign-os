# SPEC-ASM-001: Deterministic State Machine Runtime

**Subsystem:** `sovereign-core-asm`  
**Title:** Deterministic State Machine Runtime  
**Document ID:** `SPEC-ASM-001`  
**Version:** 1.0  
**Status:** Approved / Locked  
**Classification:** Technical Subsystem Contract  
**Owner:** Architecture Review Board  
**Last Updated:** 2026-07-09

---

## 1. Purpose

This specification defines the deterministic execution semantics of the `sovereign-core-asm` crate. The State Machine Runtime is the authoritative computational layer of Sovereign OS, transforming the immutable semantic graph maintained by `SPEC-REG-001` through deterministic application of ordered event vectors originating from `SPEC-EVT-001`.

The runtime SHALL execute a mathematically deterministic state transition function, mapping the current system state and a validated event into a successor state while preserving reproducibility across independent execution environments. By isolating state evolution from storage mechanics, policy evaluation, and discovery execution, this subsystem establishes the canonical operational algebra upon which higher-level runtime services depend.

---

## 2. Scope

This specification governs:

- Deterministic state transition execution.
- State vector representation.
- Event reduction semantics.
- Snapshot synchronization boundaries.
- Replay-based state reconstruction.
- Runtime scheduling order.
- State validation interfaces.
- Transition determinism guarantees.

This specification explicitly excludes:

- Persistent storage implementation (deferred to `SPEC-EVT-001`).
- Registry identity generation (deferred to `SPEC-REG-001`).
- Authorization policy evaluation (deferred to `SPEC-POL-001`).
- Discovery runtime execution (deferred to `SPEC-DSC-001`).

---

## 3. Requirements Matrix

| Requirement ID | Domain Area | Operational Constraint / Behavioral Requirement | Cross-Reference |
|---|---|---|---|
| ASM-REQ-001 | State Transition | The runtime SHALL compute deterministic state transitions using the canonical transition function δ(S,E)→S. | SPEC-EVT-001 |
| ASM-REQ-002 | Replay Determinism | Replaying an identical validated event stream SHALL always reconstruct an identical runtime state. | SPEC-EVT-001 |
| ASM-REQ-003 | Registry Consumption | All runtime state mutations SHALL operate exclusively on validated registry objects produced by `SPEC-REG-001`. | SPEC-REG-001 |
| ASM-REQ-004 | Ordering | Events SHALL be reduced strictly according to monotonically increasing LSN ordering. | SPEC-EVT-001 |
| ASM-REQ-005 | Snapshot Alignment | State snapshots SHALL only occur after successful completion of deterministic transition boundaries. | SPEC-EVT-001 |
| ASM-REQ-006 | Isolation | Transition evaluation SHALL be independent of wall-clock time, thread scheduling, allocator behavior, and hardware randomness. | ARCH-008 |
| ASM-REQ-007 | Failure Containment | Invalid transitions SHALL terminate evaluation before any observable mutation becomes externally visible. | SPEC-POL-001 |
| ASM-REQ-008 | State Consistency | Every committed state SHALL satisfy all runtime invariants prior to becoming the active system state. | ARCH-008 |


---

## 4. Architecture

### 4.1 Crate Architecture and Directory Topography

The `sovereign-core-asm` crate implements the core state transition matrix of the platform. It operates as a pure deterministic state reduction layer isolated from I/O execution loops and volatile system side effects.

The directory layout SHALL decouple abstract mathematical transitions from memory-mapped state storage:

```text
crates/sovereign-core-asm/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── engine.rs
│   ├── state.rs
│   ├── reducer.rs
│   ├── snapshot.rs
│   ├── replay.rs
│   └── error.rs
├── tests/
│   └── asm_determinism_tests.rs
└── benches/
    └── transition_overhead.rs
```

### 4.2 Module Responsibility Matrix

| Module | Deterministic Responsibility |
|---|---|
| `lib.rs` | Exposes root crate interfaces and state machine trait definitions. |
| `engine.rs` | Implements the synchronous execution engine and reduction loop supervisor. |
| `state.rs` | Defines in-memory state vector indexing and point-in-time buffers. |
| `reducer.rs` | Implements the deterministic transition function δ. |
| `snapshot.rs` | Handles checkpoint compilation, serialization, and restoration. |
| `replay.rs` | Handles sequential log ingestion and catch-up synchronization. |
| `error.rs` | Defines non-panic state mutation error variants. |

### 4.3 Memory Isolation and State Vector Representation

The active system state SHALL be formalized as an immutable structural index map.

Conforming implementations MUST manage active working state using persistent copy-on-write data structures or transactionally isolated memory pages to ensure that an aborted or invalid transition cannot pollute the parent execution tree.

---

## 5. Public Interfaces

### 5.1 Public Interface Primitives

The state reduction engine SHALL expose the following public primitives and trait interfaces. These components are strictly deterministic and SHALL execute independently of any underlying multi-threaded task scheduler.

```rust
use sovereign_ledger::{EventRecord, LedgerError, Lsn};
use sovereign_registry::{Caid, ObjectClass};

/// Opaque deterministic handle representing a verified internal system state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateVector {
    pub root_hash: [u8; 32],
    pub last_applied_lsn: Lsn,
}

/// Strongly typed classifications of state reduction failures.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AsmError {
    InvalidInitialState,
    StateVectorMismatch,
    TransitionAborted,
    SnapshotDivergence,
    LedgerIngestFailed(LedgerError),
    UnregisteredCapability(Caid),
    UnsupportedObjectClass(ObjectClass),
    OutOfOrderLsn,
}

/// Core abstract state machine operator executing the transition function δ.
pub trait StateMachineEngine {
    /// Initializes the state vector from a hardcoded system genesis block.
    fn initialize_genesis(&mut self) -> Result<StateVector, AsmError>;

    /// Executes a single atomic state transition step: δ(S_n, E_n+1) -> S_n+1.
    /// If evaluation fails, the active state vector MUST remain unmodified.
    fn apply_event(
        &mut self,
        current_state: &StateVector,
        event: &EventRecord,
    ) -> Result<StateVector, AsmError>;

    /// Processes an ordered stream of records sequentially and returns the final state vector.
    fn replay_stream<'a, I>(
        &mut self,
        initial_state: &StateVector,
        events: I,
    ) -> Result<StateVector, AsmError>
    where
        I: IntoIterator<Item = &'a EventRecord<'a>>;

    /// Compacts active memory into a snapshot block at the designated LSN boundary.
    fn emit_snapshot(&self, state: &StateVector) -> Result<Vec<u8>, AsmError>;

    /// Restores the state machine indexing surface from a raw snapshot block.
    fn restore_from_snapshot(
        &mut self,
        snapshot_payload: &[u8],
    ) -> Result<StateVector, AsmError>;
}
```

---

## 6. Runtime Invariants

Conforming implementations of `SPEC-ASM-001` SHALL enforce the following operational invariants at the runtime execution boundary.

### ASM-INV-001: Pure Reduction Identity

The state transition function δ(S_n, E_n+1) MUST behave as a pure mathematical function.

Given an identical state vector S_n and identical event payload E_n+1, the engine SHALL return an identical resulting state vector S_n+1 across all conforming environments, independent of target CPU topology, compiler optimization level, or local endianness profile.

### ASM-INV-002: Error State Non-Mutation

If an incoming event triggers an execution exception, structural validation failure, or unmapped state error, the engine SHALL discard all transient memory modifications accrued during the active evaluation step and return the exact originating state vector.

```text
δ(S_n, E_invalid) = S_n
```

### ASM-INV-003: Snapshot Equivalence Identity

Initializing the engine from a validated snapshot payload compiled at LSN_k, followed by sequential replay of events from LSN_k+1 to LSN_n, MUST produce a state vector byte-for-byte identical to a complete uninterrupted ledger replay from system genesis S_0 to LSN_n.

### ASM-INV-004: Clock and External Side-Effect Exclusion

The state machine reduction engine SHALL NOT read from host operating system clocks, hardware random number sources, concurrent thread pools, or unaligned memory addresses during evaluation of δ.

All inputs required for state evolution MUST be derived exclusively from fields explicitly serialized within the incoming `EventRecord` payload.

### ASM-INV-005: Linear Monotonic Advancement

The state transition engine SHALL refuse execution requests for any event whose assigned Log Sequence Number is less than or equal to the `last_applied_lsn` watermark bound within the active `StateVector`.

Out-of-order execution processing is strictly prohibited.

### ASM-INV-006: Registry-Bound Mutation

The state machine SHALL NOT apply transitions against unregistered, unknown, or schema-invalid registry objects.

All capability, policy, workflow, dataset, and evidence references consumed during state reduction MUST resolve through `SPEC-REG-001`.

### ASM-INV-007: Snapshot Boundary Integrity

A snapshot SHALL only be emitted after a fully successful state transition boundary.

Snapshots MUST NOT represent partially evaluated, failed, speculative, or rolled-back state transitions.


---

## 7. Failure and Boundary Management Models

When a runtime execution boundary or mathematical verification threshold is breached, the `sovereign-core-asm` engine SHALL reject arbitrary fallback paths or soft degradation modes. It MUST execute an immediate transition to a fail-closed containment envelope to safeguard state vector integrity.

### 7.1 Deterministic Exception Handling

#### ASM-FAIL-001: State Divergence Containment

If a local state reduction execution yields a `StateVector::root_hash` that deviates from the expected cryptographic signature committed within an upstream consensus or ledger checkpoint, the engine SHALL immediately emit `AsmError::SnapshotDivergence`.

The active engine node MUST discard all uncommitted mutations, halt the execution pipeline, and isolate its state memory to prevent cascading corruption.

#### ASM-FAIL-002: Capability Panic Isolation

If an active capability invokes an operation that triggers an unmanaged runtime panic or out-of-bounds memory trap, the engine transaction supervisor SHALL intercept the signal at the crate boundary.

The engine MUST discard all transient memory alterations accumulated during the current evaluation cycle, revoke the active transition, and emit `AsmError::TransitionAborted`, restoring the originating state vector.

#### ASM-FAIL-003: Snapshot Corruption Recovery

If the engine detects an invalid checksum or unmapped type definition while parsing a snapshot payload during initialization, it SHALL emit `AsmError::InvalidInitialState`.

Boot progression from the corrupted snapshot SHALL be refused.

Recovery SHALL proceed exclusively by replaying the authoritative event ledger beginning at system genesis (`S₀`).

#### ASM-FAIL-004: Resource Exhaustion Bound

If an active transition exceeds configured execution resource limits, including stack depth, execution budget, or memory allocation ceiling, the engine supervisor SHALL terminate the active transition before commit.

The implementation MUST restore the last validated state vector without exposing a partially evaluated state.

---

## 8. Validation Requirements

Conforming implementations SHALL validate all behavioral properties and runtime invariants before promotion within the Sovereign OS engineering hierarchy.

### 8.1 Traceability and Verification Matrix

| Requirement | Verification Method | Expected Result |
|---|---|---|
| ASM-INV-001 | Differential parity testing | Identical event streams produce identical state hashes across supported architectures. |
| ASM-INV-002 | Fault-injection testing | Failed transitions leave the originating `StateVector` unchanged. |
| ASM-INV-003 | Replay conformance testing | Snapshot replay and full genesis replay converge to identical state vectors. |
| ASM-INV-004 | Static isolation analysis | No prohibited nondeterministic dependencies are present. |
| ASM-INV-005 | Negative property testing | Out-of-order LSN ingestion is rejected. |
| ASM-FAIL-001 | State divergence injection | Divergent state hashes terminate execution immediately. |

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
   ┌────┼────┐
   ▼    ▼    ▼
SPEC-POL-001
SPEC-AUD-001
SPEC-DSC-001
```

### Direct Upstream Dependencies

- `SPEC-EVT-001` — Provides monotonic `Lsn` ordering and authoritative event serialization.
- `SPEC-REG-001` — Provides `Caid` identities and validated registry objects.

### Direct Downstream Dependencies

- `SPEC-POL-001` — Consumes the active runtime state for authorization evaluation.
- `SPEC-AUD-001` — Verifies historical state reconstruction.
- `SPEC-DSC-001` — Consumes deterministic snapshots for isolated execution environments.

---

## 10. Future Extension Points

### ASM-EXT-001: Isolated Sub-Graph Sharding

Future implementations MAY support concurrent reduction across independent registry partitions, provided deterministic ordering and invariant preservation remain intact.

### ASM-EXT-002: Cryptographic State Acceleration

Hardware-assisted hashing, Merkle computation, or zero-knowledge acceleration MAY be introduced through compatible interfaces without modifying the deterministic transition semantics defined by this specification.

---

## Appendix A: Glossary

| Term | Definition |
|---|---|
| Abstract State Machine | Formal computational model describing deterministic state evolution. |
| Copy-on-Write | Memory isolation strategy preserving the originating state during speculative evaluation. |
| State Vector | Canonical representation of complete runtime state at a specific LSN. |
| Transition Function | Deterministic mapping from state and event to successor state. |

---

## Appendix B: Normative References

- BCP 14 / RFC 2119 — Requirement terminology.
- SPEC-EVT-001 — Deterministic Append-Only Event Ledger.
- SPEC-REG-001 — Content-Addressable Registry Service.
- IEEE 754-2019 — Floating-point arithmetic (referenced to exclude nondeterministic dependence).

---

## Implementation Lock

This specification is approved and locked as the Phase 16 deterministic execution contract.

Downstream implementations MUST NOT weaken, bypass, or reinterpret the execution invariants defined herein.

