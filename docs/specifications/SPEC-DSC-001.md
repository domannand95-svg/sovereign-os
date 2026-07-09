# SPEC-DSC-001: Discovery Layer Runtime

**Subsystem:** sovereign-discovery

**Title:** Discovery Layer Runtime

**Document ID:** SPEC-DSC-001

**Version:** 1.0

**Status:** Draft

**Classification:** Technical Subsystem Contract

**Owner:** Architecture Review Board

**Last Updated:** 2026-07-09

---

# 1. Purpose

This specification defines the functional, structural, and behavioral interfaces for the `sovereign-discovery` crate.

The Discovery Layer Runtime serves as the untrusted, unprivileged sandbox execution substrate for Sovereign OS. It provides complete spatial, temporal, and capability isolation for speculative operations, continuous machine learning inference pipelines, autonomous agent frameworks, and unvalidated experimental software tools.

By modeling the entire discovery domain as a hostile execution boundary, this subsystem ensures that arbitrary code execution failures, infinite planning loops, or malicious payload manipulation cannot bleed into the production kernel, mutate authoritative state vectors, or degrade real-time physical plant control dynamics.

---

# 2. Scope

The specifications detailed within this contract govern the runtime lifecycle of unprivileged execution sandboxes, virtual resource quota limits (CPU, memory, thread mapping), deterministic WebAssembly, eBPF, or containerized boundary execution traps, and the Foreign Function Interface (FFI) communication channels crossing the Capability Firewall.

This document defines the abstract virtualization interface and supervisor primitives implemented by the `sovereign-discovery` crate.

It explicitly excludes the host operating system kernel security implementation, physical hardware virtualization extensions (such as AMD-V or Intel VT-x), and the internal optimization strategies of individual machine learning models.

---

# 3. Requirements Matrix

| Requirement ID | Domain Area | Operational Constraint / Behavioral Requirement | Cross-Reference |
| --- | --- | --- | --- |
| DSC-REQ-001 | Absolute Containment | The discovery runtime SHALL isolate all speculative agent execution tasks inside a zero-privilege, memory-fenced address space completely detached from production memory pages. | ARCH-003, HB-CH07 |
| DSC-REQ-002 | Interception Control | Every tool invocation, external system call, or resource fetch requested by a sandboxed actor MUST be captured by a runtime supervisor and forwarded to the Policy Engine for authorization. | SPEC-POL-001, SPEC-CF-001 |
| DSC-REQ-003 | Resource Quota Fencing | The execution supervisor MUST enforce immutable resource ceilings for CPU time, stack allocation, heap usage, thread count, and execution duration. | HB-CH07 |
| DSC-REQ-004 | Failure Independence | Panics, segmentation faults, infinite loops, or resource exhaustion occurring inside a sandbox SHALL NOT alter production execution paths or induce latency jitter in deterministic runtime components. | HB-CH07 |
| DSC-REQ-005 | Transient Lifecycle | Sandbox storage SHALL remain ephemeral. All allocations MUST be securely reclaimed and zeroed after termination unless explicitly promoted through the governed verification pathway. | ARCH-002, HB-CH06 |
| DSC-REQ-006 | Deterministic Logging | The runtime supervisor SHALL capture all execution metadata, tool requests, inputs, outputs, and lifecycle transitions as deterministic event records for ingestion by the Event Ledger. | SPEC-EVT-001 |
| DSC-REQ-007 | Promotion Pathing | The runtime SHALL expose explicit promotion interfaces allowing validated sandbox outputs to progress through the Epistemic Hierarchy under governed review. | ARCH-004, SPEC-REG-001 |
| DSC-REQ-008 | Asynchronous Decoupling | Communication across the sandbox isolation boundary MUST utilize asynchronous, lock-free message queues to prevent compromised or stalled workloads from blocking production execution. | HB-CH02 |


---

## 4. Architecture

### 4.1 Crate Architecture and Directory Topography

The `sovereign-discovery` crate implements the isolated, zero-privilege virtualization and execution runtime for speculative workloads, tools, and automated agents.

It structures sandbox supervisors and asynchronous resource channels to act as the primary hostile boundary enclosure for the platform core. The internal directory topology SHALL structurally separate guest sandbox abstraction drivers from host-side supervisor boundaries.

