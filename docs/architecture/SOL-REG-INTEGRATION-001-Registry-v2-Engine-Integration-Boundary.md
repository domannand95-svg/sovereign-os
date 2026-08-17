# SOL-REG-INTEGRATION-001 — Registry v2 Ledger Projection and Runtime Admission Boundary Audit

**Baseline:** `main@c6285a9`

**Working Branch:** `beta/governed-agent-integration-preparation`

**Status:** ACTIVE — ANALYSIS / BOUNDARY DEFINITION

**Document Authority:** NON-IMPLEMENTING ARCHITECTURAL ANALYSIS

**Implementation Authority:** NONE

**Production Code Changes Authorized:** NONE

**Related Registry Specifications:** A03, A03B, A04, A05, A05B, A06, A06B

**Isolation Constraint:** Issue #174 semantics remain unallocated and out of scope.

---

## 1. Purpose

This document defines the architectural boundary required to compose the
currently implemented and locked Registry v2 Capability V1 admission contracts
with `SovereignEngine` and the ledger projection path.

The purpose of this ticket is not to implement that composition.

The purpose is to determine, before implementation:

1. which layer owns Registry v2 wire decoding;
2. which layer owns live admission;
3. which layer owns deterministic replay projection;
4. where A03 through A06B validation occurs;
5. how authoritative admission-state references must be supplied;
6. what `SovereignEngine` may and may not conclude from an admitted capability;
7. how Legacy v1 replay compatibility remains unchanged; and
8. which questions must remain unresolved because their semantics have not been allocated.

The controlling principle is:

> Admission establishes that a governed record may enter authoritative state.
> Admission does not authorize runtime exercise of the capability represented by that record.

---

## 2. Repository Evidence at Baseline

The following observations are established directly by the repository state at
`c6285a9`.

### 2.1 `SovereignEngine` currently owns directive preflight and durable publication

`SovereignEngine::submit_directive()` currently performs the following broad sequence:

```text
DirectivePolicy evaluation
  -> ledger append engine bootstrap
  -> candidate EventRecord construction
  -> LedgerStateTransition mapping
  -> staged RegistryGraph clone
  -> RegistryLedgerSync::ingest_record()
  -> staged StateVector mutation
  -> durable ledger append
  -> publish staged StateVector
  -> publish staged RegistryGraph
  -> publish final LSN
```

The implementation therefore establishes an important failure boundary:

> Derived live state is not published until the ledger append succeeds.

Registry and state preflight occur before durable publication.

This behavior must not be weakened by Registry v2 integration.

---

### 2.2 `RegistryLedgerSync::decode_wire_record()` is version-aware

The Registry sync layer contains a wire router capable of distinguishing:

```text
Legacy v1 RegistryNode
Registry v2 VersionedRegistryNode
Registry v2 RegistryEdge
```

The router is represented by:

```rust
pub enum RegistryWireRecord {
    LegacyNode(RegistryNode),
    VersionedNode(VersionedRegistryNode),
    VersionedEdge(RegistryEdge),
}
```

and a version-aware decode boundary:

```rust
RegistryLedgerSync::decode_wire_record(bytes)
```

This establishes an existing version-aware decoding primitive.

---

### 2.3 `RegistryLedgerSync::ingest_record()` does not currently use that router

The active live/replay ingestion path handles Registry ledger events through the
Legacy v1 node decoder.

Conceptually:

```text
version-aware Registry wire decoding exists
        |
        X
        |
active ledger ingestion remains Legacy-v1-specific
```

Registry v2 wire primitives therefore exist adjacent to the authoritative ledger
projection path but are not yet composed into it.

This is an integration boundary, not by itself evidence of a defect.

---

### 2.4 Registry v2 graph primitives enforce structural invariants

`RegistryGraph::insert_versioned_node()` and `RegistryGraph::insert_edge()`
perform deterministic structural and graph-consistency validation.

Observed invariants include:

- provenance parents must resolve;
- duplicate v2 nodes fail closed;
- v2 nodes remain distinct from Legacy v1 compatibility APIs;
- typed semantic edges require admitted endpoints;
- duplicate exact edges are idempotent;
- semantic-edge queries are deterministic;
- typed edges participate in cycle detection;
- cycles across provenance and semantic edges are rejected;
- semantic edges do not rewrite Legacy v1 lineage semantics.

These primitives establish graph consistency.

