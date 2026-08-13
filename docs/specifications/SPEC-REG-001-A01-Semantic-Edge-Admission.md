# SPEC-REG-001-A01: Semantic Edge Admission and Deterministic Query Contract

**Parent Specification:** `SPEC-REG-001`
**Parent Version:** 1.0
**Amendment ID:** `SPEC-REG-001-A01`
**Subsystem:** `sovereign-registry`
**Status:** Approved / Locked
**Classification:** Versioned Specification Amendment
**Effective Date:** 2026-08-13
**Implementation Baseline:** `c770b60` — `feat(registry): add typed semantic edge graph`

---

## 1. Purpose

This amendment extends the locked `SPEC-REG-001` Version 1.0 contract with normative admission, identity, acyclicity, and query-ordering rules for typed semantic registry edges.

This amendment does not rewrite or invalidate the historical Version 1.0 registry contract.

Its purpose is to define the compatibility boundary required for Version 2 registry semantics while preserving all historical Version 1.0 identity and lineage behavior.

---

## 2. Scope

This amendment governs:

- semantic identity of `RegistryEdge`
- typed-edge endpoint resolution
- duplicate-edge admission behavior
- typed-edge cycle rejection
- interaction between legacy provenance relationships and typed semantic edges
- coexistence of multiple relation types between identical endpoints
- deterministic typed-edge query ordering
- preservation of legacy lineage semantics
- preservation of existing node CAIDs

This amendment does not govern:

- Version 2 ledger ingestion
- replay event encoding
- Discovery runtime integration
- Capability Firewall integration
- independent edge content addressing
- mutation of historical Version 1.0 node identities

Those concerns remain outside this amendment until separately specified.

---

## 3. Semantic Edge Identity

For graph-storage and replay purposes, the semantic identity of a typed registry edge SHALL be the tuple:

`(parent_caid, child_caid, relation_type)`

An exact match across all three tuple fields identifies the same semantic edge.

`RegistryEdge` instances SHALL NOT receive independent CAIDs under this amendment.

The CAIDs of the parent and child nodes SHALL remain unchanged by insertion, lookup, or traversal of typed semantic edges.

Canonical wire encoding of `RegistryEdge` remains governed separately from semantic graph-storage identity.

---

## 4. Endpoint Resolution

Before a genuinely new typed semantic edge may be admitted, both endpoint CAIDs MUST resolve to already admitted registry nodes.

If either endpoint is unknown, insertion MUST fail closed.

Under the current `sovereign-registry` implementation surface, this condition is represented by:

```rust
RegistryError::UnresolvedReference
```

No partial edge state may become observable after endpoint-resolution failure.

---

## 5. Exact-Duplicate Idempotence

Re-insertion of an already admitted exact tuple:

`(parent_caid, child_caid, relation_type)`

MUST be idempotent.

An exact duplicate insertion MUST:

1. return success;
2. leave graph state unchanged;
3. create no duplicate stored edge;
4. preserve all node identities and lineage state.

Exact-duplicate detection SHALL occur before cycle analysis.

This ordering establishes duplicate replay or repeated admission of an already accepted semantic relation as an operational no-op rather than a new graph transition.

---

## 6. Relation Multiplicity

Multiple typed semantic edges MAY exist between the same parent and child CAIDs when their `relation_type` values differ.

For example:

```text
(A, B, DependsOn)
(A, B, Verifies)
```

represent two distinct semantic edges.

Relation type therefore participates in semantic edge identity and MUST NOT be discarded when testing edge equality or graph membership.

---

## 7. Unified DAG Admission Invariant

Legacy provenance relationships and typed semantic edges SHALL participate in one unified directed acyclic graph invariant for admission and cycle-detection purposes.

The effective directed graph used for admission SHALL include:

1. legacy parent-to-child provenance relationships derived from `RegistryNode::parents()`;
2. admitted typed semantic `RegistryEdge` relationships.

A proposed insertion MUST be rejected if the combined graph would contain a directed cycle.

This includes cycles formed entirely by typed semantic edges and cycles formed by any combination of legacy provenance relationships and typed semantic edges.

For example, if legacy provenance establishes:

```text
A -> B
```

and a proposed typed semantic edge establishes:

```text
B -> A
```

