---
Document: Canonical Identity and Lineage Contract
ID: SPEC-ID-001
Version: 1.0
Status: Implemented
Owner: domannand95-svg
Reviewers:
- Core Reviewers
Last Updated: 2026-08-11
Evidence:
- Commit 2caa151
- 35 sovereign-registry tests in the v0.1 closure baseline
Depends On:
- ADR-0002
Supersedes: None
---

# SPEC-ID-001: Canonical Identity and Lineage

## Implementation Status

The version-one identity and lineage types, canonical encodings, fixed vectors,
and malformed-input rejection tests are implemented in the authoritative
`sovereign-registry` workspace crate. Runtime authority beyond these identity
contracts remains outside this specification.

## Scope

This specification defines the first A03 runtime slice: deterministic identity
records and explicit lineage records for agents, model versions, tools,
datasets, policies, evaluators, and institutions.

It does not define research artifacts, grants, trust assessments, promotion,
incident decisions, or agent execution. Those remain A04-A07 work.

## Identity kinds

The version-one kind tags are fixed:

| Tag | Kind |
|---:|---|
| `0x01` | Agent |
| `0x02` | Model version |
| `0x03` | Tool |
| `0x04` | Dataset |
| `0x05` | Policy |
| `0x06` | Evaluator |
| `0x07` | Institution |

Unknown tags fail closed. Changing a tag or its meaning requires a new encoding
version.

## Canonical identity encoding

An identity descriptor is an opaque, non-empty canonical byte sequence supplied
by the schema governing that identity kind. A03 limits descriptors to 65,536
bytes. Later kind-specific schemas must define their internal fields without
changing this envelope.

Version-one encoding is:

```text
u16 identity_encoding_version, big endian
u8  identity_kind
u32 descriptor_length, big endian
[descriptor_length] descriptor
```

The content-derived identifier is:

```text
BLAKE3("SOVEREIGN_IDENTITY_V1" || canonical_identity_encoding)
```

The domain separator contains exactly the displayed ASCII bytes and no null
terminator.

Decoding rejects:

- unsupported versions;
- unknown kind tags;
- empty or oversized descriptors;
- length overflow;
- truncation; and
- trailing bytes.

## Canonical lineage encoding

Identity and lineage are separate. An identity states what a particular
versioned subject is. A lineage record states which earlier identities directly
contributed to or preceded it.

Parent order is significant and must be assigned by the kind-specific schema.
For example, a future model schema may distinguish a base-model parent from a
training-dataset parent. Implementations must not silently sort parents.

Version-one encoding is:

```text
u16 lineage_encoding_version, big endian
[32] subject_identity_id
u16 direct_parent_count, big endian
direct_parent_count * [32] parent_identity_id
```

The lineage content root is:

```text
BLAKE3("SOVEREIGN_LINEAGE_V1" || canonical_lineage_encoding)
```

Version one permits at most 64 direct parents. A root identity may have zero
parents. Lineage validation rejects:

- a zero subject;
- zero-valued parent identifiers;
- duplicate parents;
- the subject as its own parent;
- too many parents;
- unsupported versions;
- truncation; and
- trailing bytes.

Graph-wide cycle and reference-existence checks remain the responsibility of
the registry graph that admits these records.

## Stability vectors

The authoritative tests fix these outputs:

```text
IdentityKind::ModelVersion + "model:sovereign-small:1"
  -> 5c7db193dade08b572b255609c5f129f40e176acfc2741b4621c064813d0e111

lineage(subject = identity("model:v2"),
        parents = [identity("model:v1")])
  -> a83f042f90d9d74fe8dcf41f963a7c0abaf39d30e28300fcf65e4d9273102eee
```

Linux and Windows verification must produce identical vectors.

## Security and governance properties

- A display name, provider name, account, process, or prompt is not an identity.
- A different model version receives a different descriptor and identity.
- A local model receives no automatic trust or authority from local ownership.
- Identity proves canonical content equality, not honesty, competence,
  independence, permission, or promotion.
- Lineage is explicit evidence for later policy; it does not itself grant
  authority.
- Alias and common-control analysis required by ADR-0002 is later policy work
  and cannot be inferred from distinct identifiers alone.

## Verification

A03 requires:

- canonical identity and lineage round trips;
- fixed content-root vectors;
- kind, descriptor, and parent-order separation;
- malformed version, tag, length, truncation, and trailing-byte cases;
- empty, zero, duplicate, self-parent, and parent-limit cases;
- strict linting;
- focused registry tests;
- the authoritative workspace test suite; and
- Linux and Windows continuous integration before merge.

## Changelog

- **v1.0 (2026-07-31):** Proposed the A03 canonical identity and lineage
  contract.

---

## Identity Encoding Version 2 Extension

Identity Encoding Version 2 extends the canonical identity encoding while
preserving all Version 1 identity encodings and derived identifiers
byte-for-byte.

### Version Routing

The `identity_encoding_version` field determines the permitted identity-kind
namespace.

- `0x0001` permits only the Version 1 identity kinds `0x01` through `0x07`.
- `0x0002` permits identity kinds `0x01` through `0x08`.
- `Environment` is assigned identity-kind tag `0x08`.
- `Environment` MUST NOT be encoded using identity encoding version `0x0001`.
- A decoder encountering identity kind `0x08` with identity encoding version
  `0x0001` MUST reject the encoding.
- Unknown identity encoding versions MUST fail closed.
- Unknown identity-kind tags for the selected encoding version MUST fail
  closed.

