# SPEC-REG-001: Content-Addressable Registry Service

**Subsystem:** `sovereign-registry`  
**Document ID:** `SPEC-REG-001`  
**Version:** 1.0  
**Status:** Approved / Locked  
**Classification:** Technical Subsystem Contract  
**Owner:** Architecture Review Board  
**Last Updated:** 2026-07-09  

---

## 1. Purpose

This specification defines the functional, structural, and behavioral interfaces for the `sovereign-registry` crate.

The Registry Service acts as the authoritative semantic knowledge layer for Sovereign OS, translating sequential event vectors from the underlying ledger (`SPEC-EVT-001`) into a structured, immutable Directed Acyclic Graph (DAG). By binding system identities strictly to cryptographic payloads, this subsystem eliminates name collision vulnerabilities, configuration drift, and file-system dependency loops.

It provides higher-level operating system layers with a mathematically verifiable provenance map of all active capabilities, policies, specifications, and execution evidence records.

---

## 2. Scope

The specifications detailed within this document govern:

- Content-Addressable Identifier (CAID) generation
- Schema validation protocols for first-class object classifications
- Topological sort mechanics for relational dependency tracking
- Query surfaces exposed by the graph runtime
- In-memory index structures
- Relational consistency gates
- State-reconstruction interfaces of the `sovereign-registry` crate

This specification explicitly excludes:

- Physical disk-write optimizations, deferred to memory-mapped storage backends
- Runtime execution enforcement mechanisms, deferred to `SPEC-POL-001`

---

## 3. Requirements Matrix

| Requirement ID | Domain Area | Operational Constraint / Behavioral Requirement | Cross-Reference |
|---|---|---|---|
| REG-REQ-001 | Identity Generation | The registry subsystem SHALL compute the Content-Addressable Identifier (CAID) for any object uniquely and deterministically using a cryptographically secure hashing algorithm mandated by project governance, such as BLAKE3, executing over its canonical, deterministic serialized byte stream. | ARCH-006 |
| REG-REQ-002 | Classification Gating | The runtime MUST enforce strict schema boundaries for the Version 1.0 taxonomy, consisting of exactly eight first-class object types: Capabilities, Specifications, Policies, Events, Datasets, Workflows, Verification Artifacts, and Evidence Packages. | HB-CH06 |
| REG-REQ-003 | Topological Order | The graph index engine SHALL track directed relations as typed, un-mutated edges. It MUST detect, log, and explicitly reject any operation that introduces circular references or cycles into the graph. | HB-CH06 |
| REG-REQ-004 | Lineage Enclosure | Every registered object entry MUST include an immutable provenance array detailing parent CAID signatures, establishing an unbroken chain of custody back to system genesis. | ARCH-007 |
| REG-REQ-005 | Location Neutrality | The primary indexing mechanism SHALL execute independently of physical filenames, disk offsets, network node addresses, or runtime file-system organization models. | ARCH-006 |
| REG-REQ-006 | Stream Reconstitution | The registry state engine SHALL be fully capable of reconstructing its entire topological index by replaying valid records compiled sequentially from the authoritative event log crate. | SPEC-EVT-001 |
| REG-REQ-007 | Query Isolation | Read operations, graph projections, and relation traversals SHALL operate via lock-free pointer structures, ensuring zero blocking interference on concurrent verification threads. | HB-CH02 |
| REG-REQ-008 | Delta Enforcement | Modifications to existing registry nodes SHALL NOT mutate historical data segments. All updates MUST manifest as append-only additions of new, distinct nodes holding independent CAID keys. | HB-CH06 |


---

## 4. Crate Architecture and Directory Topography

The `sovereign-registry` crate SHALL maintain a strictly modular internal architecture separating graph storage, node identity, relation typing, schema validation, indexing, traversal, lineage verification, and ledger reconstruction.

The crate SHALL use the following repository structure:

```text
crates/sovereign-registry/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── graph.rs
│   ├── node.rs
│   ├── edge.rs
│   ├── caid.rs
│   ├── schema.rs
│   ├── index.rs
│   ├── traversal.rs
│   ├── replay.rs
│   ├── lineage.rs
│   ├── error.rs
│   └── config.rs
├── tests/
│   └── registry_conformance_tests.rs
└── benches/
    └── traversal_benchmarks.rs
```