They do not, by themselves, establish completion of the complete A03–A06B
Capability V1 authoritative admission contract.

---

### 2.5 Capability V1 semantic and authority primitives exist independently

The Registry exposes Capability V1 validation contracts corresponding to the
implemented admission family, including:

```rust
validate_capability_references(...)
validate_capability_identities(...)
validate_capability_temporal(...)
validate_capability_issuer(...)
validate_capability_governing_policy(...)
```

The Registry also contains authority-resolution abstractions corresponding to:

```text
IdentityStateRef
IssuerStateRef
PolicyStateRef
```

Capability V1 contains fields including:

```text
issuer_identity
subject_identity
operation
target_scope
authorized_executable
resource_constraints
execution_budget
expiry
governing_policy
```

The presence of `OperationCodeV1::Execute` or `authorized_executable` does not
constitute runtime capability exercise.

---

### 2.6 A03 through A06B are not orchestrated by `SovereignEngine`

At the current baseline, `SovereignEngine` does not compose the full A03–A06B
admission pipeline before Registry persistence.

The current `sovereign-policy` directive layer evaluates directive admission
rather than Capability V1 exact-grant authorization.

Therefore:

> Current directive policy admission is not equivalent to Registry v2 Capability V1 admission.

---

## 3. Required Architectural Separation

The integration architecture must preserve three distinct concepts:

```text
LIVE ADMISSION

candidate bytes
     |
     v
wire decoding
     |
     v
structural validation
     |
     v
authoritative Capability V1 admission
(A03 -> A06B)
     |
     v
ledger/state preflight
     |
     v
durable append
     |
     v
publish admitted projection
```

```text
HISTORICAL REPLAY

durable historical record
     |
     v
historical admission evidence/context
     |
     v
deterministic reconstruction
     |
     v
Registry projection
```

```text
RUNTIME EXERCISE

separate future boundary
not allocated by this ticket
```

These concepts must not collapse into one another.

---

## 4. Boundary A — Wire Projection and Ledger Ingestion

### 4.1 Existing Registry event boundary

Repository evidence currently assigns Registry ingestion behavior to existing
Registry-related ledger events including:

```rust
EventType::RegistryMutation
EventType::CapabilityPromotion
```

This ticket does not allocate new ledger `EventType` variants.

Any future event-envelope change requires an explicit contract.

### 4.2 Version-aware routing

A future Registry ledger projection path may conceptually require:

```rust
match RegistryLedgerSync::decode_wire_record(record.payload)? {
    RegistryWireRecord::LegacyNode(node) => { ... }
    RegistryWireRecord::VersionedNode(node) => { ... }
    RegistryWireRecord::VersionedEdge(edge) => { ... }
}
```

This is architectural analysis only.

It does not authorize implementation.

### 4.3 Legacy v1 preservation requirement

The following historical boundary must remain behaviorally stable:

```text
Legacy v1 bytes
  -> Legacy discriminator
  -> Legacy RegistryNode decoding
  -> historical RegistryNode CAID
  -> historical graph behavior
```

A future version-aware router must not reinterpret historical Legacy v1 bytes
through v2 structures.

Required invariant:

> Previously valid Legacy v1 wire bytes must continue to decode and project according to the historical Legacy v1 contract.

### 4.4 Unknown wire forms remain fail-closed

Malformed, empty, unknown, or incorrectly versioned Registry wire records must
continue to fail closed.

No decoder may silently reinterpret unknown v2 bytes as Legacy v1 or vice versa.

---

## 5. Boundary B — Admission vs Deterministic Projection

### 5.1 Required semantic distinction

The architecture must distinguish:

```text
authoritative admission decision
```

from:

```text
deterministic projection into RegistryGraph
```

These are not interchangeable operations.

### 5.2 Graph insertion does not prove complete admission

A successful:

```rust
graph.insert_versioned_node(...)
```

or:

```rust
graph.insert_edge(...)
```

establishes applicable graph-level invariants.

It does not independently prove that a Capability V1 candidate has passed all
authoritative A03–A06B admission gates.

### 5.3 Live admission

For newly proposed Capability V1 artifacts, all applicable allocated admission
requirements must succeed before durable publication.

No incomplete admission may become authoritative merely because graph insertion
succeeds.

### 5.4 Replay projection

Historical replay reconstructs previously admitted state.

Replay is not a fresh authorization event.

Replay may rerun a validation gate only where every authoritative historical
input required by that gate is recoverable or deterministically reconstructible
as the original input.

