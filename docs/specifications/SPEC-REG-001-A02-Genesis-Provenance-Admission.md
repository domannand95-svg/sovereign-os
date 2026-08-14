# SPEC-REG-001-A02: Genesis and Provenance Admission Contract

**Parent Specification:** `SPEC-REG-001`
**Parent Version:** 1.0
**Amendment ID:** `SPEC-REG-001-A02`
**Subsystem:** `sovereign-registry`
**Status:** Approved / Locked
**Classification:** Versioned Specification Amendment
**Effective Date:** 2026-08-14
**Implementation Baseline:** Specification precedes implementation

---

## 1. Purpose

This amendment extends the locked `SPEC-REG-001` contract with deterministic Registry v2 genesis and provenance admission semantics.

Its purpose is to:

- define the zero-parent exception required by `REG-INV-003`;
- define deterministic identification and authorization of a native Registry v2 Genesis Object;
- require provenance completeness for ordinary Registry v2 nodes;
- preserve historical Registry v1 identity and admission semantics;
- support mixed v1/v2 provenance without historical reinterpretation; and
- establish exact fail-closed admission behavior before implementation begins.

This amendment is additive.

It does not rewrite or invalidate the historical Registry v1 compatibility contract.

---

## 2. Scope

This amendment governs:

- Registry v2 provenance completeness;
- Registry v2 zero-parent admission;
- definition of the Registry v2 Genesis Object;
- deterministic genesis payload encoding;
- external provisioning of the authorized Genesis CAID;
- Registry v2 genesis singularity;
- mixed v1/v2 provenance references;
- deterministic genesis and provenance admission ordering;
- failure semantics for genesis and provenance admission; and
- minimum conformance validation for these boundaries.

This amendment does not govern:

- historical Registry v1 CAID derivation;
- historical Registry v1 decoding;
- Version 2 ledger ingestion;
- replay event encoding;
- Registry mutation event discriminators;
- Discovery runtime integration;
- Capability Firewall integration;
- domain-specific `RelationType × ObjectClass` authorization;
- policy evaluation;
- execution authorization; or
- migration or reinterpretation of historical Registry nodes.

---

## 3. Historical Registry v1 Compatibility Boundary

The introduction of Registry v2 genesis and provenance admission semantics SHALL NOT alter the interpretation, identity, admission history, or deterministic reconstruction of historical Registry v1 objects.

A02 establishes an additive Version 2 admission contract. It does not rewrite the historical Version 1 registry protocol.

### 3.1 Historical Semantic Preservation

Historical Registry v1 node bytes SHALL continue to be interpreted exclusively according to their historical v1 semantics.

A02 MUST NOT reinterpret a historical v1 discriminator, payload, parent array, or other encoded field according to Registry v2 semantics.

### 3.2 Historical CAID Preservation

The deterministic CAID derivation of every historical Registry v1 node SHALL remain unchanged.

A02 MUST NOT:

- alter the byte sequence used for historical v1 CAID derivation;
- introduce a new domain separator into historical v1 identity derivation;
- re-hash an existing historical v1 node under Registry v2 rules; or
- assign a replacement Registry v2 CAID to an existing historical v1 object.

Historical replay over identical v1 records MUST continue to reconstruct identical historical v1 CAIDs.

### 3.3 Legacy Zero-Parent Preservation

A historical Registry v1 node that was validly admitted under the historical v1 admission rules SHALL remain a valid historical artifact even when its immutable `parents[]` array is empty.

A02 SHALL NOT retroactively invalidate such a node solely because it does not satisfy the Registry v2 genesis or provenance requirements introduced by this amendment.

This protection applies only to historical v1 semantics.

It SHALL NOT establish zero-parent admission as a general rule for Registry v2 nodes.

### 3.4 Prohibition of Semantic Reassignment

A02 SHALL NOT:

- reinterpret historical v1 `parents[]` arrays as Registry v2 semantic edges;
- alter the historical meaning of v1 provenance relationships;
- reclassify historical v1 nodes;
- infer or assign a Registry v2 `ObjectClass` to a historical v1 node; or
- mutate historical v1 nodes so that they conform to Registry v2 schemas.

Historical compatibility SHALL be achieved through preservation rather than migration or reinterpretation.

### 3.5 Exemption from Registry v2 Genesis Rules

Historical Registry v1 records SHALL be exempt from the Registry v2 genesis-identification and non-genesis provenance-admission rules defined by this amendment.

