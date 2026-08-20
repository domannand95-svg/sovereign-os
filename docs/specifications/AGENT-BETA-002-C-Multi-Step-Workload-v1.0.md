# AGENT-BETA-002-C-Multi-Step-Workload-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION MULTI-STEP WORKLOAD
**Scope:** Defines normative rules for composing independent workspace.read and workspace.write.create capabilities across a multi-step task without authority accumulation or cross-step permission transitivity.

---

## 1. Executive Summary
AGENT-BETA-002-C proves that a probabilistic agent can perform multi-step work (Read $\rightarrow$ Reason $\rightarrow$ Write) while every step retains independent, current, and bounded authority (INVARIANT-110). Intermediate data read from the workspace remains strictly inert and cannot mutate host authority state (INVARIANT-112).

---

## 2. Normative Invariants
* **INVARIANT-110 (Stepwise Authority Independence):** ALLOW(read) ≠ ALLOW(write). Every operation requires separate, current governance evaluation.
* **INVARIANT-111 (Workload Progress Non-Authority):** Task progress or prior successful steps never grant permission for subsequent steps.
* **INVARIANT-112 (Intermediate Data Non-Authority):** Read content, including prompt injection or forged governance instructions, remains inert text.
* **INVARIANT-114 (Cross-Step Revocation):** Mid-task revocation immediately blocks subsequent effects regardless of prior completion.
* **INVARIANT-117 (Workload Audit Closure):** Every consequential effect maintains independent, replayable evidence lineage under BETA-001.