Replay must not substitute present mutable authority state for historical
admission state.

---

## 6. Boundary C — Genesis Bootstrap

Registry v2 bootstrap requires an expected Registry Genesis identity.

The graph must not self-select its authorized genesis.

Conceptually:

```text
trusted bootstrap configuration
           |
           v
expected RegistryGenesis CAID
           |
           v
Registry bootstrap configuration
           |
           v
Registry v2 replay
```

The exact owner and persistence contract for this bootstrap authority remain
outside the implementation scope of this ticket.

---

## 7. Boundary D — Capability Admission Gate Composition

The locked Capability V1 admission sequence is:

```text
Gate 1  Structural decoding
Gate 2  Internal semantic coherence
Gate 3A Governed-reference validation
Gate 3B Authoritative identity resolution
Gate 4  Temporal validation
Gate 5  Issuer operational eligibility and issuer competency
Gate 6  Governing-policy exact-grant authorization
```

Failure at a required gate prevents progression.

A later integration must preserve this ordering unless the authoritative
specification family is explicitly amended.

---

## 8. Boundary E — Pre-Persistence Failure Requirement

For live admission:

> Failure of any required admission gate must occur before durable append.

The intended transactional property is:

```text
candidate
   |
   v
complete required preflight
   |
   +---- failure ----> no durable mutation
   |
   v
durable append
   |
   v
publish staged state
```

A rejected candidate must not:

- append to the ledger;
- publish Registry state;
- publish StateVector changes; or
- advance authoritative final state.

---

## 9. Boundary F — Authoritative Admission Context

A03B through A06B demonstrate that authoritative admission depends on context
external to the raw Capability V1 candidate.

Known contextual components include:

```text
IdentityStateRef
admission_context_time
issuer-state reference
policy-state reference
```

These components are supplied by the governing admission environment.

They must not be independently selected from unrelated mutable state.

---

## 10. Same-Context Binding Requirement

The locked specifications establish that the authoritative state used by the
Capability V1 gates belongs to one governing admission context.

Conceptually:

```text
AUTHORITATIVE ADMISSION CONTEXT
===============================

IdentityStateRef
admission_context_time
issuer-state reference
policy-state reference
deterministic cross-gate binding
```

The concrete representation of this context is not yet allocated.

The values need not:

- use the same Rust type;
- use the same storage backend;
- encode one another;
- reduce to one block height;
- reduce to one state root;
- reduce to one timestamp.

They must nevertheless belong to, or be deterministically bound to, the same
authoritative admission context.

---

## 11. Boundary G — Engine Responsibility

Two broad designs remain possible.

### Option 1 — `SovereignEngine` directly orchestrates Registry admission

```text
SovereignEngine
   |
   +-- DirectivePolicy
   +-- Registry decoding
   +-- A03-A06B admission
   +-- Registry projection
   +-- StateVector staging
   +-- durable append
```

Potential benefit:

- keeps the complete pre-append decision inside one transactional owner.

Risks:

- couples general engine persistence to Registry-specific authority semantics;
- increases authority surface inside `SovereignEngine`;
- risks future confusion between admission and runtime exercise.

### Option 2 — Distinct Registry admission authority

```text
candidate
   |
   v
Registry admission authority
   |
   | verified/bound result
   v
SovereignEngine
   |
   v
durable append and projection publication
```

Potential benefits:

- separates authority evaluation from persistence;
- allows independent admission conformance testing;
- limits Registry-specific semantics inside `SovereignEngine`;
- reinforces admission != execution.

Risks:

- any result must remain inseparably bound to the exact candidate and exact authoritative context;
- stale or substituted admission evidence must fail closed.

---

## 12. Preliminary Architectural Disposition

Based on the current repository boundaries, separation between Registry
admission authority and general engine persistence appears architecturally
preferable for further specification.

This is not an implementation allocation.

A verified admission result must never be an unbound boolean such as:

```rust
true
```

nor may a generic directive result such as:

```rust
PolicyDecision::Allow
```

be treated as equivalent to completed Capability V1 admission.

---

## 13. Candidate-Binding Invariant

If a distinct admission authority exists in the future:

```text
candidate A
   -> evaluate
   -> admission result A
```

must never permit:

```text
candidate B
   +
admission result A
   ->
persistence
```

The admission result must remain deterministically bound to the exact candidate
and exact authoritative admission context.

