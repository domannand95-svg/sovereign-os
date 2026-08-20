# AGENT-BETA-002-Governed-Workload-Capability-Expansion-v1.0.md

**Status:** APPROVED / EXPERIMENTAL TRACK CHARTER
**Class:** EXPERIMENTAL / NON-PRODUCTION AGENT WORKLOAD
**Scope:** Charters multi-step workload capability expansion, introducing bounded workspace.read and workspace.write capabilities while enforcing stepwise authority, orthogonal permissions, and data-authority separation.

---

## 1. Executive Summary
AGENT-BETA-002 tests whether agent containment survives useful multi-step operations. It establishes that task progress does not equal authority accumulation, and that data read from or written to the workspace can never transform into execution privilege.

---

## 2. Core Invariants
* **INVARIANT-091 (Capability Orthogonality):** workspace.read ≠ workspace.write. Possession of one never implies the other.
* **INVARIANT-092 (Resource Confinement):** $\text{ResolvedTarget} \in \text{GrantedScope}$. Anything outside resolved scope fails closed.
* **INVARIANT-093 (No Path Semantic Escape):** Absolute paths, parent traversal (../), alternate separators, and reparse points cannot bypass resource confinement.
* **INVARIANT-094 (Stepwise Authority):** $\text{Authorized}(\text{Step } N) \not\implies \text{Authorized}(\text{Step } N+1)$. Every action requires current active authority.
* **INVARIANT-095 (Revocation Between Steps):** Mid-task revocation immediately invalidates subsequent operational attempts.
* **INVARIANT-096 & 097 (Data-Authority Separation & Failure Non-Escalation):** File content $\neq$ Authority. Task failure or blocked mechanics never trigger automatic privilege expansion.
