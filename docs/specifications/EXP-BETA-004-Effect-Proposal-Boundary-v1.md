# EXP-BETA-004: Effect Proposal Boundary v1

## 1. Purpose
Freeze the epistemic boundary between:
* Model intent $\neq$ Authority claim
* Authority claim $\neq$ Capability grant
* Capability grant $\neq$ Capability exercise

## 2. Problem Statement
The current Sovereign OS agent output (`AGENT_OUTPUT-v1`) can express additional context needs, but no canonical object exists for model-originated effect proposals. If models emit raw capabilities directly, the semantic distinction between a proposal and an authorized capability is blurred. Capability authority must remain strictly host-controlled.

## 3. Definition: Effect
For the purposes of EXP-BETA-004, an Effect is any proposed external consequence that would require authority, authorization, policy evaluation, or controlled execution to occur.

Examples include:
* filesystem mutation
* network communication
* tool invocation
* external system interaction
* state transition requests

An Effect Proposal describes intended consequence only. It does not represent permission, authorization, or execution eligibility.

## 4. Objective
* Define a non-authoritative proposal layer for model-generated tool/effect intents.
* Establish a translation boundary toward the existing `CapabilityPayloadV1`.
* Prevent model-originated authority mutation.

**Note:** An Effect Proposal is not an authority request. It is a structured description of desired consequence that may be evaluated by a separate authority-bearing process.

## 5. Normative Invariants

* **INVARIANT-001:** An Effect Proposal is candidate intent only.
* **INVARIANT-002:** An Effect Proposal cannot authorize execution.
* **INVARIANT-003:** An Effect Proposal cannot select its own authority source.
* **INVARIANT-004:** An Effect Proposal cannot mutate the active harness or admission rules.
* **INVARIANT-005:** `CapabilityPayloadV1` remains the authority-bearing representation.
* **INVARIANT-006:** Capability evaluation remains separate from capability exercise.
* **INVARIANT-007:** An Effect Proposal cannot establish its own requester, issuer, authority scope, or execution identity.
* **INVARIANT-008:** Any Effect Proposal evaluation must preserve provenance linking the proposal to the originating model output, execution context, and evaluation decision.

## 6. Lifecycle Boundary

```text
AGENT OUTPUT
    |
    v
EFFECT_PROPOSAL-v1
(non-authoritative evidence)
    |
    v
AUTHORITY EVALUATION
(host-controlled)
    |
    +----------------+
    |                |
    v                v
REJECT          OPTIONAL CAPABILITYPAYLOADV1 CANDIDATE
                     |
                     v
              CAPABILITY V1 ADMISSION
                     |
                     v
              AUTHORIZED STATE
```

## 7. Rejection Boundary
Failure, rejection, or denial of an Effect Proposal does not create permission to seek alternative authority paths, bypass controls, or modify evaluation conditions.

A denied proposal remains denied unless a separate authorized process creates a new evaluation context.

## 8. Explicit Non-Scope
This specification explicitly does **not** implement:
* Runtime tool invocation
* Delegation
* Multi-agent authority
* Requester binding
* Revocation
* Freshness semantics
* Capability exercise
* Autonomous harness modification
* Self-improving authority loops

## 9. RSI / Co-Evolution Boundary
Recursive Self-Improvement (RSI) necessitates strict separation between the intelligence producing data and the architecture governing it.

**Core Principle:** A system may generate evidence suggesting future model or harness evolution, but active authority controls cannot be modified by the intelligence they currently govern.

This preserves the dual-plane architecture:

**INTELLIGENCE PLANE**
* Reasoning
* Proposals
* Evidence
* Optimization

**AUTHORITY PLANE**
* Permissions
* Admission
* Policy
* Execution boundaries