The mechanism of that binding remains unallocated.

---

## 14. `CapabilityPromotion` Boundary

The existing event name:

```rust
EventType::CapabilityPromotion
```

must not be interpreted as runtime execution authority merely because of its
name.

This ticket does not allocate:

- runtime authority elevation;
- delegation;
- lifecycle promotion semantics;
- capability exercise;
- trust-tier changes; or
- automatic execution.

Any distinct semantics for `CapabilityPromotion` require separate specification.

---

## 15. Runtime Execution Firewall

Capability V1 includes fields such as:

```text
OperationCodeV1::Execute
authorized_executable
resource_constraints
execution_budget
```

These describe admitted capability data.

They do not themselves cause execution.

Required invariant:

```text
admitted Execute capability
        !=
runtime capability exercise
        !=
tool invocation
        !=
external effect
```

No interface derived from this ticket may invoke a:

- tool;
- subprocess;
- network request;
- filesystem mutation;
- workload executor;
- model;
- agent scheduler; or
- other external side effect

merely because a Registry Capability exists.

---

## 16. Legacy Runtime Components Outside the Current Workspace

Historical repository components include older orchestration and node-management
systems.

Examples include:

```text
registry-service
governance-wrapper
network-service
active-memory
event-log
```

These are not the current authoritative Capability V1 admission path.

The locked A05B specification additionally establishes that historical
`registry-service::OperationalStatus` values are not equivalent to Gate 5
issuer eligibility.

For example:

```text
OperationalStatus::Active
        !=
Gate 5 Eligible
```

Legacy node capability strings likewise do not establish modern Capability V1
issuer authority.

Any reuse requires a separate architecture and conformance review.

---

## 17. Audit Evidence Boundary

Governed audit evidence may record failures such as policy denial.

However:

> Evidence of a failed admission or execution attempt is not itself authority.

A failed-attempt record does not:

- grant retry authority;
- override policy;
- authorize a capability;
- mutate issuer competency;
- change policy semantics; or
- create runtime exercise permission.

---

## 18. Explicit Non-Goals

This ticket does not specify or authorize:

1. runtime capability exercise;
2. runtime tool invocation;
3. external side effects;
4. requester/caller identity binding;
5. requester-to-subject authorization;
6. delegation;
7. authority propagation;
8. confused-deputy handling;
9. collusion handling;
10. capability revocation;
11. capability supersession;
12. trust-tier mutation;
13. present-state freshness;
14. "latest state" semantics;
15. present-state reinterpretation of historical admission;
16. composite authority orchestration;
17. agent scheduling;
18. model routing;
19. tool selection;
20. credential issuance;
21. credential exercise;
22. runtime resource enforcement;
23. runtime execution-budget enforcement; or
24. any behavior parked under Issue #174.

---

## 19. Conceptual Integration Interface Shape

The following interfaces are illustrative only.

They are not authorized production code.

### 19.1 Candidate decoding boundary

```rust
fn decode_registry_candidate(
    event_type: EventType,
    payload: &[u8],
) -> Result<Option<RegistryWireRecord>, RegistryError>;
```

Conceptually:

```text
non-Registry event
    -> None

valid Legacy Registry record
    -> LegacyNode

valid Registry v2 node
    -> VersionedNode

valid Registry v2 edge
    -> VersionedEdge

malformed or unknown Registry payload
    -> fail closed
```

### 19.2 Admission authority boundary

A future abstraction may conceptually resemble:

```rust
trait RegistryAdmissionAuthority {
    type Error;
    type Context;

    fn evaluate(
        &self,
        graph: &RegistryGraph,
        candidate: &RegistryWireRecord,
        context: &Self::Context,
    ) -> Result<RegistryAdmissionDecision, Self::Error>;
}
```

This exists only to illustrate separation:

```text
decode
  !=
admit
  !=
persist
  !=
execute
```

Concrete Rust types remain unallocated.

### 19.3 Projection boundary

Conceptually:

```rust
fn project_admitted_registry_record(
    graph: &mut RegistryGraph,
    record: &RegistryWireRecord,
) -> Result<(), RegistryError>;
```

Such a function would represent deterministic projection only.

It would not independently grant authority.

---

## 20. Proposed Live Event Flow

