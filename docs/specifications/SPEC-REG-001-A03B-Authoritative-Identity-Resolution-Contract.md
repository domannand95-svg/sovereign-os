# SPEC-REG-001-A03B

**Title:** Authoritative Identity Resolution Contract (Gate 3B)

**Document Status:** REVIEW-READY / PROVISIONAL (Pending Merge)

**Domain:** SOL Registry / Admission Pipeline

## 1. Scope

This specification defines the Gate 3B interface contract for resolving authoritative identities during the admission of `CapabilityPayloadV1` artifacts. It establishes the minimal resolution guarantees required to prove identity existence and structural validity prior to temporal or authorization evaluations.

## 2. Namespace Separation

The authoritative namespace for identity resolution is the canonical SOL identity subsystem. `IdentityId` is the native, 32-byte lookup key for this subsystem. A Registry-object `Caid` SHALL NOT be used as an alias for, converted into, or required to correspond to an `IdentityId`. Registry object identity and canonical identity-subsystem identity remain strictly separated namespaces.

## 3. IdentityStateRef Contract

Gate 3B SHALL consume an `IdentityStateRef` supplied by the admission environment. The `IdentityStateRef` MUST be a storage-neutral, deterministic reference to a specific authoritative state (e.g., an opaque identifier, state root, or canonical byte value). It SHALL NOT assume a specific ledger architecture, block height, or wall-clock timestamp.

Two `IdentityStateRef` values that are bytewise identical under their canonical representation MUST designate the same authoritative identity state.

The canonical representation and maximum encoded size of `IdentityStateRef` are not allocated by this specification and MUST be defined before persistent wire encoding is introduced.

## 4. Resolver Interface Semantics

The identity resolver MUST expose a deterministic resolution interface accepting an `IdentityId` and an `IdentityStateRef`.

For a fixed authoritative identity state, identical `(IdentityId, IdentityStateRef)` inputs MUST produce the identical resolution outcome.

The resolver SHALL NOT independently select ambient mutable state, determine a "current" ledger head, or access wall-clock time.

The interface MUST return either a `ResolvedIdentity` or an explicit resolution error.

## 5. ResolvedIdentity Minimum Guarantees

A successful resolution yielding a `ResolvedIdentity` provides the following minimal guarantees:

* The queried `IdentityId` exists within the authoritative state referenced by the `IdentityStateRef`.
* The corresponding canonical identity record is valid under the structural and identifier-derivation rules of the canonical identity subsystem.
* The canonical identity record deterministically derives the exact `IdentityId` queried.

`ResolvedIdentity` SHALL NOT contain operational state, activity status, trust state, revocation state, or authorization flags used for later admission decisions unless a subsequent specification explicitly extends the Gate 3B resolution result.

## 6. Gate 3B Algorithm

Upon receiving a structurally decoded and internally coherent `CapabilityPayloadV1` and a unified `IdentityStateRef`, Gate 3B MUST execute the following sequence:

1. Extract the `issuer_identity`.
2. Extract the `subject_identity`.
3. Resolve the `issuer_identity` against the provided `IdentityStateRef`.
4. Resolve the `subject_identity` against the exact same `IdentityStateRef`.
5. If both resolutions succeed, advance the admission pipeline. If either fails, the pipeline MUST immediately halt and fail closed.

## 7. Failure Mapping

Gate 3B MUST fail closed if resolution cannot produce an unambiguous result. It SHALL expose the following explicit identity-subsystem failures mapped into the standard `RegistryError` enum:

* `IdentityNotFound`: The `IdentityId` does not exist in the referenced authoritative state.
* `IdentityStateUnavailable`: The identity subsystem or the referenced state cannot be reached or queried.

Existing structural identity errors (where authoritative identity bytes fail canonical decoding) SHALL be reused. Gate 3B SHOULD NOT introduce new generic payload errors that duplicate existing canonical identity faults.

## 8. Replay Determinism

Identity resolution MUST be fully deterministic and replayable. Providing the same `IdentityId` and `IdentityStateRef` MUST always yield the identical resolution outcome. Subsequent key rotations, revocations, or state changes MUST NOT retroactively alter the resolution result of a historical `IdentityStateRef`.

## 9. Authorization and Operational-State Exclusion

Gate 3B SHALL NOT determine whether the `issuer_identity` is active, revoked, suspended, trusted, or authorized to issue the requested Capability. Those issuer-authority evaluations are reserved for Gate 5.

Gate 3B SHALL likewise make no authorization or operational-status determination concerning the `subject_identity`.

Any future requirement to evaluate subject activity, revocation, eligibility, or authorization MUST be defined by an explicit admission contract and SHALL NOT be inferred by Gate 3B.

**Identity existence ≠ identity activity ≠ capability authority.**

## 10. Non-Goals

This specification explicitly excludes:

* Defining the underlying storage, database, or ledger architecture for the authoritative identity subsystem.
* Requiring full historical lineage traversal or graph resolution at admission time.
* Defining the exact temporal relationship between the `IdentityStateRef` and the payload's `admission_context_time` (reserved for subsequent specifications).