A02 SHALL NOT require a historical v1 node to be retrospectively classified as either:

- the Registry v2 Genesis Object; or
- a Registry v2 non-genesis object.

The Version 2 genesis contract applies only to Registry v2 admission.

### 3.6 Mixed v1/v2 Graph Compatibility

A Registry graph containing both historical v1 nodes and Registry v2 nodes SHALL preserve the historical identity and interpretation of all admitted v1 nodes.

Registry v2 genesis and provenance requirements SHALL apply when admitting Registry v2 nodes and MUST NOT modify the historical admission semantics of existing v1 nodes.

Where a Registry v2 node is permitted to reference an admitted historical v1 node, that reference SHALL use the historical v1 CAID unchanged.

Nothing in this section authorizes semantic reclassification of the referenced v1 node.

### 3.7 Replay and Ledger Scope Boundary

A02 defines Registry admission semantics only.

It does not define or modify:

- Version 2 ledger event encoding;
- Registry mutation event discriminators;
- ledger ingestion routing;
- replay transport formats; or
- canonical event-log sequencing.

Those concerns remain governed by their respective specifications and future integration gates.

A future ledger/replay integration MUST preserve the historical v1 compatibility guarantees established by this section.

---

## 4. Registry v2 Provenance Admission Model

This section establishes the provenance requirements governing Registry v2 node admission.

### 4.1 Universal Provenance Requirement

Except for an authorized Registry v2 Genesis Object admitted under Section 5, every Registry v2 node MUST declare one or more provenance parents.

A non-genesis Registry v2 node submitted with an empty `parents[]` array MUST fail closed.

### 4.2 Parent Resolution

Every provenance parent declared by a candidate Registry v2 node MUST resolve to an already admitted Registry object before the candidate becomes observable as admitted state.

A parent MAY resolve to:

- an admitted historical Registry v1 node; or
- an admitted Registry v2 node.

If any declared parent does not resolve, admission MUST fail closed with `RegistryError::UnresolvedReference`.

No partial node admission may become observable following parent-resolution failure.

Malformed CAID encodings SHALL fail at the applicable decoding or schema-validation boundary before graph admission.

### 4.3 Provenance DAG Integrity

Provenance parent relationships SHALL participate in the unified Registry DAG invariant.

A candidate Registry v2 node MUST be rejected if its proposed provenance relationships would introduce a directed cycle into the combined Registry graph.

Cycle rejection remains governed by the Registry graph-integrity contract and `SPEC-REG-001-A01`.

### 4.4 Mixed-Lineage Resolution

A Registry v2 node MAY identify an admitted historical Registry v1 node as a provenance parent.

Such a reference satisfies the non-genesis provenance requirement without reclassifying, re-encoding, re-hashing, or otherwise modifying the historical v1 parent.

### 4.5 Zero-Parent Rejection

A Registry v2 node with an empty `parents[]` array MUST be rejected unless the candidate satisfies every applicable structural and authorization requirement for Registry v2 genesis admission defined in Section 5.

Structural resemblance to a Genesis Object SHALL NOT by itself constitute authorization to establish a Registry root.

---

## 5. Explicit Registry v2 Genesis Object

The Registry v2 Genesis Object provides the zero-parent provenance root for a Registry initialized directly under Version 2 semantics.

The Genesis Object is a bootstrap mechanism.

It SHALL NOT be used to reinterpret, replace, or supersede historical Registry v1 roots.

### 5.1 Applicability

A Registry v2 Genesis Object is required only when establishing a new Registry graph directly under Registry v2 semantics.

A Registry reconstructed from valid historical Registry v1 state MAY continue into Registry v2 operation without introducing a Registry v2 Genesis Object.

In such a mixed graph, newly admitted Registry v2 non-genesis nodes MUST establish provenance through one or more already admitted v1 or v2 parent CAIDs.

A02 SHALL NOT require insertion of a new zero-parent Registry v2 root into an existing historical Registry graph.

### 5.2 Registry v2 Taxonomy Extension

A02 explicitly extends the Registry v2 `ObjectClass` taxonomy with:

    RegistryGenesis = 0x09

Under Registry v2 semantics, the valid object-class taxonomy therefore consists of nine classes.

For Registry v2 only, this amendment extends the constraints established by `REG-REQ-002` and `REG-INV-008` accordingly.

The historical Registry v1 taxonomy SHALL remain unchanged.