```text
candidate proposal
        |
        v
directive boundary
        |
        v
directive admission
        |
        v
Registry wire decoding
        |
        v
A03-A06B authoritative admission
        |
        +---- DENY ----> no durable mutation
        |
       ADMIT
        |
        v
stage Registry projection
        |
        v
stage StateVector transition
        |
        v
durable append
        |
        +---- ambiguous durability ---> engine restart required
        |
        v
publish staged StateVector
        |
        v
publish staged RegistryGraph
        |
        v
return durable receipt
```

No runtime capability exercise occurs in this flow.

---

## 21. Historical Replay Flow

The replay flow cannot be reduced to raw v2 wire decoding alone.

Conceptually:

```text
trusted boot configuration
        |
        v
restore durable ledger/state
        |
        v
recover historical admission evidence/context
        |
        v
version-aware Registry decoding
        |
        v
reconstruct historically authoritative admission result
        |
        v
deterministic Registry projection
        |
        v
continue in LSN order
        |
        v
verify reconstructed tail
        |
        v
publish engine
```

Replay must fail closed if the required historical authoritative context cannot
be restored according to the future persistence contract.

---

## 22. Required Acceptance Properties for Future Implementation

### 22.1 Legacy compatibility

Historical Legacy v1 records must retain:

```text
identical decoding semantics
identical CAID semantics
identical compatibility behavior
identical deterministic replay behavior
```

### 22.2 Registry v2 node projection

An admitted v2 node must:

```text
survive durable append
reconstruct after restart
retain identical CAID
retain identical object class
retain identical provenance
```

### 22.3 Registry v2 edge projection

An admitted typed edge must:

```text
survive durable append
reconstruct after restart
retain its exact semantic tuple
preserve Legacy v1 lineage behavior
```

### 22.4 Admission rejection

Failure of any required admission gate must result in:

```text
no ledger append
no RegistryGraph publication
no StateVector publication
no authoritative tail advancement
```

### 22.5 Admission-context mismatch

If required historical/contextual inputs cannot be proven to belong to the
required authoritative admission context:

```text
candidate admission must fail closed
```

No freshness or "latest" semantics may be invented to resolve the mismatch.

### 22.6 Replay independence from present state

Historical replay must not substitute:

```text
current wall-clock time
current identity state
current issuer state
current policy state
current Registry head
latest policy
runtime agent state
tool availability
```

for the original admission inputs.

### 22.7 Execute capability non-effect

Admitting:

```rust
OperationCodeV1::Execute
```

must not itself:

```text
spawn a process
invoke a tool
open a network connection
write a filesystem resource
dispatch an agent
change caller authority
```

### 22.8 Ambiguous durability

Existing ambiguous-commit safeguards must remain fail-closed.

Registry v2 integration must not weaken restart requirements after ambiguous
durability.

---

## 23. Open Architectural Questions

### Q1. Registry ledger event envelope

Which existing Registry ledger event types may carry:

```text
LegacyNode
VersionedNode
VersionedEdge
```

and should the permitted payload classes differ by event type?

This remains unresolved.

### Q2. Admission authority ownership

Should complete Capability V1 admission composition reside:

```text
inside sovereign-registry
inside sovereign-engine
or behind a distinct admission component
```

?

This ticket does not allocate the final implementation location.

### Q3. Authoritative admission-context representation

What canonical persistent mechanism identifies the common authoritative
admission context containing or binding:

```text
IdentityStateRef
admission_context_time
issuer-state reference
policy-state reference
```

?

This remains unallocated.

### Q4. Historical policy semantics

How does replay guarantee that the Policy object is interpreted under the same
deterministic governing-policy semantics as the original admission?

Potential representations are intentionally not selected here.

### Q5. Durable admission evidence

What durable artifact proves that this exact candidate completed the A03–A06B
pipeline under this exact historical context?

Possible design families include:

```text
persist every required historical input
persist an immutable/reconstructible context reference
persist a candidate-bound admission result
use a hybrid
```

No option is selected by this ticket.

---

## 24. Current Disposition

At `main@c6285a9`:

### CONFIRMED

- Registry v2 wire decoding primitives exist.
- Registry v2 node and typed-edge graph primitives exist.
- A03–A06B validation/resolution primitives exist.
- active ledger ingestion remains Legacy-v1-specific.
- `SovereignEngine` does not compose A03–A06B.
- current directive policy admission is not Capability V1 Gate 6 authorization.
- graph insertion does not prove complete Capability V1 admission.
- runtime capability exercise is not established by Registry admission.
- legacy runtime/node-management components are not authoritative Gate 5 state.
- historical replay contracts explicitly prohibit substitution of current mutable state.

