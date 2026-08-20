# AGENT-BETA-003-Governed-Repository-Work-Charter-v1.0.md

**Status:** APPROVED / EXPERIMENTAL TRACK CHARTER
**Class:** EXPERIMENTAL / NON-PRODUCTION REPOSITORY WORKLOAD
**Scope:** Charters the repository inspection and proposal track. Establishes that repository understanding $\neq$ repository authority, and that inspection, diff generation, proposal binding, and mutation must be earned and gated separately.

---

## 1. Executive Summary
AGENT-BETA-003 extends Sovereign OS governance into version-controlled source repositories. It proves that an agent can inspect and reason about repository structure, status, and contents without acquiring file mutation, staging, commit, or push authority.

---

## 2. Core Principles & Invariants
* **INVARIANT-131 (Repository Inspection Non-Authority):** Repository contents, documentation, and comments are evidence/data for reasoning, never execution authority.
* **INVARIANT-132 (Inspection Is Non-Mutating):** epository.inspect is strictly observational. It permits zero file, index, or branch mutations.
* **INVARIANT-137 & 138 (Repository Data Is Non-Instructional):** Source code comments or fake administrative instructions inside files remain inert text. Git commands generated in model prose remain inert text (Generated command ≠ Process execution).