### 4.1 Module Responsibility Matrix

| Module | Deterministic Responsibility |
|---|---|
| `lib.rs` | Exposes the public registry trait, core types, and crate-level documentation. |
| `graph.rs` | Owns the immutable Directed Acyclic Graph representation and insertion gate. |
| `node.rs` | Defines canonical registry node structures and object metadata. |
| `edge.rs` | Defines typed parent-child relations between registry objects. |
| `caid.rs` | Implements deterministic Content-Addressable Identifier generation. |
| `schema.rs` | Validates Version 1.0 object taxonomy and canonical serialization boundaries. |
| `index.rs` | Maintains lookup indices for CAID-to-node and relation traversal queries. |
| `traversal.rs` | Implements zero-copy graph traversal and projection routines. |
| `replay.rs` | Reconstructs registry state from `SPEC-EVT-001` event records. |
| `lineage.rs` | Verifies immutable provenance chains and genesis ancestry. |
| `error.rs` | Defines fail-closed registry error variants. |
| `config.rs` | Stores compile-time allocation, schema, and traversal limits. |

---

## 5. Public Interface Primitives

The public interface for the `sovereign-registry` crate MUST compile against the following technology-neutral abstractions. These primitives define identity, classification, relation tracking, lineage validation, and replay-driven reconstruction.

```rust
/// Strongly typed content-addressable identifier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Caid(pub [u8; 32]);

/// Version 1.0 first-class object taxonomy.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ObjectClass {
    Capability = 0x01,
    Specification = 0x02,
    Policy = 0x03,
    Event = 0x04,
    Dataset = 0x05,
    Workflow = 0x06,
    VerificationArtifact = 0x07,
    EvidencePackage = 0x08,
}

/// Typed graph relation categories.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RelationType {
    DerivedFrom,
    DependsOn,
    Supersedes,
    Verifies,
    Governs,
    Produces,
    Consumes,
}

/// Immutable registry node representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryNode<'a> {
    pub id: Caid,
    pub class: ObjectClass,
    pub payload: &'a [u8],
    pub parents: &'a [Caid],
}

/// Immutable directed edge representation.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RegistryEdge {
    pub parent: Caid,
    pub child: Caid,
    pub relation: RelationType,
}

/// Operational error classifications forcing fail-closed handling.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RegistryError {
    InvalidCaid,
    InvalidSchema,
    UnknownObjectClass,
    DuplicateNode,
    MissingParent,
    CycleDetected,
    OrphanNode,
    ReplayDivergence,
    UnsupportedVersion,
}

/// Core functional contract implemented by the registry graph engine.
pub trait Registry {
    type NodeIter<'a>: Iterator<Item = &'a RegistryNode<'a>>
    where
        Self: 'a;

    type EdgeIter<'a>: Iterator<Item = &'a RegistryEdge>
    where
        Self: 'a;

    fn insert_node(&mut self, node: RegistryNode<'_>) -> Result<Caid, RegistryError>;

    fn insert_edge(&mut self, edge: RegistryEdge) -> Result<(), RegistryError>;

    fn lookup(&self, id: &Caid) -> Option<&RegistryNode<'_>>;

    fn parents(&self, id: &Caid) -> Self::NodeIter<'_>;

    fn children(&self, id: &Caid) -> Self::NodeIter<'_>;

    fn edges(&self, id: &Caid) -> Self::EdgeIter<'_>;

    fn verify_lineage(&self, id: &Caid) -> Result<(), RegistryError>;

    fn verify_acyclic(&self) -> Result<(), RegistryError>;

    fn replay_from_ledger(&mut self) -> Result<(), RegistryError>;
}
```

---

## 6. Runtime Invariants

Conforming implementations of `SPEC-REG-001` SHALL enforce the following programmatic boundaries at runtime.

### REG-INV-001: Identity Immutability

A CAID SHALL permanently identify exactly one canonical serialized object.

