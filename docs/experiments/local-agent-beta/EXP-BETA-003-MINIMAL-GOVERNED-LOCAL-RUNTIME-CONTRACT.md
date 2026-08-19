# EXP-BETA-003 — Minimal Governed Local Runtime Contract

**Status:** FROZEN / EXPERIMENTAL

**Constraint Level:** NON-PRODUCTION / LOCAL BETA EXPERIMENT

**Allocation Baseline:** `main@7f8bf00f90a6d2bef9ae9ef5519cace248dfb65b`

**Implementation Branch:** `spec/exp-beta-003-minimal-governed-local-runtime`

---

## 1. Objective

EXP-BETA-003 shall demonstrate the smallest governed runtime boundary through which raw bytes produced by a local probabilistic inference provider may enter the Sovereign OS beta harness.

The experiment shall prove that inference output is treated as untrusted candidate material and cannot acquire host authority merely because it was produced by a model process.

The governing maxim remains:

> **Probabilistic intelligence; deterministic authority.**

---

## 2. Normative Boundary

The minimal runtime pipeline is:

```text
Untrusted Local Inference
        |
        v
Preserved Raw Output Bytes
        |
        v
EXP-BETA-002 RawOutputAdapter
        |
        v
Deterministic Runtime Disposition
        |
        +--> Containment / Runtime Observation
        |
        +--> Evidence Assembly
        v
Auditable Experimental Result
```

No stage above grants model output authority to mutate governed state, expand capability scope, authorize tools, admit evidence, or perform external effects.

---

## 3. Primary Invariants

### 3.1 Intelligence Is Not Authority

Inference output is data only.

A model response cannot authorize execution, mutate policy, enlarge runtime scope, issue a capability, issue a context grant, admit itself as authoritative evidence, or promote its own conclusions.

### 3.2 Raw Output Preservation

The exact inference bytes presented to the runtime boundary shall remain available for forensic evidence.

No semantic repair, field synthesis, prose extraction, field renaming, coercion, or model-assisted normalization is permitted.

### 3.3 EXP-BETA-002 Remains the Admission Boundary

All candidate model output entering EXP-BETA-003 shall pass through `RawOutputAdapter::admit`.

EXP-BETA-003 shall not create a competing model-output normalization path.

### 3.4 Runtime Failure Is Fail-Closed

Provider failure, malformed output, adapter rejection, containment failure, telemetry failure, evidence failure, or evaluation failure shall not expand authority or silently bypass governance.

### 3.5 Provider Replaceability

The deterministic runtime contract shall not depend on Ollama-specific authority semantics.

Ollama may later serve as an experimental local inference provider, but it remains a replaceable source of untrusted bytes.

### 3.6 No Ambient Host Authority

The inference provider receives no authority merely by being invoked.

Model-controlled filesystem paths, arbitrary shell execution, arbitrary external networking, privilege escalation, package installation, and unrestricted host process control are outside this experiment.

### 3.7 Observation Is Independent of Model Self-Report

Containment and runtime telemetry are harness-controlled observations.

Model claims concerning its own compliance do not substitute for independent observation.

### 3.8 Candidate Evidence Is Not Authoritative Admission

A successfully parsed or sealed experimental artifact remains candidate/audit evidence.

EXP-BETA-003 does not allocate production admission, ledger mutation, autonomous promotion, or other governed-state authority.

---

## 4. First Implementation Slice

The first Rust implementation slice shall use **injected inference bytes only**.

It shall not start, install, configure, discover, or contact Ollama.

The slice shall establish a deterministic harness-owned runtime function whose conceptual input is:

```text
runtime constraints + untrusted raw inference bytes
```

and whose conceptual output records:

```text
preserved raw bytes
+ EXP-BETA-002 admission disposition
+ deterministic runtime outcome
+ harness-controlled observation/evidence state
```

The precise Rust API shall be derived from the existing beta harness surfaces rather than inventing parallel containment, telemetry, evidence, or adapter abstractions.

---

## 5. Required First-Slice Tests

The first implementation slice shall prove at minimum:

1. identical injected raw bytes and identical runtime inputs replay deterministically;
2. raw inference bytes are preserved exactly;
3. an EXP-BETA-002 rejection remains rejected at the runtime boundary;
4. permitted bounded normalization is not reimplemented by the runtime layer;
5. runtime failure cannot convert rejection into admission;
6. no provider invocation is required to test the deterministic runtime boundary;
7. the implementation introduces no model-controlled filesystem, shell, or network authority.

---

## 6. Explicitly Out of Scope

The following are not authorized by EXP-BETA-003:

- Tauri or other desktop packaging;
- Base44 integration;
- SOL visual identity or installer work;
- Ollama installation;
- automatic model download or model pulling;
- model-provider lifecycle management;
- arbitrary tool execution;
- model-controlled filesystem access;
- unrestricted network access;
- persistent autonomous agents;
- production capability issuance;
- production ContextGrant issuance;
- Issue #174 authority allocation;
- autonomous evidence admission;
- ledger or governed-state mutation;
- BKI repository mutation;
- EXP-BETA-004 or EXP-BETA-005 implementation.

---

## 7. Experimental Provider Boundary

A later EXP-BETA-003 slice may bind a local inference provider only after the injected-byte runtime boundary is independently proven.

That provider binding must terminate at a narrow interface that returns untrusted response bytes and provider-status metadata.

Provider transport is not an authority channel.

---

## 8. Closure Questions

EXP-BETA-003 shall not close until the experiment can answer:

1. Can a real local inference result enter SOL only through the deterministic adapter/runtime boundary?
2. Can raw probabilistic output be preserved while deterministic admission remains authoritative?
3. Can provider/runtime failure remain fail-closed without permission expansion?
4. Can containment and telemetry remain independent of model self-report?
5. Can the provider remain replaceable without changing the authority model?
6. What additional authority contract, if any, must be allocated before a governed live-model beta may exercise tools or context capabilities?

---

## 9. Governance Boundary

This document allocates an experimental local runtime boundary only.

It does not authorize production effects, packaging, autonomous capability use, model self-authorization, or expansion into EXP-BETA-004 / EXP-BETA-005.

Any such transition requires a separately allocated and explicitly authorized gate.
