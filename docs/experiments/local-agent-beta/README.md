# Local Agent Beta Governance Scaffold

**Status:** BETA-TESTING READY / NON-PRODUCTION

**Implemented baseline:** `main@d3ce781fa647a24442b051b8b0e3a461881f0376`

**Latest closed experiment:** `EXP-BETA-002`

## Purpose

This directory defines the governed, non-authoritative test boundary for bounded local-agent context, task routing, structured candidate output, containment evidence, and deterministic raw-output admission.

The scaffold supports controlled beta testing of frozen fixtures and deterministic harness behavior. It does not activate a live production agent or grant effect authority.

## Governing Principle

> **Probabilistic intelligence; deterministic authority.**

Context access is an admitted capability, not ambient memory. Raw model output is untrusted evidence, not a candidate until it passes the frozen adapter boundary. Candidate admission is not authority approval.

The following invariants remain active:

- `ContextGrant` authorizes only explicitly enumerated context resources.
- Context possession does not imply filesystem, network, tool, mutation, or promotion authority.
- Probabilistic roles may request context but may not issue, enlarge, or delegate authority.
- JSON Schema validates artifact shape only.
- Deterministic policy validation remains responsible for role, resource, digest, budget, and grant compatibility.
- A structurally valid context request remains an unprivileged candidate request.
- Experimental `APPROVED` does not mean production execution permission.

## Implemented Artifacts

- `ROLE_REGISTRY-v1.json`
- `CONTEXT_GRANT-v1.schema.json`
- `TASK_ENVELOPE-v1.schema.json`
- `AGENT_OUTPUT-v1.schema.json`
- `CONTEXT_TELEMETRY-v1.schema.json`
- `fixtures/dry-run-sol-test-worker/`
- `fixtures/raw-output-adapter/`
- `crates/beta001-harness`

## EXP-BETA-002 Result

EXP-BETA-002 implements a deterministic raw-output admission adapter with three result classes: `ADMIT`, `ADMIT_NORMALIZED`, and `REJECT`.

It preserves original raw bytes in every result class, accepts exact schema-valid JSON, permits only exact outer `json` Markdown-fence removal, rejects uncontracted wrappers or trailing content, and fails closed on invalid UTF-8, invalid JSON, or schema violations.

Fixture 006 exposed an oracle inconsistency rather than an adapter or schema defect. The original malformed fixture remains frozen; `EXPECTED_OUTCOMES-v2.json` records its corrected rejection outcome and adds fixture 007 as the canonical valid context-request case.

See [`../../reports/EXP-BETA-002-Closure-Record.md`](../../reports/EXP-BETA-002-Closure-Record.md) for the formal findings disposition.

## Run the Beta Gate

From the repository root:

```powershell
.\scripts\verify-beta-testing.ps1
```

The gate checks frozen fixture identities and line-ending policy, formatting, Clippy, targeted EXP-BETA-002 regressions, and the full locked workspace test suite. See [`BETA-TESTING-RUNBOOK.md`](BETA-TESTING-RUNBOOK.md) for setup and interpretation.

## Explicitly Out of Scope

- Live-agent production authority
- Automatic repository mutation
- Ambient filesystem or network access
- Commit, push, merge, or branch-management authority for model workers
- Context authorization by the raw-output adapter
- Capability expansion by inference
- Production execution or promotion
- Requester binding, delegation, revocation, or state-freshness semantics
- Composite authority orchestration
- Issue #174 activation

## Release Boundary

“Beta-testing ready” means the frozen non-production harness and its deterministic test corpus can be reproduced in a controlled workspace. It does not mean packaged-node beta, public production release, or authorization to connect a live model to governed effects.
