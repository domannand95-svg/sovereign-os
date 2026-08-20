# AGENT-BETA-002-D-Adversarial-Multi-Step-Composition-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION ADVERSARIAL COMPOSITION
**Scope:** Defines normative rules and verification test suites for multi-step adversarial compositions, bounding-authority prompt injections, TOCTOU revocation races, and cross-resource isolation.

---

## 1. Executive Summary
AGENT-BETA-002-D proves that individually secure capabilities (workspace.read and workspace.write) maintain absolute security when subjected to aggressive composition attacks, including mid-task revocations, state changes between evaluation and execution, and prompt-injection attacks targeting agents that genuinely possess bounded write privileges.

---

## 2. Normative Invariants
* **INVARIANT-120 (No Capability Union by Convenience):** Two narrow grants never silently combine into a broader effective grant (Grant A + Grant B ≠ Union).
* **INVARIANT-122 (Denial Dominance):** Any denial in a composite trace immediately dominates (ALLOW + ALLOW + DENY + ALLOW = DENY).
* **INVARIANT-126 (Concurrent State Change / TOCTOU Resistance):** Revocation occurring between evaluation and adapter execution immediately resolves to DENY.
* **INVARIANT-127 (Partial Success Non-Authority):** Partial completions never permit the invention of compensating unauthorized effects elsewhere.