Existing identities using kinds `0x01` through `0x07` MUST continue to use
their existing Version 1 canonical encoding unless a future specification
explicitly requires Version 2 semantics. Implementations MUST NOT silently
upgrade an existing Version 1 identity to Version 2.

Version 2 retains the existing canonical envelope structure:

```text
u16 identity_encoding_version, big endian
u8  identity_kind
u32 descriptor_length, big endian
[descriptor_length] descriptor
```

The existing identity derivation remains unchanged:

```text
BLAKE3("SOVEREIGN_IDENTITY_V1" || canonical_identity_encoding)
```

The identity encoding version is contained within
`canonical_identity_encoding`; therefore Version 1 and Version 2 encodings
remain cryptographically distinct without changing the established identity
domain separator.

### Environment Identity Kind

`Environment` is identity-kind tag `0x08`.

An Environment identity represents a canonical identity for a version-specific
execution or experimental environment.

Environment identity is a commitment to a canonical environment definition.
It does not, by itself, assert reproducibility, availability, safety,
authorization, successful execution, or permission to use the identified
environment.

Derivation of an Environment identity does not establish an associated
registry node type or grant registry admission.

### Environment Descriptor Version 1

The Environment Version 1 descriptor is exactly 35 bytes:

```text
u8      schema_version
u8      definition_namespace
u8      digest_algorithm
[32]    definition_digest
```

The fields are defined as follows:

| Offset | Length | Field | Requirement |
|---:|---:|---|---|
| `0x00` | 1 | `schema_version` | MUST equal `0x01`. |
| `0x01` | 1 | `definition_namespace` | Governed canonical-definition namespace discriminator. |
| `0x02` | 1 | `digest_algorithm` | `0x01` = SHA-256; `0x02` = BLAKE3. |
| `0x03` | 32 | `definition_digest` | Digest of the canonical environment-definition bytes defined by the selected namespace. |

A decoder MUST reject:

- descriptor schema versions other than `0x01`;
- unallocated `definition_namespace` values;
- digest algorithms other than `0x01` and `0x02`;
- descriptors shorter or longer than exactly 35 bytes;
- truncated descriptors; and
- trailing descriptor bytes.

### Governed Definition Namespaces

A `definition_namespace` is valid only when its value has been formally
allocated by a normative specification that defines the exact canonical byte
sequence constituting the `definition_digest` preimage.

There is no generic, implementation-defined, vendor-defined, or custom escape
namespace.

An implementation MUST NOT construct an Environment identity for an
unallocated production namespace.

Namespace value `0xAA` is reserved exclusively for normative test fixtures in
this specification and MUST NOT be used as a production environment-definition
namespace.

Architecture, operating-system, ABI, accelerator, runtime, packaging, image,
manifest, or other environment properties are not universal fields of the
Environment descriptor. When such properties are semantically required, the
governing definition namespace MUST incorporate them into its canonical
definition contract.

### Digest Algorithms

Environment Descriptor Version 1 allocates:

| Tag | Algorithm |
|---:|---|
| `0x01` | SHA-256 |
| `0x02` | BLAKE3 |

The selected algorithm is applied to the exact canonical definition bytes
specified by `definition_namespace`.

The Environment identity layer does not canonicalize manifests, images,
configuration files, JSON, CBOR, text, package definitions, or other external
representations. Canonicalization belongs to the normative specification
governing the selected definition namespace.

### Fixed Vector ENV-V1-001

`ENV-V1-001` uses the specification-reserved test namespace `0xAA` and a
32-byte all-zero definition digest.

Inputs:

```text
identity_encoding_version = 0x0002
identity_kind             = 0x08
descriptor_length         = 0x00000023

schema_version            = 0x01
definition_namespace      = 0xAA
digest_algorithm          = 0x02
definition_digest         = 32 bytes of 0x00
```

The exact 35-byte descriptor is:

```text
01 AA 02
00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00
```

The exact 42-byte canonical identity encoding is:

```text
00 02
08
00 00 00 23
01 AA 02
00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00
```

The derived IdentityId is:

```text
8824d2f6ca5015a4d311756938257a00af1b70fe4f639241d28db6e241eb3d67
```

This value is derived using:

```text
BLAKE3("SOVEREIGN_IDENTITY_V1" || canonical_identity_encoding)
```

against the exact 42-byte canonical identity encoding above.

### Required Rejection Vectors

Implementations MUST reject at least the following malformed encodings:

1. Environment kind `0x08` encoded with identity encoding version `0x0001`.
2. Environment descriptor schema version other than `0x01`.
3. An unallocated `definition_namespace` other than a namespace explicitly
   permitted for normative testing.
4. Digest algorithm other than `0x01` or `0x02`.
5. Descriptor length less than 35 bytes.
6. Descriptor length greater than 35 bytes.
7. Truncated Environment descriptors.
8. Trailing bytes after the declared identity encoding.
9. Unknown identity kinds for the selected identity encoding version.
10. Unknown identity encoding versions.

### Compatibility Invariants

The Version 2 extension MUST NOT alter:

- the canonical bytes of any existing Version 1 identity;
- the derived identifier of any existing Version 1 identity;
- the meaning or tag assignment of Version 1 identity kinds `0x01` through
  `0x07`;
- the existing identity domain separator;
- the Version 1 lineage encoding or lineage domain separator; or
- registry-node semantics.

Environment identity introduces identity semantics only. Authorization,
capability, execution, evidence admission, registry admission, and
reproducibility remain governed by their respective contracts.