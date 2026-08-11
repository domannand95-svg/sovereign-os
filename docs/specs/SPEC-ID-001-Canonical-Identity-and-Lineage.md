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
