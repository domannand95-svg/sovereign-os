# SPEC-REG-001-A04

**Title:** Deterministic Admission Context Time (Gate 4)

**Document Status:** Approved / Locked

**Domain:** SOL Registry / Admission Pipeline

## 1. Scope

This specification defines the Gate 4 contract for evaluating the temporal validity of `CapabilityPayloadV1` artifacts.

It defines the authoritative treatment of `admission_context_time`, its deterministic replay requirements, its relationship to the authoritative admission context used by earlier gates, and the exact evaluation semantics for the optional Capability V1 `expiry` field.

Gate 4 SHALL NOT obtain time from ambient process or machine state.

## 2. Admission Context Time

The admission pipeline SHALL receive `admission_context_time` explicitly from the governing admission environment.

Gate 4 SHALL NOT independently query or derive time from:

- the operating-system wall clock;
- `SystemTime` or equivalent runtime clock APIs;
- hardware real-time clocks;
- external time servers;
- an implicitly selected "current" ledger head;
- any other ambient mutable state.

`admission_context_time` SHALL be represented in the same temporal domain as Capability V1 `expiry`: an unsigned POSIX-seconds value.

The admission pipeline MUST therefore compare two values expressed using the same epoch and unit.

## 3. Authoritative Admission Context

`admission_context_time` MUST belong to an explicitly identified authoritative admission context.

The admission environment is responsible for supplying the temporal value used for the admission decision.

Gate 4 SHALL consume that supplied value and SHALL NOT independently choose, advance, normalize, or replace it.

The mechanism by which the governing environment establishes authoritative admission context is outside the scope of this specification.

## 4. Relationship to Gate 3B Identity State

The `IdentityStateRef` consumed by Gate 3B and the `admission_context_time` consumed by Gate 4 MUST belong to the same authoritative admission context.

An admission evaluation SHALL NOT resolve identities against one authoritative state while evaluating temporal validity against an unrelated admission context.

Replay and audit MUST restore the same authoritative context relationship used by the original evaluation.

This specification does NOT require:

- `IdentityStateRef` to contain or encode a timestamp;
- `admission_context_time` to be derivable from `IdentityStateRef`;
- a particular ledger position, block timestamp, state-root format, or snapshot representation.

The admission environment is responsible for establishing the deterministic association between the two values.

## 5. Replay Determinism

For a fixed `CapabilityPayloadV1` and authoritative admission context, Gate 4 MUST produce the same result on every execution.

A later replay SHALL use the recorded or reconstructed `admission_context_time` belonging to the original authoritative admission context.

The actual wall-clock time at which replay or audit occurs SHALL NOT affect the result.

Subsequent passage of time SHALL NOT retroactively change the outcome of a historical Gate 4 evaluation.

## 6. Gate 4 Algorithm

Upon receiving a `CapabilityPayloadV1` that has successfully passed the preceding required admission gates and an authoritative `admission_context_time`, Gate 4 MUST evaluate temporal validity as follows:

1. Read the optional `expiry` value from the decoded Capability V1 representation.
2. If `expiry` is absent, Gate 4 SHALL succeed because the payload declares no expiry constraint.
3. If `expiry` is present, evaluate:

   `admission_context_time < expiry`

4. If the inequality is true, Gate 4 SHALL succeed.
5. If `admission_context_time == expiry`, Gate 4 MUST fail closed.
6. If `admission_context_time > expiry`, Gate 4 MUST fail closed.

Therefore the validity interval is strictly bounded above by `expiry`; the expiry instant itself is not valid.

## 7. Failure Mapping

A Capability V1 artifact that fails the Gate 4 temporal comparison MUST return:

`RegistryError::CapabilitySemanticViolation`

This preserves the failure mapping allocated by SPEC-REG-001-A03.

A missing or unavailable authoritative admission context SHALL NOT be silently substituted with ambient wall-clock time or another state.

The admission pipeline MUST fail closed when its required authoritative context cannot be supplied.

This specification does not allocate a new Registry error variant for admission-context unavailability.

## 8. Deterministic Arithmetic

Gate 4 requires only comparison of two `u64` POSIX-seconds values.

It SHALL NOT:

- perform wall-clock arithmetic;
- calculate elapsed runtime duration;
- apply timezone conversion;
- apply locale-dependent interpretation;
- infer clock skew;
- apply grace periods;
- alter `expiry`;
- substitute sentinel values for absence.

No temporal sentinel semantics are allocated by this specification.

## 9. Authorization Boundary

Temporal validity of the Capability and authorization of its issuer remain separate admission concerns.

A Capability passing Gate 4 does not establish that its issuer is active or authorized to grant the requested authority.

Likewise, Gate 4 SHALL NOT infer subject activity, revocation, eligibility, or authorization state.

## 10. Non-Goals

This specification explicitly excludes:

- defining a consensus mechanism or clock-synchronization protocol;
- defining the storage architecture that records authoritative admission context;
- defining a ledger, block, snapshot, or state-root representation;
- defining persistent wire encoding for admission context;
- defining `not_before`, issuance-time, or temporal lower-bound semantics not present in Capability V1;
- defining identity activity, revocation, or authorization semantics;
- deriving `IdentityStateRef` from time or time from `IdentityStateRef`.
