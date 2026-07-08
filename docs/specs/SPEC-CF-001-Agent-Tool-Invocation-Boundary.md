# SPEC-CF-001: Agent Tool Invocation Boundary

**Subsystem:** Capability Firewall  
**Title:** Agent Tool Invocation Boundary  
**Document ID:** SPEC-CF-001  
**Version:** 1.0  
**Status:** Approved / Locked  
**Classification:** Technical Architecture Contract  
**Owner:** Architecture Review Board (ARB)  
**Reviewers:** Core Contributors, Security Review Board  
**Last Updated:** 2026-07-09

---

# 1. Purpose

This specification defines the deterministic execution contract between external agent infrastructure and the Sovereign OS Production Core.

It establishes the mandatory authentication, validation, authorization, execution, logging, and promotion requirements governing all tool invocations crossing the Capability Firewall.

This document provides the implementation contract beneath the constitutional architecture defined within the Engineering Handbook while preserving the architectural invariants established by ARCH-001 through ARCH-007.

---

# 2. Scope

This specification governs:

- external agent requests;
- collaborator agent frameworks;
- tool definitions;
- prompt configurations;
- workspace profiles;
- memory objects;
- runtime policy evaluation;
- capability authorization;
- sandbox execution;
- deterministic logging;
- production promotion.

This specification SHALL apply to every executable artifact operating within or requesting services from the Discovery Layer.

---

# 3. Objectives

The Capability Firewall SHALL guarantee that:

- every execution request is authenticated;
- every executable artifact possesses a governed identity;
- every invocation is evaluated against active policy;
- every execution is deterministic from the perspective of the authoritative event log;
- every execution produces immutable audit evidence;
- speculative execution remains isolated from Production Core authority;
- no execution path bypasses constitutional governance.

---

# 4. Architectural Principles

The Capability Firewall exists to preserve the constitutional separation between speculative computation and authoritative production execution.

Accordingly, conforming implementations SHALL satisfy the following architectural principles.

## 4.1 Constitutional Authority

The operating system kernel SHALL remain the sole authority responsible for granting execution privileges.

Large language models, external orchestration frameworks, or autonomous agents SHALL possess advisory authority only.

They MAY request execution.

They SHALL NOT authorize execution.

---

## 4.2 Representation Independence

The firewall SHALL operate independently of:

- programming language;
- operating system;
- database engine;
- serialization format;
- orchestration framework;
- deployment topology.

Reference implementations MAY utilize Rust, Go, C++, Java, Python, or equivalent technologies provided all observable behaviour conforms to this specification.

---

## 4.3 Deterministic Behaviour

Identical authenticated requests evaluated against identical registry state, policy state, and governed artifacts SHALL produce observationally equivalent execution decisions.

Implementation details SHALL NOT alter constitutional behaviour.

---

## 4.4 Complete Auditability

Every execution request SHALL generate immutable audit evidence.

Silent execution is prohibited.

Every execution SHALL be reconstructable from the authoritative event ledger.

---

## 4.5 Fail-Closed Integrity

When uncertainty exists regarding:

- authentication;
- provenance;
- policy;
- capability;
- registry integrity;
- execution safety;

the Capability Firewall SHALL deny execution.

Integrity SHALL take precedence over availability.

---

# 5. Invocation Pipeline

Every invocation SHALL traverse the complete authorization pipeline.

No stage MAY be bypassed.

