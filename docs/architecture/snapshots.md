---
Document: Snapshot Architecture Specification
ID: ARCH-SNAP-001
Version: 1.1
Status: Implemented Specification
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-07-31
Evidence:
  PRs:
    - PR-070
    - PR-071
    - PR-072
    - PR-073
    - PR-074
    - PR-075
Depends On:
  - GOV-CORE-001
  - ARCH-INDEX-001
  - ARCH-REG-001
  - ARCH-LOG-001
Supersedes: None
---

# Snapshot Architecture

This document describes the snapshot architecture for the Sovereign OS / GUSA repository.

## 1. Purpose

The snapshot system provides compact recovery state so registry startup can avoid replaying the full event history when a valid snapshot is available.

## 2. Production Boundary

Snapshots are part of the active production substrate. They accelerate recovery but do not replace the event log as the authoritative history of state transitions.

## 3. Authoritative implementation

The active implementation is `sovereign-ledger::snapshot`. Snapshot files use
the canonical name `<16-lowercase-hex-lsn>.snap` and contain:

1. the associated LSN as an eight-byte big-endian integer;
2. the 32-byte normative state-root hash;
3. the payload length as a four-byte big-endian integer;
4. the encoded state-vector payload; and
5. a four-byte CRC32C covering the header and payload.

Discovery accepts only canonical filenames, requires the embedded LSN to match
the filename, validates the exact envelope length and checksum, and returns
valid candidates newest-first. State decoding and root validation occur before
a candidate can become live recovery state.

## 4. Responsibilities

- Store compact recovery state.
- Track snapshot format versioning.
- Validate snapshot integrity before use.
- Support checksum-based corruption detection.
- Fall back to event replay when validation fails.

## 5. Atomic publication

Snapshot publication is fail-closed and no-clobber:

1. create a unique pending file in the snapshot directory;
2. write the complete envelope and synchronize the pending file;
3. publish the synchronized inode under its canonical name using an atomic hard
   link, which fails if that snapshot LSN already exists;
4. remove the pending link; and
5. synchronize directory metadata where the platform exposes that boundary.

An interrupted write therefore exposes either no canonical snapshot or one
complete canonical snapshot. A duplicate writer cannot replace an existing
snapshot. If publication succeeds but directory durability cannot be proven,
the operation returns `CommitAmbiguous`, removes the canonical name, and
requires callers to treat the attempt as failed closed.

Windows does not expose portable directory-handle synchronization through the
Rust standard library. On Windows, synchronizing the complete pending file
before atomic publication is the strongest portable commit boundary used by
the implementation.

Stale files ending in `.pending` are not candidates and cannot affect recovery.

## 6. Non-Responsibilities

The snapshot system does not define governance policy, validate research claims, or replace event-sourcing traceability.

## 7. Recovery Rule

Snapshot recovery is allowed only when the snapshot format and integrity checks pass. Invalid, incompatible, or corrupted snapshots must fall back to event-log replay.

## 8. Integration Points

- **Sovereign Engine:** Coordinates boot restoration before exposing live state.
- **Sovereign Ledger:** Owns snapshot envelopes, discovery, validation, and
  replay fallback.
- **Sovereign Core ASM:** Owns deterministic state-vector encoding and decoding.

The preserved `registry-service` prototype has a separate JSON snapshot format.
It is outside the authoritative workspace and must not be treated as this
specification's implementation.

## 9. Safety Rules

- Snapshot loading must fail safely.
- Snapshot corruption must not panic the runtime.
- Format versions must be explicit.
- Recovery must preserve deterministic state reconstruction.
- Snapshot acceleration must not weaken auditability.
- Canonical snapshots are immutable once published.
- No caller may infer success from the presence of a pending file.

## 10. Future Work

Future work includes an explicit snapshot-envelope format version and a
versioned migration framework. Those capabilities are not implemented yet.

## Changelog

- **v1.1 (2026-07-31):** Documented the authoritative binary envelope,
  validation pipeline, atomic no-clobber publication, and platform durability
  boundary.
- **v1.0 (2026-07-07):** Initialized snapshot architecture specification under PR #76.
