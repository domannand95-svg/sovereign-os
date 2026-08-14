# SPEC-REG-001-A03: Class-Specific Payload Schemas

## 1. Purpose and Scope
This amendment establishes the common structural framework for class-specific Registry v2 payload schemas and normatively defines `CapabilityPayloadV1` (`A03-C01`) as the first non-Genesis class-specific payload schema. Payload schemas for Specification, Policy, Event, Dataset, Workflow, VerificationArtifact, and EvidencePackage remain unallocated within this amendment until separately specified.

This amendment does not define the authoritative time source used for temporal admission decisions. Ambient host time SHALL NOT constitute Registry authority.

## 2. Common Envelope V1
Every non-Genesis class-specific payload schema defined under A03 SHALL begin with Common Envelope V1 unless a later normative amendment explicitly allocates a different envelope version. `CapabilityPayloadV1` SHALL implement Common Envelope V1.

The Common Envelope V1 SHALL consist strictly of:
1. **`schema_version` (2 bytes):** An unsigned 16-bit integer in big-endian byte order (`u16 BE`), defining the structural version of the *entire* subsequent payload schema, not merely the envelope.
2. **`issuer_identity` (32 bytes):** The authority responsible for the object's issuance. This field SHALL be encoded as the exact canonical 32-byte representation of an `IdentityId` and SHALL satisfy all structural validity rules applicable to `IdentityId` under the identity subsystem.

The envelope SHALL NOT accept or evaluate alternative identity representations (e.g., raw public keys, X.509 certificates, DID strings, or `Caid` references) within the `issuer_identity` field.

## 3. A03-C01: `CapabilityPayloadV1` Canonical Encoding

### 3.1 Payload Bounds and Preflight Rejection
A `CapabilityPayloadV1` (where `schema_version == 0x0001`) has an absolute minimum length of 111 bytes and an absolute maximum length of 538 bytes.

Prior to field-level decoding, the parser MUST evaluate the total payload length. Any payload where `len < 111` or `len > 538` MUST fail closed immediately as structurally malformed. Falling within this bound permits sequential decoding to begin but does not imply structural validity.

### 3.2 Sequential Cursor Invariant
The implementation MUST decode sequentially. Each successfully decoded field MUST consume exactly its canonical byte representation and advance a single, shared payload cursor by exactly that number of bytes. The parser MUST NOT evaluate nested structures independently or allow overlapping byte consumption.

### 3.3 Canonical Byte Layout
The payload MUST be evaluated strictly in the following order:

| Cursor | Width | Field Name | Canonical Encoding Rules |
| :--- | :--- | :--- | :--- |
| `0` | `2` | `schema_version` | `u16 BE`. MUST be exactly `0x0001`. |
| `2` | `32` | `issuer_identity` | Exact 32-byte `IdentityId`. MUST satisfy identity subsystem validity. |
| `34` | `32` | `subject_identity` | Exact 32-byte `IdentityId`. MUST satisfy identity subsystem validity. |
| `66` | `2` | `operation` | `u16 BE` `OperationCodeV1`. (`0x0001` READ, `0x0002` CREATE, `0x0003` EXECUTE). |
| `68` | `variable` | `target_scope` | `TargetScopeV1` tagged union. |
| `next` | `1` | `auth_exec_marker` | `0x00` (absent) or `0x01` (present). |
| `next` | `0` or `32` | `authorized_executable` | If marker is `0x01`, exactly 32-byte `Caid`. |
| `next` | `variable` | `resource_constraints` | `ResourceConstraintsV1` struct. REQUIRED (no marker). |
| `next` | `1` | `exec_budget_marker` | `0x00` (absent) or `0x01` (present). |
| `next` | `0` or `33` | `execution_budget` | If marker is `0x01`, exactly 33-byte `ExecutionBudgetV1`. |
| `next` | `1` | `expiry_marker` | `0x00` (absent) or `0x01` (present). |
| `next` | `0` or `8` | `expiry` | If marker is `0x01`, exactly 8-byte `u64 BE` representing POSIX/Unix seconds since `1970-01-01T00:00:00Z`. |
| `next` | `32` | `governing_policy` | Exact 32-byte `Caid`. REQUIRED (no marker). |
| `EOF` | `0` | **End of Payload** | MUST be exactly EOF. No trailing bytes or padding. |

### 3.4 Nested Subcontracts

#### 3.4.1 `TargetScopeV1`
* **`0x01` ExactObject:** Followed by exactly 32 bytes (`Caid`).
* **`0x02` NamedScope:** Followed by 2 bytes `length` (`u16 BE`), then exactly `length` bytes of valid UTF-8. `length` MUST be `1 <= length <= 256`. No trimming, case-folding, or canonicalization is permitted.