If two payloads produce the same CAID but differ at the canonical byte level, the registry MUST halt insertion and emit:

```rust
RegistryError::InvalidCaid
```

### REG-INV-002: Graph Acyclicity

The registry graph SHALL remain a Directed Acyclic Graph at all times.

Any insertion that introduces a cycle MUST be rejected before the mutation becomes visible to readers.

### REG-INV-003: Provenance Completeness

Every registered node, except the explicit genesis object, MUST include at least one valid parent CAID.

Parent references MUST resolve to existing registry nodes before the child node is accepted.

### REG-INV-004: Append-Only Semantic Evolution

Existing nodes and edges SHALL NOT be mutated after insertion.

Semantic updates MUST be represented as new nodes with distinct CAID values and explicit relation edges such as `Supersedes` or `DerivedFrom`.

### REG-INV-005: Deterministic Reconstruction

Replaying the authoritative event ledger from genesis SHALL reconstruct an identical registry graph, including node set, edge set, object classes, relation types, and lineage paths.

### REG-INV-006: Location Independence

Registry identity SHALL remain independent of filenames, storage offsets, memory addresses, network locations, runtime ordering of readers, or host execution environment.

### REG-INV-007: Query Non-Interference

Read-only graph traversal SHALL NOT block verification, replay, lineage validation, or other concurrent read operations.

### REG-INV-008: Schema Closure

Only the eight Version 1.0 object classes defined in this specification SHALL be accepted by the registry.

Unknown or future object classes MUST fail closed unless explicitly enabled by a versioned amendment.


---

## 7. Failure and Boundary Management Models

When a structural, cryptographic, or relational validation boundary is breached, the `sovereign-registry` crate MUST reject partial acceptance and enter a fail-closed containment path.

### REG-FAIL-001: Invalid CAID Containment

If a submitted node's declared CAID does not match the canonical hash of its deterministic serialized byte stream, the registry SHALL reject the node before insertion and emit:

```rust
RegistryError::InvalidCaid
```

### REG-FAIL-002: Schema Rejection

If a submitted object fails Version 1.0 schema validation, the registry SHALL reject the object and emit:

```rust
RegistryError::InvalidSchema
```

### REG-FAIL-003: Missing Parent Rejection

If a submitted node references a parent CAID that is not present in the current registry graph, the insertion SHALL fail and emit:

```rust
RegistryError::MissingParent
```

### REG-FAIL-004: Cycle Rejection

If a proposed node or edge insertion would introduce a directed cycle, the registry SHALL reject the operation before it becomes visible to readers and emit:

```rust
RegistryError::CycleDetected
```

### REG-FAIL-005: Replay Divergence Halt

If replaying the authoritative event ledger produces a registry state that differs from the expected deterministic reconstruction path, replay SHALL halt and emit:

```rust
RegistryError::ReplayDivergence
```

The registry SHALL NOT continue graph reconstruction beyond a divergent replay boundary.

### REG-FAIL-006: Unsupported Version Halt

If a registry event, object class, schema version, or serialization marker is not supported by the active runtime, the registry SHALL reject the operation and emit:

```rust
RegistryError::UnsupportedVersion
```

---

## 8. Validation Requirements

The `sovereign-registry` crate SHALL include conformance validation covering identity generation, schema boundaries, graph acyclicity, lineage verification, replay reconstruction, and query isolation.

| Validation ID | Target Requirement | Verification Method |
|---|---|---|
| REG-VAL-001 | REG-REQ-001 | Deterministic CAID generation test using canonical byte fixtures. |
| REG-VAL-002 | REG-REQ-002 | Object taxonomy schema validation test. |
| REG-VAL-003 | REG-REQ-003 | Cycle injection and rejection test. |
| REG-VAL-004 | REG-REQ-004 | Parent lineage verification test. |
| REG-VAL-005 | REG-REQ-005 | Location-neutral identity test. |
| REG-VAL-006 | REG-REQ-006 | Full replay reconstruction test from `SPEC-EVT-001` event records. |
| REG-VAL-007 | REG-REQ-007 | Concurrent read traversal non-interference test. |
| REG-VAL-008 | REG-REQ-008 | Append-only semantic update test. |
| REG-VAL-009 | REG-INV-005 | Byte-equivalent registry reconstruction test across independent runs. |
| REG-VAL-010 | REG-FAIL-004 | Directed cycle fault-injection test. |