A02 MUST NOT introduce `0x09` into the historical Registry v1 `RegistryNodeType` decoding boundary or reinterpret any historical discriminator.

### 5.3 Exact Canonical Genesis Payload Encoding

The `RegistryGenesis` payload SHALL use a byte-exact canonical representation.

The payload SHALL contain fields in exactly the following order:

    environment_id_length : u16 BE
    environment_id        : [u8; environment_id_length]
    protocol_version      : u16 BE
    root_policy_present   : u8
    root_policy_caid      : conditional [u8; 32]

#### Environment Identifier Length

`environment_id_length` SHALL be encoded as exactly two bytes representing an unsigned 16-bit integer in big-endian byte order.

#### Environment Identifier

`environment_id` SHALL contain exactly `environment_id_length` bytes.

The byte sequence MUST be valid UTF-8.

Environment Identifier identity SHALL be bytewise.

The Registry SHALL NOT perform:

- Unicode normalization;
- case folding;
- whitespace trimming;
- locale-dependent transformation; or
- any other textual canonicalization.

Distinct valid UTF-8 byte sequences SHALL constitute distinct Environment Identifiers.

An environment identifier of zero bytes is structurally valid unless restricted by a future amendment.

#### Protocol Version

`protocol_version` SHALL be encoded as exactly two bytes in big-endian order.

For this amendment, the only valid value is:

    0x0002

Any other protocol-version value MUST fail closed.

#### Root Policy Presence Marker

`root_policy_present` SHALL contain exactly one byte.

Valid values are:

    0x00 — no root policy CAID follows
    0x01 — exactly one 32-byte root policy CAID follows

Any other marker value MUST fail closed.

When the marker is `0x00`, no root-policy bytes may follow.

When the marker is `0x01`, exactly 32 bytes MUST follow and SHALL be interpreted as the root policy CAID.

#### Payload Closure

Any:

- truncation;
- missing required field;
- invalid UTF-8;
- invalid protocol version;
- invalid root-policy marker;
- missing required root-policy bytes;
- unexpected root-policy bytes;
- trailing byte;
- padding; or
- additional field

MUST cause the candidate to fail closed as a malformed Genesis payload.

### 5.4 Expected-Genesis-CAID Provisioning Boundary

Structural validity is distinct from admission authority.

A node possessing:

- `ObjectClass::RegistryGenesis`;
- an empty `parents[]` array; and
- a valid Genesis payload

is only a structurally well-formed Genesis candidate.

The expected Genesis CAID SHALL originate from governance-approved bootstrap configuration external to the candidate Registry object.

A candidate node SHALL NOT:

- define;
- modify;
- infer;
- replace; or
- self-authorize

the expected Genesis CAID.

A Registry instance establishing a native Registry v2 graph SHALL be initialized with the governance-approved expected Genesis CAID before Genesis admission is attempted.

If Genesis admission is attempted when no governance-approved expected Genesis CAID has been provisioned for the Registry instance, admission MUST fail closed with `RegistryError::UnauthorizedGenesis`.

A candidate `RegistryGenesis` node MUST satisfy:

    candidate.caid == expected_genesis_caid

If the identities differ, admission MUST fail closed with `RegistryError::UnauthorizedGenesis`.

The Registry SHALL NOT establish genesis according to first-arrival or insertion-order semantics.

### 5.5 Genesis Singularity

A native Registry v2 graph SHALL contain exactly one authorized Registry v2 Genesis Object.

The authorized Genesis Object may be admitted only when no Registry v1 or Registry v2 nodes have previously been admitted to the graph.

Re-submission of the already established authorized Genesis Object MUST fail with `RegistryError::GenesisAlreadyEstablished`.

Submission of an otherwise authorized Genesis Object to a graph that contains admitted nodes but does not contain the established authorized Genesis Object MUST fail with `RegistryError::GenesisNotPermittedInExistingGraph`.

A Registry v2 Genesis Object SHALL NOT be appended to an existing historical or otherwise populated topology.

---

## 6. Deterministic Admission Ordering

Registry v2 node admission SHALL apply A02 validation in deterministic order.

### 6.1 RegistryGenesis Candidate

For a candidate whose class is `ObjectClass::RegistryGenesis`, admission SHALL evaluate:

1. Genesis payload validity.
2. Zero-parent requirement.
3. Expected Genesis CAID authorization.
4. Whether the authorized Genesis Object is already established.
5. Whether any other Registry v1 or Registry v2 node has already been admitted.
6. Final Genesis admission.

