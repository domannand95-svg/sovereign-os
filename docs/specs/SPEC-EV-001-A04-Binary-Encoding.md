---
Document: A04 Governed Research Record Binary Encoding
ID: SPEC-EV-001-ENC-001
Version: 1.0
Status: Approved Baseline
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-08-11
Evidence:
  - Owner approval recorded 2026-08-11
Depends On:
  - SPEC-EV-001
  - SPEC-ID-001
Supersedes: None
---

# A04 Governed Research Record Binary Encoding

## 1. Architectural Intent

The A04 encoding rules establish a permanent, deterministic compatibility
contract for governed research records, including objectives, claims, sources,
methods, reviews, disputes, and decisions. Enforcing strict canonical encoding
ensures predictable memory usage, protection against hostile
resource-exhaustion vectors, consistent cross-machine behavior, and immutable
cryptographic identities.

## 2. Hard Limits and Resource Boundaries

Oversized or malformed values are strictly rejected prior to admission. The
system will never perform silent truncation.

- **Maximum Text Field:** 64 KiB (65,536 bytes) of valid UTF-8.
- **Maximum List Length:** 64 items per list, including success criteria,
  constraints, and evidence references.
- **Maximum Immediate Parents:** 64 direct ancestry or provenance records.
- **Maximum Overall Payload:** 1 MiB (1,048,576 bytes) total combined payload
  per record.

Exceptionally large research packages, datasets, or full papers must be stored
as separately identified external artifacts and referenced by digest. This
keeps individual governance records focused, attributable, safely reviewable,
and within the decoding budget.

## 3. Temporal Determinism

Operational timestamps, including retrieval, expiry, review, and decision
times, use the following immutable v1 representation:

- **Format:** Unsigned Unix seconds in UTC.
- **Precision:** Second-level.
- **Historical Dates:** Publication dates before 1 January 1970 UTC are not
  execution timestamps and remain separately governed descriptive metadata.
- **Floating Point:** Prohibited for timestamps.

This prevents locale ambiguity, floating-point rounding, and platform-specific
time encodings from changing record identity.

## 4. Canonical Encoding Rules

- Unsigned integers use big-endian byte order.
- Ordered lists retain their explicit governed order.
- Set-like lists are sorted exclusively by identifier bytes and reject
  duplicates.
- Text must be valid UTF-8, no longer than 65,536 bytes, and free of NUL
  characters. Required text must not be empty.
- Optional values use one explicit marker byte: `0x00` absent or `0x01`
  present. Any other marker is invalid.
- Unknown enum discriminators are rejected.
- Padding and trailing bytes are rejected.
- Declared counts and lengths are checked before allocation.
- Oversized values are rejected and never truncated.

## 5. Schema Evolution and Versioning

Schema Version 1 is immutable. Expanded field sizes, increased list capacities,
sub-second timestamps, or structural field changes require Schema Version 2.

Version 1 records remain indefinitely readable under Version 1 rules. A future
implementation must not reinterpret version 1 bytes or silently change their
identity.

## 6. Authority Boundary

Canonical encoding proves deterministic structure and identity only. It does
not admit a record to the ledger, verify a claim, approve a disposition, grant
a capability, authorize execution, or promote an artifact.

## 7. Initial Objective Fixed Vectors

The first typed Objective slice freezes these v1 vectors:

- Minimal field fixture `statement="S"`, `scope="P"`, one success criterion
  `"C"`, one constraint `"K"`, and `expires_at=42`:
  `00000001530000000150000100000001430001000000014b01000000000000002a`
- Governed Objective record fixture in the Rust test suite:
  `07f6521b9f6105551b1c356ebd92629d1f532298c180d2615b006bbff2264ade`

Changing either vector requires a new schema version rather than reinterpretation
of version 1.