the semantic edge insertion MUST fail because the combined graph would become cyclic.

Under the current `sovereign-registry` implementation surface, cycle rejection is represented by:

```rust
RegistryError::GraphCycleDetected
```

---

## 8. Self-Loop Rejection

A proposed edge for which:

`parent_caid == child_caid`

constitutes a directed cycle of length one.

Such an insertion MUST fail closed with the graph cycle error before mutation becomes visible.

---

## 9. Legacy Lineage Preservation

Typed semantic edges SHALL NOT mutate or redefine the historical lineage semantics encoded by `RegistryNode::parents()`.

Legacy lineage resolution SHALL continue to derive ancestry exclusively from the immutable parent CAID array carried by registry nodes.

Insertion of a typed semantic edge MUST NOT:

- append to or remove from `RegistryNode::parents()`;
- modify legacy dependent relationships derived from node provenance;
- alter the observable result of legacy lineage resolution;
- cause either endpoint node to receive a new CAID.

The unified DAG invariant defined in this amendment applies to admission safety only. It does not merge semantic-edge traversal into historical Version 1.0 lineage semantics.

---

## 10. Deterministic Query Ordering

Public typed-edge query APIs MUST NOT expose nondeterministic container iteration order.

The current query surface includes:

```rust
contains_edge(&self, edge: &RegistryEdge) -> bool
outgoing_edges(&self, parent: &Caid) -> Vec<&RegistryEdge>
incoming_edges(&self, child: &Caid) -> Vec<&RegistryEdge>
```

Collections returned by `outgoing_edges` and `incoming_edges` MUST be deterministically ordered according to the canonical ordering implemented for `RegistryEdge`.

Internal use of unordered containers such as `HashSet` is permitted only when their iteration order is normalized before becoming observable through the public API.

---

## 11. Required Admission Order

Typed semantic edge insertion SHALL apply validation in the following order:

1. Resolve both endpoint nodes.
2. Detect an already admitted exact semantic edge tuple.
3. If the tuple already exists, return success without mutation.
4. For a genuinely new edge, perform unified graph cycle analysis.
5. Reject any cycle, including a self-loop.
6. Admit the edge only after all preceding checks succeed.

Implementations MUST NOT expose partially admitted graph state if any validation stage fails.

---

## 12. Validation Contract

Conforming implementations of this amendment SHALL validate at minimum:

| Validation ID | Required Behavior |
|---|---|
| REG-A01-VAL-001 | Unknown endpoint is rejected. |
| REG-A01-VAL-002 | Valid typed semantic edge is admitted and queryable. |
| REG-A01-VAL-003 | Exact duplicate insertion is idempotent and does not duplicate storage. |
| REG-A01-VAL-004 | Identical endpoints with different relation types may coexist. |
| REG-A01-VAL-005 | Two-edge semantic cycle is rejected. |
| REG-A01-VAL-006 | Self-loop is rejected. |
| REG-A01-VAL-007 | Cycle formed across legacy provenance and semantic edges is rejected. |
| REG-A01-VAL-008 | Outgoing-edge query ordering is deterministic. |
| REG-A01-VAL-009 | Incoming-edge query ordering is deterministic. |
| REG-A01-VAL-010 | Semantic-edge insertion does not alter legacy lineage resolution. |

---

## 13. Compatibility Guarantees

This amendment SHALL NOT alter the deterministic derivation of any historical Version 1.0 registry node CAID.

It SHALL NOT reinterpret existing Version 1.0 `parents[]` arrays as typed semantic edges.

It SHALL NOT assign independent CAIDs to typed semantic edges.

It SHALL NOT authorize Version 2 ledger ingestion.

It SHALL NOT authorize Discovery or Capability Firewall integration.

Those transitions require their own specification and validation gates.

---

## 14. Implementation Lock

The semantic-edge admission contract defined by this amendment is locked for the Registry Version 2 compatibility workstream represented by implementation baseline `c770b60`.

Future changes to:

- semantic edge identity;
- duplicate admission behavior;
- unified DAG semantics;
- lineage isolation;
- query determinism;
- edge content-addressing policy

MUST occur through a subsequent explicit versioned amendment.

The locked `SPEC-REG-001` Version 1.0 document remains historically authoritative for its original contract.