### PROPOSED

- preserve Legacy v1 behavior exactly;
- keep version-aware Registry decoding separate from authoritative admission;
- separate live admission from deterministic projection;
- preserve the existing preflight-before-durable-publication engine boundary;
- require exact candidate/context binding for any future verified admission result;
- keep complete Capability V1 admission separate from runtime execution.

### UNRESOLVED

- exact event-type/payload-class matrix;
- final admission authority ownership;
- canonical authoritative admission-context representation;
- persistence representation of identity-state references;
- persistence representation of issuer-state references;
- persistence representation of policy-state references;
- canonical binding between context components;
- representation of historical policy-evaluation semantics;
- durable admission-evidence representation.

### NOT ALLOCATED

- runtime capability exercise;
- delegation;
- caller/requester binding;
- revocation;
- supersession-as-authority;
- current-state freshness;
- "latest" authority semantics;
- policy inheritance;
- policy fallback;
- composite authority orchestration;
- Issue #174 semantics.

---

## 25. Historical Admission Reconstruction Constraint

The locked Capability V1 admission specifications establish that a durable
Registry v2 Capability record cannot be fully replayed from
`CapabilityPayloadV1` bytes alone.

Historical admission depends on authoritative context external to the candidate,
including:

1. the Gate 3B `IdentityStateRef`;
2. the Gate 4 `admission_context_time`;
3. the Gate 5 authoritative issuer-state reference;
4. the Gate 6 authoritative policy-state reference; and
5. the deterministic binding establishing that all required state inputs belong
   to the same original authoritative admission context.

Gate 6 additionally requires deterministic governing-policy evaluation
semantics. Historical replay therefore requires not only preservation of
authoritative state, but preservation or deterministic reconstruction of the
policy interpretation under which the original authorization decision was made.

Replay MUST NOT substitute:

- current identity state;
- current issuer state;
- current policy state;
- current Registry head;
- current wall-clock time;
- latest Policy objects;
- root or fallback Policies;
- current policy-engine configuration;
- newly introduced evaluator semantics; or
- any other mutable ambient authority source.

The specifications deliberately do not allocate persistent wire encodings for
the admission context, the identity-state reference, issuer-state reference, or
policy-state reference. They likewise do not allocate a canonical Policy V1
payload schema or policy language.

Therefore this integration ticket MUST record these historical dependencies as
prerequisites and MUST NOT invent their persistent representations.

### 25.1 Historical Authority-State Boundary

Historical reconstruction must preserve or deterministically reconstruct the
authoritative state inputs against which the original decision was made.

At minimum:

```text
IdentityStateRef
admission_context_time
issuer-state reference
policy-state reference
authoritative admission-context binding
```

Historical replay is therefore not equivalent to re-running the candidate
against the system's present state.

### 25.2 Historical Evaluation-Semantics Boundary

Gate 6 additionally depends on deterministic governing-policy semantics.

The same historical Policy object evaluated under semantically different
software could otherwise produce a different authorization result.

Conceptually:

```text
same candidate
+
same historical Policy object
+
different evaluation semantics
=
potentially different result
```

Such behavior would violate deterministic replay.

Therefore future persistence design must establish how the historically
applicable governing-policy semantics remain deterministically reproducible.

This ticket does not allocate whether that mechanism is represented by:

```text
schema version
evaluator version
semantic identifier
content identity
specification identity
code identity
or another mechanism
```

That allocation belongs to the follow-on persistence specification.

### 25.3 Historical Admission Is Not Present-State Validity

The following questions are distinct:

```text
Was this Capability valid and authorized under its original admission context?
```

and:

```text
Would this Capability be valid or exercisable under current state?
```

The first is required for deterministic historical reconstruction.

The second belongs to future lifecycle/runtime authority semantics and is not
allocated by this ticket.

A later revocation, policy change, issuer-state transition, identity change, or
passage of time must not rewrite the original historical admission result during
replay.

### 25.4 Projection Is Not Historical Proof of Admission

A version-aware ledger router such as:

```text
ledger record
    |
    v
decode_wire_record
    |
    +-- LegacyNode
    +-- VersionedNode
    +-- VersionedEdge
```

is sufficient only to identify a structural Registry wire form.

It does not, by itself, prove that a Capability V1 node historically completed
the authoritative A03–A06B admission pipeline.