The resulting behavior SHALL be:

    malformed Genesis payload
        → MalformedGenesisPayload

    non-empty parents[]
        → InvalidGenesisProvenance

    candidate CAID != expected Genesis CAID
        → UnauthorizedGenesis

    authorized Genesis already established
        → GenesisAlreadyEstablished

    graph otherwise contains admitted nodes
        → GenesisNotPermittedInExistingGraph

    otherwise
        → ADMIT

### 6.2 Ordinary Registry v2 Candidate

For every Registry v2 node whose class is not `RegistryGenesis`, admission SHALL evaluate:

1. Non-empty provenance requirement.
2. Resolution of every declared parent.
3. Unified DAG / cycle safety.
4. Existing duplicate and structural admission boundaries.
5. Final node admission.

The resulting provenance behavior SHALL include:

    parents[] empty
        → MissingProvenance

    declared parent unresolved
        → UnresolvedReference

    proposed relationship creates directed cycle
        → GraphCycleDetected

    otherwise
        → continue ordinary v2 admission

No failed admission path may expose partially admitted graph state.

---

## 7. Mixed v1/v2 Provenance Rules

Historical Registry v1 nodes MAY serve as provenance parents for Registry v2 non-genesis nodes.

Such references SHALL:

- resolve using the existing historical v1 CAID;
- preserve the historical v1 object's bytes;
- preserve its discriminator semantics;
- preserve its historical provenance;
- preserve its CAID derivation; and
- confer no Registry v2 `ObjectClass` upon it.

A historical zero-parent Registry v1 node MAY therefore anchor later Registry v2 provenance without being reclassified as the Registry v2 Genesis Object.

---

## 8. Genesis Identity Preservation

The Registry v2 Genesis Object is an immutable content-addressed Registry v2 node.

Its CAID SHALL continue to derive from the existing Registry v2 canonical node encoding and identity rules, including:

- Registry v2 domain separation;
- Registry v2 encoding version;
- `ObjectClass::RegistryGenesis`;
- its empty parent array; and
- the canonical Genesis payload defined by this amendment.

A02 SHALL NOT introduce an independent or alternative CAID algorithm specifically for Genesis Objects.

Changing any canonical Genesis content SHALL produce a different CAID and therefore a different Genesis candidate.

Such a candidate SHALL NOT become authoritative unless its resulting CAID is independently provisioned through the external governance boundary defined in Section 5.4.

---

## 9. Compatibility Guarantees

A02 SHALL preserve:

- historical Registry v1 decoding;
- historical Registry v1 CAID derivation;
- historical v1 zero-parent validity;
- historical `parents[]` semantics;
- Registry v2 CAID domain separation;
- existing mixed v1/v2 parent resolution;
- unified DAG integrity; and
- typed semantic-edge behavior established by `SPEC-REG-001-A01`.

A02 SHALL NOT silently upgrade historical Registry v1 objects into Registry v2 objects.

---

## 10. Explicit Exclusions

A02 does not authorize implementation changes to:

- `RegistryLedgerSync::ingest_record()`;
- event-log discriminators;
- Version 2 ledger mutation encoding;
- replay routing;
- Discovery binary contracts;
- Discovery runtime behavior;
- Capability Firewall integration;
- policy authorization semantics;
- domain-specific relation-permission matrices; or
- historical Registry v1 identity derivation.

Those concerns remain subject to their own specification and implementation gates.

---

## 11. Failure Semantics

A02 defines the following dedicated fail-closed Registry error categories.

### `RegistryError::MissingProvenance`

A non-genesis Registry v2 node was submitted with an empty `parents[]` array.

### `RegistryError::InvalidGenesisProvenance`

A `RegistryGenesis` candidate declared one or more provenance parents.

A Registry v2 Genesis Object MUST have an empty `parents[]` array.

### `RegistryError::UnauthorizedGenesis`

A structurally valid `RegistryGenesis` candidate produced a CAID that does not match the governance-provisioned expected Genesis CAID.

### `RegistryError::GenesisAlreadyEstablished`

The authorized Registry v2 Genesis Object has already been admitted and an attempt was made to establish it again.

### `RegistryError::GenesisNotPermittedInExistingGraph`

An otherwise authorized Registry v2 Genesis candidate was submitted to a graph containing admitted Registry nodes but no established authorized Registry v2 Genesis Object.

### `RegistryError::MalformedGenesisPayload`

