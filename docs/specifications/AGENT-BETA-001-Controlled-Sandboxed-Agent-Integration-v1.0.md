# AGENT-BETA-001-Controlled-Sandboxed-Agent-Integration-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION AGENT INTEGRATION
**Scope:** Defines normative boundaries, isolation contracts (AgentBackend / AgentHost), and adversarial acceptance criteria for hosting a probabilistic agent behind the Sovereign OS governance spine.

---

## 1. Executive Summary
AGENT-BETA-001 establishes the first governed sandboxed agent integration. It proves that a probabilistic agent can perform a bounded task while every consequential action remains strictly mediated, constrained, and audited by deterministic Sovereign OS authority controls without granting ambient privileges or allowing model persuasion to alter authority outcomes.

---

## 2. Normative Invariants
* **INVARIANT-076 (Agent Non-Authority):** Agent reasoning, output, confidence, or identity cannot constitute execution authority (Agent Output ≠ External Effect).
* **INVARIANT-077 (No Ambient Capability):** An agent begins every bounded task with zero effective capabilities ($\text{EffectiveCapabilities} = \emptyset$).
* **INVARIANT-078 (Effects Require Governed Transit):** Every consequential effect must traverse the complete BETA-002/003 governance and enforcement spine.
* **INVARIANT-079 (Denial Persistence):** A denied action cannot become allowed due to repetition, paraphrasing, urgency claims, or persuasion (AGENT-001-AUTHORITY-INDEPENDENCE).
* **INVARIANT-080 (Failure Does Not Expand Authority):** Task failure or blocked mechanics never grant permission to bypass or circumvent.
* **INVARIANT-081 (Context Is Non-Authority):** Supplied context or retrieved memory informs reasoning but grants zero execution rights.
* **INVARIANT-082 (No Authority Persistence Across Tasks):** Completed sessions cannot donate capabilities to subsequent tasks.
* **INVARIANT-083 (Agent Cannot Fabricate Governance State):** Model-generated representations of grants, admissions, or audits remain inert text and cannot establish real state.
* **INVARIANT-084 (Effects Are Auditable):** Real external effects require unbroken verifiable evidence lineage back to BETA-001.