```text
Agent Request
        │
                ▼
                Authentication
                        │
                                ▼
                                Schema Validation
                                        │
                                                ▼
                                                Content Address Validation (CAID)
                                                        │
                                                                ▼
                                                                Policy Evaluation
                                                                        │
                                                                                ▼
                                                                                Capability Firewall
                                                                                        │
                                                                                                ├─────────────┬─────────────┬──────────────┐
                                                                                                        ▼             ▼             ▼              ▼
                                                                                                            ALLOWED       DENIED      THROTTLED     QUARANTINED
                                                                                                                    │             │             │              │
                                                                                                                            ▼             ▼             ▼              ▼
                                                                                                                            Sandbox Runner   Audit Event   Queue Policy   Isolated Sandbox
                                                                                                                            ```

                                                                                                                            Every stage SHALL emit sufficient telemetry to support deterministic replay.

                                                                                                                            ---

                                                                                                                            # 6. Authentication

                                                                                                                            Every invocation SHALL originate from an authenticated execution context.

                                                                                                                            Authentication SHALL establish:

                                                                                                                            - requesting agent identity;
                                                                                                                            - workspace identity;
                                                                                                                            - execution session;
                                                                                                                            - credential source;
                                                                                                                            - authorization domain.

                                                                                                                            Unauthenticated requests SHALL terminate immediately.

                                                                                                                            The firewall SHALL emit an immutable denial event before returning control to the requesting runtime.

                                                                                                                            ---

                                                                                                                            # 7. Schema Validation

                                                                                                                            Every executable artifact SHALL possess an immutable schema.

                                                                                                                            The schema SHALL define:

                                                                                                                            - identifier;
                                                                                                                            - version;
                                                                                                                            - input schema;
                                                                                                                            - output schema;
                                                                                                                            - declared capabilities;
                                                                                                                            - execution domain;
                                                                                                                            - resource limits;
                                                                                                                            - timeout policy;
                                                                                                                            - provenance metadata;
                                                                                                                            - content-derived identifier.

                                                                                                                            Malformed schemas SHALL NOT enter execution.

                                                                                                                            Schema validation SHALL occur before policy evaluation.

                                                                                                                            ---

                                                                                                                            # 8. Content-Derived Identity

                                                                                                                            Every executable artifact SHALL possess a Content Addressable Identifier (CAID).

                                                                                                                            The authoritative identity SHALL be derived exclusively from the canonical serialized representation of the governed artifact.

                                                                                                                            Informatively:

                                                                                                                            ```text
                                                                                                                            CAID = H(canonical_content)
                                                                                                                            ```

                                                                                                                            Logical filenames, repository paths, aliases, network locations, database keys, or deployment directories SHALL NOT constitute authoritative identity.

                                                                                                                            Every execution request SHALL reference governed artifacts exclusively through CAIDs.

                                                                                                                            ---

                                                                                                                            # 9. Policy Evaluation

                                                                                                                            Following successful authentication and schema validation, the invocation SHALL be evaluated by the Policy Engine.

                                                                                                                            The Policy Engine SHALL determine:

                                                                                                                            - requested capabilities;
                                                                                                                            - privilege level;
                                                                                                                            - provenance validity;
                                                                                                                            - epistemic state;
                                                                                                                            - resource budget;
                                                                                                                            - execution authority;
                                                                                                                            - governance constraints.

                                                                                                                            The Policy Engine SHALL produce exactly one execution decision.

                                                                                                                            The Capability Firewall SHALL enforce that decision without modification.

                                                                                                                            ---

                                                                                                                            # 10. Capability Firewall

                                                                                                                            The Capability Firewall SHALL enforce the execution decision returned by the Policy Engine.

                                                                                                                            The firewall SHALL NOT reinterpret policy.

                                                                                                                            Its responsibility is enforcement.

                                                                                                                            Permitted execution outcomes are:

                                                                                                                            - ALLOWED
                                                                                                                            - DENIED
                                                                                                                            - THROTTLED
                                                                                                                            - QUARANTINED

                                                                                                                            Subsequent sections formally define each execution state together with mandatory kernel behaviour.

                                                                                                                            ---

                                                                                                                            **End of Part A**
 # 11. Firewall Enforcement States

 Every invocation reaching the Capability Firewall SHALL terminate in exactly one execution state.

 Execution states are mutually exclusive.

 No implementation SHALL return multiple execution states for a single invocation.

 | State | Description | Mandatory Kernel Action |
 |--------|-------------|-------------------------|
 | **ALLOWED** | Authentication, schema validation, CAID validation, provenance verification, and policy evaluation succeed. | Record the invocation in the authoritative Event Ledger and forward execution to an approved sandbox runtime. |
 | **DENIED** | Authentication failure, invalid schema, missing provenance, invalid CAID, or policy violation. | Reject execution immediately, emit an immutable denial event, and return a deterministic error to the requesting agent. |
 | **THROTTLED** | Invocation exceeds configured execution budgets or platform resource policies. | Apply deterministic scheduling or rate limiting while preserving Production Core stability. |
 | **QUARANTINED** | Invocation originates from speculative artifacts or requires elevated containment. | Redirect execution into an isolated Discovery Layer sandbox with no production authority. |

 ---

 # 12. Capability Descriptors

 Tools SHALL declare every privileged operation prior to execution.

 Representative capability descriptors include:

 ```text
 filesystem.read
 filesystem.write
 registry.lookup
 registry.update
 database.query
 network.http
 event.emit
 workspace.spawn
 gpu.compute
 shell.execute
 ```

 Policies SHALL authorize capability descriptors rather than arbitrary executable names.

 A tool SHALL NOT request capabilities absent from its governed schema.

 ---

 # 13. Deterministic Logging

 Every invocation SHALL generate immutable audit evidence.

 Silent execution SHALL NOT occur.

 Every event SHALL include sufficient metadata to reconstruct execution history.

 The minimum event record SHALL include:

 - Invocation Identifier
 - Agent Identifier
 - Workspace Identifier
 - Session Identifier
 - Parent Event Identifier
 - Correlation Identifier
 - Tool CAID
 - Request CAID
 - Policy Version
 - Timestamp
 - Event Type
 - Result State
 - Provenance References
 - Log Sequence Number (LSN)

 ---

 ## 13.1 Event Invariants

 Every invocation SHALL generate:

 1. Invocation Requested
 2. Firewall Decision
 3. Sandbox Started (if applicable)
 4. Sandbox Completed (if applicable)
 5. Result Recorded

 Failure to emit any mandatory event SHALL constitute a registry integrity violation.

 ---

 # 14. Result States

 Every execution SHALL terminate in exactly one result state.

 | Result State | Description |
 |--------------|-------------|
 | **COMPLETED** | Execution completed successfully within policy constraints. |
 | **FAILED** | Execution terminated due to internal runtime failure. |
 | **TIMED_OUT** | Execution exceeded deterministic execution budgets. |
 | **CANCELLED** | Execution cancelled by an authorized operator or policy engine. |
 | **QUARANTINED** | Execution completed inside an isolated environment and was denied promotion. |
 | **ABORTED** | Execution terminated due to integrity, provenance, or constitutional violations. |

 ---

 # 15. Resource Budgets

 Every executable artifact SHALL declare deterministic resource limits.

 Representative limits include:

 - CPU quota
 - Memory quota
 - GPU quota
 - Network quota
 - Filesystem quota
 - Token quota
 - Maximum concurrency
 - Wall-clock timeout

 Artifacts exceeding declared limits SHALL transition into either the THROTTLED or TIMED_OUT execution state.

 Unbounded execution SHALL NOT be permitted.

 ---

 # 16. Deterministic Timeout Behaviour

 The Capability Firewall SHALL terminate execution exceeding approved runtime budgets.

 Termination SHALL generate:

 ```text
 Event::InvocationTimedOut
 ```

 Timeout events SHALL include:

 - Invocation Identifier
 - Tool CAID
 - Runtime Duration
 - Resource Budget
 - Sandbox Identifier
 - LSN
 - Timestamp

 Timeout behaviour SHALL be deterministic and reproducible across conforming implementations.

 ---

 # 17. Production Isolation

 Discovery Layer execution SHALL remain permanently isolated from Production Core authority.

 Discovery Layer runtimes SHALL NOT possess:

 - direct registry mutation authority;
 - production filesystem write access;
 - privileged hardware interfaces;
 - actuator control;
 - kernel memory access;
 - unrestricted network authority.

 All requests affecting Production Core state SHALL be expressed as declarative proposals requiring constitutional validation.

 ---

 # 18. Promotion Through the Epistemic Hierarchy

 Artifacts originating within the Discovery Layer SHALL begin in speculative epistemic states.

 Promotion into Production Core authority SHALL require successful progression through the verification gates defined by ARCH-004.

 Promotion SHALL NOT occur automatically.

 Every promotion SHALL generate immutable governance evidence.

 ---

 # 19. Policy Engine Separation

 The Policy Engine determines authorization.

 The Capability Firewall enforces authorization.

 Implementations MAY replace the internal Policy Engine provided the externally observable behaviour of the Capability Firewall remains fully compliant with this specification.

 This separation preserves representation independence while maintaining constitutional consistency.

 ---

 **End of Part B**                     # 20. Deterministic Execution Contract

 Conforming implementations SHALL ensure that identical authenticated requests executed against identical:

 - registry state;
 - policy state;
 - governed tool schema;
 - executable implementation;
 - input payload;

 produce observationally equivalent execution results.

 Where interaction with inherently non-deterministic external systems is required, the boundary SHALL be explicitly declared by policy and recorded within the authoritative Event Ledger.

 Undeclared sources of non-determinism SHALL constitute a constitutional policy violation.

 ---

 # 21. Sandbox Requirements

 Every Discovery Layer execution environment SHALL satisfy the following isolation properties.

 ## 21.1 Temporal Isolation

 Speculative workloads SHALL NOT interfere with Production Core scheduling guarantees.

 ## 21.2 Spatial Isolation

 Discovery Layer processes SHALL execute within isolated memory regions independent of authoritative production state.

 ## 21.3 Hardware Isolation

 Discovery workloads SHALL NOT directly access:

 - actuator interfaces;
 - production hardware drivers;
 - privileged kernel services;
 - safety-critical control paths.

 ## 21.4 Network Isolation

 External network access SHALL be explicitly governed by policy.

 Network capabilities SHALL NOT be implicitly granted.

 ---

 # 22. Failure Handling

 Failure SHALL be deterministic.

 Failure SHALL produce immutable evidence.

 Failure SHALL NOT corrupt authoritative state.

 Representative failure classes include:

 - authentication failure;
 - schema validation failure;
 - CAID mismatch;
 - provenance failure;
 - policy violation;
 - timeout;
 - resource exhaustion;
 - sandbox failure;
 - runtime panic.

 Each failure SHALL emit a corresponding immutable event before control returns to the requesting runtime.

 ---

 # 23. Security Considerations

 The Capability Firewall constitutes the primary constitutional security boundary separating speculative computation from authoritative execution.

 Accordingly, implementations SHALL protect against:

 - privilege escalation;
 - unauthorized registry mutation;
 - execution outside approved capability descriptors;
 - provenance spoofing;
 - CAID forgery;
 - replay attacks;
 - undeclared network communication;
 - unauthorized hardware access;
 - policy bypass.

 Security policy SHALL default to deny unless explicitly authorized.

 ---

 # 24. Observability

 Every invocation SHALL support complete operational observability.

 Implementations SHOULD expose telemetry suitable for:

 - deterministic replay;
 - distributed tracing;
 - audit reconstruction;
 - performance profiling;
 - policy verification;
 - constitutional compliance assessment.

 Observability data SHALL NOT become authoritative state unless committed to the Event Ledger.

 ---

 # 25. Reference Implementation Roadmap

 The Architecture Review Board establishes the following implementation sequence.

 1. Normalize governed tool schemas.
 2. Compute Content Addressable Identifiers (CAIDs).
 3. Implement authentication adapters.
 4. Implement Policy Engine interfaces.
 5. Implement Capability Firewall enforcement.
 6. Emit immutable invocation events.
 7. Execute within Discovery Layer sandboxes.
 8. Emit deterministic result events.
 9. Validate replay compatibility.
 10. Expand to multi-agent execution.

 This sequence minimizes implementation risk while preserving constitutional invariants.

 ---

 # 26. Out of Scope

 This specification SHALL NOT define:

 - operating system scheduling algorithms;
 - distributed consensus protocols;
 - formal verification tooling;
 - model training procedures;
 - neural network architectures;
 - laboratory instrumentation;
 - physical process control;
 - hardware implementation details.

 These concerns are delegated to subordinate specifications.

 ---

 # 27. Related Documents

 ### Constitutional Architecture

 - HB-CH03 — Mathematical Foundations
 - HB-CH04 — Governance & Epistemic Hierarchy
 - HB-CH05 — Capability Firewall & Security Model
 - HB-CH06 — Registry & Identity Services
 - HB-CH07 — Discovery Layer
 - HB-CH08 — Verification Framework
 - HB-CH09 — Reference Implementation Methodology

 ### Constitutional Invariants

 - ARCH-001 — Authoritative Sourcing
 - ARCH-002 — Production Immutability
 - ARCH-003 — Capability Isolation
 - ARCH-004 — Epistemic Progression
 - ARCH-005 — Invariant Dominance
 - ARCH-006 — Content-Derived Identity
 - ARCH-007 — Provenance Integrity

 ---

 # 28. Compliance

 A conforming implementation SHALL satisfy every normative requirement contained within this specification.

 Failure to satisfy any mandatory requirement SHALL constitute non-conformance with SPEC-CF-001.

 Future revisions MAY extend this specification.

 Future revisions SHALL NOT weaken or contradict the constitutional invariants defined by ARCH-001 through ARCH-007 without formal Architecture Review Board approval.

 ---

 # Approval

 **Document ID:** SPEC-CF-001

 **Status:** Approved / Locked

 **Classification:** Technical Architecture Contract

 **Approved By:** Architecture Review Board (ARB)

 **Baseline:** Constitutional Architecture v1.0

 ---

 **End of Document**                                                                                                      