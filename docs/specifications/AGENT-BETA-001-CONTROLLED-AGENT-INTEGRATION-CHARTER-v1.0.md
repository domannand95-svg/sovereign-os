# AGENT-BETA-001-CONTROLLED-AGENT-INTEGRATION-CHARTER-v1.0.md

**Status:** APPROVED / EXPERIMENTAL TRACK CHARTER
**Class:** ARCHITECTURAL SPECIFICATION & GOVERNANCE BOUNDARY
**Scope:** Chartering the introduction of the first governed, sandboxed probabilistic agent (AGENT-BETA-001) behind the Sovereign OS authority substrate.

---

## 1. Executive Summary
With BETA-001 (Provenance), BETA-002 (Negotiation), and BETA-003 (Enforcement) fully frozen, Sovereign OS is ready to host its first controlled agent workload. The agent remains strictly a requester of intent, entirely bounded by deterministic infrastructure.

---

## 2. Core Operational Constraints
1. **Single Task Isolation:** One agent, one bounded task at a time.
2. **Zero Ambient Authority:** No direct filesystem, network, or registry access without traversed capability requests.
3. **No Self-Authorization:** Agents cannot issue, renew, or escalate capabilities.
4. **Complete Traceability:** Every agent action maps cleanly through request, evaluation, negotiation, grant, enforcement, and audit binding.