The candidate Genesis payload violates the canonical payload schema defined by Section 5.3.

Examples include:

- truncation;
- invalid UTF-8;
- invalid protocol version;
- invalid root-policy presence marker;
- incorrect root-policy byte count; and
- trailing bytes.

Existing errors remain applicable where their existing boundaries are unchanged, including:

- `RegistryError::UnresolvedReference`
- `RegistryError::GraphCycleDetected`
- `RegistryError::DuplicateEntity`
- `RegistryError::SchemaViolation`

A02 does not require historical Registry v1 errors to be replaced with the new A02-specific errors.

---

## 12. Validation Contract

A conforming implementation of A02 SHALL establish failing tests before production implementation where practical and SHALL demonstrate the following behaviors before A02 implementation is considered complete.

### REG-A02-VAL-001 — Missing Provenance

A structurally valid ordinary Registry v2 node with an empty `parents[]` array MUST fail with `RegistryError::MissingProvenance`.

### REG-A02-VAL-002 — Invalid Genesis Provenance

A structurally valid `RegistryGenesis` candidate containing one or more parents MUST fail with `RegistryError::InvalidGenesisProvenance`.

### REG-A02-VAL-003 — Unauthorized Genesis

A structurally valid zero-parent `RegistryGenesis` candidate whose CAID does not equal the externally provisioned expected Genesis CAID MUST fail with `RegistryError::UnauthorizedGenesis`.

### REG-A02-VAL-004 — Historical v1 Preservation

A historical Registry v1 node with an empty parent array that is valid under historical v1 admission semantics MUST remain admissible and MUST NOT be subjected to Registry v2 Genesis validation.

### REG-A02-VAL-005 — Genesis Already Established

After successful admission of the authorized Registry v2 Genesis Object, re-submission of that Genesis Object MUST fail with `RegistryError::GenesisAlreadyEstablished`.

### REG-A02-VAL-006 — Genesis Not Permitted in Existing Graph

Submission of the otherwise authorized Registry v2 Genesis Object to a populated Registry graph that does not contain the established authorized Genesis Object MUST fail with `RegistryError::GenesisNotPermittedInExistingGraph`.

The populated graph MAY contain:

- historical Registry v1 nodes;
- Registry v2 nodes; or
- both.

### REG-A02-VAL-007 — Malformed Genesis Payload

Malformed Genesis payload tests SHALL independently cover at minimum:

- invalid root-policy marker `0x02`;
- trailing byte;
- invalid UTF-8 environment identifier;
- unsupported protocol version such as `0x0003`; and
- truncated payload.

Each malformed candidate MUST fail with `RegistryError::MalformedGenesisPayload`.

### REG-A02-VAL-008 — Successful Native v2 Bootstrap

The exact `RegistryGenesis` candidate whose:

- class is `RegistryGenesis`;
- parent array is empty;
- payload satisfies Section 5.3;
- derived CAID equals the governance-provisioned expected Genesis CAID; and
- target Registry graph contains no admitted v1 or v2 nodes

MUST be admitted successfully.

---

## 13. Future Amendment Boundary

Future changes to any of the following SHALL require an explicit versioned specification amendment:

- `RegistryGenesis` discriminator allocation;
- Genesis canonical payload layout;
- Genesis protocol version;
- Genesis authorization mechanism;
- expected-Genesis-CAID provisioning semantics;
- zero-parent exception rules;
- native-v2 graph singularity;
- provenance completeness requirements; or
- historical v1 compatibility guarantees established by A02.

Future amendments MUST NOT silently weaken:

- content-derived identity;
- provenance integrity;
- deterministic admission;
- fail-closed bootstrap authorization;
- historical v1 replay compatibility; or
- separation between structural validity and authorization.

---

## 14. Implementation Lock

`SPEC-REG-001-A02` establishes the locked Registry v2 genesis and provenance admission contract.

Implementation SHALL conform to this document rather than deriving missing semantics from existing code behavior.

The first implementation slice SHALL remain bounded to:

- Registry v2 taxonomy extension;
- Genesis payload encoding and validation;
- externally provisioned Genesis authorization;
- Registry v2 provenance completeness;
- deterministic genesis singularity enforcement;
- dedicated A02 failure categories; and
- conformance validation.

Ledger ingestion, replay integration, Discovery, Capability Firewall behavior, policy authorization, and historical Registry v1 identity semantics remain outside this implementation gate.