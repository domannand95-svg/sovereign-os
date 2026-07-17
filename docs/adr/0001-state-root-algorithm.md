---
Document: State Root Algorithm and Domain Separator
ID: ADR-0001
Version: 1.0
Status: Accepted
Owner: domannand95-svg
Reviewers:
- Core Reviewers
Last Updated: 2026-07-17
Evidence: Gate 8 implementation contract
Depends On:
- GOV-CORE-001
Supersedes: None
---

# ADR-0001: State Root Algorithm and Domain Separator

## Status

Accepted

## Context

`sovereign-ledger` stores snapshot envelopes containing a `state_root_hash: [u8; 32]` field, but currently does not compute or verify this hash. The core snapshot module (`sovereign-core-asm`) provides canonical `encode` and `decode` operations for `StateVector`, but does not prescribe a hashing algorithm.

## Decision

The state root shall be computed as:

```text
state_root = BLAKE3("SOVEREIGN_STATE_V1" || core_asm::snapshot::encode(state_vector))
```

Where:

- `"SOVEREIGN_STATE_V1"` is the domain separator as exactly 18 ASCII bytes, with no null terminator.
- `||` denotes concatenation.
- BLAKE3 is the cryptographic hash function already present in the `sovereign-ledger` dependency set.

## Alternatives Considered

1. **SHA-256** - Cryptographic, but not already present in the dependency tree and slower for this use case.
2. **CRC32C** - Already used for envelope integrity, but not cryptographic and therefore insufficient as the authoritative state root.
3. **No domain separator** - Simpler, but lacks versioning and cross-domain safety.

## Rationale

- BLAKE3 is already a dependency of `sovereign-ledger`.
- The domain separator protects against cross-domain hash reuse and establishes an explicit version boundary.
- State encoding is already canonical through `sovereign_core_asm::snapshot::encode`.
- The ledger remains authoritative for the hash algorithm while `sovereign-core-asm` remains dependency-neutral.

## Consequences

- The state-root definition is tied to both the canonical snapshot encoding and the `SOVEREIGN_STATE_V1` domain separator.
- Any incompatible encoding or state-root change requires a new domain separator, such as `SOVEREIGN_STATE_V2`.
- A fixed test vector must verify the exact byte concatenation and resulting digest.

## Invariants

- `state_root_hash` in a snapshot header MUST equal `BLAKE3("SOVEREIGN_STATE_V1" || encode(restored_state))`.
- The snapshot filename LSN MUST equal `associated_lsn` in its header.
- Snapshot creation MUST compute the state root from the exact encoded payload written into the envelope.
- Snapshot restoration MUST reject a candidate whose decoded payload does not match the stored state root.

## Verification

- Fixed test vector: a known `StateVector` produces a known expected digest.
- Integration test: create a snapshot, read it back, and verify the root matches the payload.
- Negative test: a filename/header LSN mismatch rejects the candidate.
- Negative test: a state-root mismatch rejects the candidate and permits recovery through the approved fallback path.