#### 3.4.2 `ResourceConstraintsV1`
Must be parsed sequentially:
1. **`version` (1 byte):** MUST be exactly `0x01`.
2. **`network`:** `0x00` (DENIED) or `0x01` (ALLOWED_WITHIN_SCOPE). If `0x01`, followed by `NetworkScopeV1` (`0x01` + 32-byte `Caid`).
3. **`filesystem_read`:** `0x00` (DENIED) or `0x01` (ALLOWED_WITHIN_SCOPE). If `0x01`, followed by `FilesystemReadScopeV1` (`0x01` + `Caid` OR `0x02` + `Caid`).
4. **`filesystem_write`:** `0x00` (DENIED) or `0x01` (ALLOWED_WITHIN_SCOPE). If `0x01`, followed by `FilesystemWriteScopeV1` (`0x01` + `Caid`).

#### 3.4.3 `ExecutionBudgetV1`
Must be parsed sequentially (33 bytes total). Sentinels representing "unlimited" are prohibited. Absence of this struct equates to zero independent execution-budget authority.
1. **`version` (1 byte):** MUST be exactly `0x01`.
2. **`wall_time_ms` (8 bytes):** `u64 BE`.
3. **`memory_bytes` (8 bytes):** `u64 BE`.
4. **`network_egress_bytes` (8 bytes):** `u64 BE`.
5. **`filesystem_write_bytes` (8 bytes):** `u64 BE`.

## 4. Failure Taxonomy and Admission Gates

The Registry MUST evaluate a candidate `CapabilityPayloadV1` through six sequential validation gates. Failure at any gate MUST halt validation and reject the candidate.

### Gate 1: Structural Decoder
Verifies byte-exact compliance, preflight bounds, markers, UTF-8 validity, and truncation.

* **Failure:** `MalformedCapabilityPayload`

### Gate 2: Internal Coherence
Verifies that the structurally valid payload is internally semantically coherent. The following conditions MUST fail closed:

* `OperationCodeV1 == CREATE` combined with `TargetScopeV1 == ExactObject(Caid)`.
* `network_constraint == DENIED` combined with an `ExecutionBudgetV1` where `network_egress_bytes > 0`.
* `filesystem_write_constraint == DENIED` combined with an `ExecutionBudgetV1` where `filesystem_write_bytes > 0`.
* `authorized_executable` is present while `operation != EXECUTE`.

Absence of `authorized_executable` when `operation == EXECUTE` is structurally and semantically permissible at this gate. Policy authorization MAY impose a narrower requirement.

* **Failure:** `CapabilitySemanticViolation`

### Gate 3: Reference and Identity Resolution
Verifies all declared references through their authoritative namespaces.

* `Caid` fields SHALL resolve to admitted Registry objects where resolution is required by the field's semantics.
* `IdentityId` fields SHALL resolve and remain valid under the canonical identity subsystem.
* An `IdentityId` SHALL NOT be interpreted as, converted into, or required to correspond to a Registry-object `Caid`.
* **Failure:** `UnresolvedCapabilityReference` (for unresolved Registry references). Identity-resolution failures SHALL be represented by an explicitly mapped identity error from the identity subsystem.

### Gate 4: Deterministic Temporal Validity
If `expiry` is present, the Registry SHALL evaluate it only against a canonical `admission_context_time` supplied by the authoritative admission environment.

`expiry` and `admission_context_time` SHALL use the same temporal domain: unsigned POSIX/Unix seconds since `1970-01-01T00:00:00Z`.

A Capability is temporally valid only while:

`admission_context_time < expiry`

If:

`admission_context_time >= expiry`

the candidate MUST fail closed.

The Registry SHALL NOT derive `admission_context_time` from an ambient host wall clock, local system clock, process clock, filesystem timestamp, or other non-canonical environmental source.

This amendment defines the temporal comparison rule but does not allocate the authoritative source or wire representation of `admission_context_time`. That authority MUST be specified before temporal Capability admission is implemented.

* **Failure:** `CapabilitySemanticViolation`

### Gate 5: Issuer Authorization
Verifies the `issuer_identity` exists, is active, and is authorized to grant the requested authority.

* **Failure:** `UnauthorizedCapabilityIssuer`

### Gate 6: Policy Authorization
Verifies the `governing_policy` `Caid` resolves specifically to an admitted object of `ObjectClass::Policy`, and that this policy authorizes the exact parameters of the grant.

* **Failure:** `InvalidGoverningPolicy`