```text
crates/sovereign-discovery/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── sandbox.rs
│   ├── supervisor.rs
│   ├── quota.rs
│   ├── ffi.rs
│   ├── queue.rs
│   └── error.rs
├── tests/
│   └── sandbox_isolation_tests.rs
└── benches/
    └── context_switch_overhead.rs
```

### 4.2 Sandbox Isolation Mechanics and Virtualization Layer

Every unprivileged payload executed within this crate MUST run inside a constrained virtual target machine context, such as deterministic WebAssembly core execution loops or hardened eBPF container cells.

The allocation of computing units, virtual scratchpad pages, and peripheral channels SHALL be constrained by a quota controller, preventing compromised guest tasks from causing CPU starvation, memory exhaustion, or infinite planning locks on host threads.

---

## 5. Public Interfaces

### 5.1 Public Interface Primitives

```rust
use sovereign_core_asm::StateVector;
use sovereign_ledger::Lsn;
use sovereign_policy::PolicyDecision;
use sovereign_registry::Caid;

/// Globally unique runtime descriptor for an active isolated sandbox instance.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SandboxId(pub u128);

/// Resource limits applied to an isolated sandbox instance.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SandboxQuota {
    pub max_memory_bytes: usize,
    pub max_stack_depth: u32,
    pub instruction_quota: u64,
    pub microsecond_timeout: u64,
}

/// Operational states for a sandbox execution thread.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SandboxStatus {
    Initializing,
    Executing,
    Suspended,
    Terminated { exit_code: i32 },
    Faulted { error: SandboxError },
}

/// Isolation failures and virtualization faults.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SandboxError {
    MemoryAccessViolation,
    InstructionQuotaExceeded,
    ExecutionTimeout,
    FirewallPolicyViolation,
    LockFreeQueueOverflow,
    IllegalHostCall(u32),
}

/// Core supervisor contract governing hostile discovery layer runtimes.
pub trait DiscoveryRuntimeSupervisor {
    type MessageStream<'a>: Iterator<Item = &'a [u8]> + 'a
    where
        Self: 'a;

    fn spawn_sandbox(
        &mut self,
        image_caid: Caid,
        limits: SandboxQuota,
    ) -> Result<SandboxId, SandboxError>;

    fn terminate_sandbox(
        &mut self,
        id: SandboxId,
    ) -> Result<(), SandboxError>;

    fn poll_status(
        &self,
        id: SandboxId,
    ) -> Result<SandboxStatus, SandboxError>;

    fn dispatch_boundary_call(
        &self,
        id: SandboxId,
        tool_caid: Caid,
        payload: &[u8],
    ) -> Result<PolicyDecision, SandboxError>;

    fn stream_guest_telemetry(
        &self,
        id: SandboxId,
    ) -> Self::MessageStream<'_>;
}
```

---

## 6. Runtime Invariants

### DSC-INV-001: Spatial Memory Enclosure

Sandboxed guest execution memory regions MUST remain disjoint from the address space, register set, and writable memory structures used by the production core.

A guest workspace SHALL NOT possess pointers or references capable of dereferencing host kernel memory structures.

### DSC-INV-002: Temporal Non-Interference

Sandboxed routines, model inference tasks, and agent planning loops SHALL NOT degrade, intercept, or introduce timing jitter into hard real-time scheduling frames of production crates.

If a guest execution loop breaches its configured instruction or execution quota, the host supervisor MUST interrupt and halt the task.

### DSC-INV-003: Mandatory Firewall Interception

Every tool invocation, peripheral access request, or external data read initiated by a sandboxed guest MUST be trapped by the runtime supervisor and passed through `SPEC-POL-001`.

No host-side tool capability SHALL execute without explicit authorization from the policy firewall.

### DSC-INV-004: Transient Storage Sanitization

All scratchpad allocations, execution pages, and virtual register sets used by a sandbox instance MUST be byte-zeroed and reclaimed by the host allocator upon context termination.

### DSC-INV-005: Production Non-Mutation

Discovery runtimes SHALL NOT possess write handles or emit execution directives capable of directly modifying production state.

Generated tool outputs, proposed state updates, or experimental payloads MUST be represented as declarative, non-authoritative delta blocks requiring validation through the Epistemic Hierarchy.