Therefore:

```text
wire decoding
        !=
historical admission proof
```

and:

```text
RegistryGraph projection
        !=
Capability V1 authorization
```

---

## 26. Updated Integration-Gap Matrix

| Contract | Required Historical Evidence | Live Admission Dependency | Replay Dependency | Integration Gap |
| :--- | :--- | :--- | :--- | :--- |
| **A03** | Exact canonical payload + historical inputs required by Gates 3B–6. | Sequential fail-closed Gates 1–6. | Authority gates require historical context. | Payload alone does not encode complete admission context. |
| **A03B** | Historical `IdentityStateRef`. | Deterministic resolution against supplied state ref. | Same IDs + same state ref must yield identical result. | Persistent `IdentityStateRef` encoding explicitly unallocated. |
| **A04** | Historical context time + association with `IdentityStateRef`. | Time supplied by admission environment. | Replay original time, not current time. | Persistent admission-context encoding unallocated. |
| **A05** | Historical authoritative issuer state + context association. | Prove `Eligible` + general Capability V1 authority. | Later state changes must not alter historical result. | No persistent issuer-state representation allocated. |
| **A05B** | Exact historical issuer-state reference. | Deterministic issuer-state resolution. | Same referenced state must reproduce identical outcome. | Reference encoding and maximum size unallocated. |
| **A06** | Historical policy state, exact candidate, exact Policy object, policy semantics, admission-context binding. | Exact-grant authorization only. | Original policy state, context, and evaluator semantics required. | Policy schema and evaluator persistence explicitly unallocated. |
| **A06B** | Exact policy-state ref and deterministic binding among candidate, object, semantics, and context. | Resolution + evaluation; only `Authorized` succeeds. | Current/latest policy substitution explicitly forbidden. | No policy-state encoding or canonical evaluator allocated. |

---

## 27. Follow-On Specification Boundary

### Ticket

`SOL-REG-INTEGRATION-002`

### Title

**Authoritative Admission Context Persistence and Replay Contract**

### Type

**Specification Allocation**

### Purpose

Define the canonical persistent mechanism required to:

1. durably identify the historical authoritative admission context;
2. bind an exact `CapabilityPayloadV1` candidate to that context;
3. reconstruct or verify the exact historical state inputs required by A03–A06B;
4. preserve deterministic governing-policy evaluation semantics across replay;
5. define fail-closed behavior when required historical context cannot be restored.

### Must Allocate

- what historical admission context must be durably identifiable;
- how the exact candidate is deterministically bound to that context;
- how historical `IdentityStateRef` dependencies are represented or recovered;
- how historical `admission_context_time` is represented or recovered;
- how historical issuer-state dependencies are represented or recovered;
- how historical policy-state dependencies are represented or recovered;
- how cross-gate same-context binding is proven;
- how historically applicable policy-evaluation semantics remain deterministic;
- what the canonical durable representation is;
- how replay fails when required context cannot be restored.

### Must Not Presuppose

The specification must not presuppose that the binding is necessarily:

- cryptographic;
- a state root;
- a block hash;
- a ledger height;
- a receipt;
- a snapshot;
- a new Registry object;
- a new event type; or
- any other particular representation

until that representation is explicitly allocated.

### Explicit Non-Goals

`SOL-REG-INTEGRATION-002` must not allocate:

- runtime capability exercise;
- runtime tool execution;
- delegation;
- authority propagation;
- confused-deputy semantics;
- revocation;
- supersession;
- state freshness;
- "latest-state" semantics;
- requester identity;
- caller identity;
- requester-to-subject binding;
- subject operational validity;
- runtime executable integrity;
- runtime resource enforcement;
- execution-budget enforcement;
- policy inheritance;
- policy fallback;
- policy delegation;
- composite policy authorization;
- agent orchestration;
- model routing;
- any semantics isolated under Issue #174.

---

## 28. Governing Invariant

The integration phase governed by this analysis must preserve the following boundary:

> Registry v2 may determine whether a governed Capability record was admissible
> into authoritative state under an explicitly bound historical admission
> context. It does not determine whether any actor may exercise that Capability
> at runtime.

The corresponding engineering invariants remain:

> **Admission is not execution.**

> **Historical admission is not present-state validity.**

> **Evidence is not authority.**

> **A stored capability is not a runtime permission check.**

> **Probabilistic intelligence; deterministic authority.**