Validation suites SHALL reside under:

```text
crates/sovereign-registry/tests/registry_conformance_tests.rs
```

Benchmark suites SHOULD reside under:

```text
crates/sovereign-registry/benches/traversal_benchmarks.rs
```

---

## 9. Dependency Graph

`SPEC-REG-001` depends directly on the deterministic chronological guarantees established by `SPEC-EVT-001`.

```text
[SPEC-EVT-001: Event Ledger]
        │
        ▼
[SPEC-REG-001: Registry DAG]
        │
        ▼
[SPEC-ASM-001: ASM Core]
        │
        ├──────────────┐
        ▼              ▼
[SPEC-POL-001]    [SPEC-AUD-001]
        │              │
        └──────┬───────┘
               ▼
[SPEC-DSC-001: Discovery Runtime]
```

### 9.1 Depends On

| Specification | Dependency Reason |
|---|---|
| `SPEC-EVT-001` | Provides canonical chronological event ordering and replay substrate. |

### 9.2 Required By

| Specification | Dependency Reason |
|---|---|
| `SPEC-ASM-001` | Consumes registry graph state as the semantic substrate for deterministic state reduction. |
| `SPEC-POL-001` | Consumes registry object classes and policy nodes for authorization evaluation. |
| `SPEC-AUD-001` | Consumes registry lineage, provenance, and graph reconstruction evidence. |
| `SPEC-DSC-001` | Consumes registry-enclosed tool, workflow, and evidence package identities. |

---

## 10. Future Extension Points

Future versions of this specification MAY introduce extensions through explicit versioned amendments only.

Permitted extension domains include:

- Additional object classes beyond the Version 1.0 taxonomy
- Additional typed relation categories
- Alternative governance-approved cryptographic hash profiles
- Hardware-accelerated graph traversal backends
- Persistent memory-backed graph snapshots
- Distributed registry replication protocols
- Cross-node CAID proof exchange formats

Future extensions SHALL NOT weaken the following locked properties:

- CAID determinism
- Append-only semantic evolution
- DAG acyclicity
- Lineage completeness
- Location-neutral identity
- Replay-based reconstruction
- Fail-closed schema validation

---

## Appendix A: Glossary

| Term | Definition |
|---|---|
| CAID | Content-Addressable Identifier generated from the canonical serialized byte stream of a registry object. |
| DAG | Directed Acyclic Graph; a directed graph that contains no cycles. |
| Registry Node | Immutable graph vertex representing a first-class Sovereign OS object. |
| Registry Edge | Immutable typed relation connecting a parent registry node to a child registry node. |
| Object Class | One of the eight Version 1.0 first-class registry classifications. |
| Lineage | Immutable ancestry path linking a registry object to its declared parent objects and ultimately to system genesis. |
| Genesis Object | Explicit root object from which registry lineage begins. |
| Replay Reconstruction | Process of rebuilding registry graph state by sequentially consuming authoritative ledger events. |

---

## Appendix B: Normative References

| Reference | Description |
|---|---|
| `SPEC-EVT-001` | Deterministic Append-Only Event Ledger. |
| `SPEC-ASM-001` | Deterministic State Machine Runtime. |
| `SPEC-POL-001` | Policy Engine. |
| `SPEC-AUD-001` | Audit and Replay Engine. |
| `SPEC-DSC-001` | Discovery Layer Runtime. |
| `ARCH-006` | Content-addressable architectural identity reference. |
| `ARCH-007` | Provenance and lineage architectural reference. |
| `HB-CH02` | Handbook chapter governing deterministic subsystem coordination. |
| `HB-CH06` | Handbook chapter governing registry, object taxonomy, and semantic graph logic. |

---

## Implementation Lock

This specification is approved and locked as the Phase 16 content-addressable registry contract.

Downstream crates MUST NOT weaken, bypass, or reinterpret the registry invariants defined in this document.

