# Local Agent Beta Governance Scaffold

**Status:** NON-PRODUCTION / EXPERIMENTAL SPECIFICATION

**Baseline:** `main@930d3b907cc1fabfc9c620cdeb330b1591672bba`

**Checkpoint:** `SOL-BETA-CHECKPOINT-POST-PR203`

## Purpose

This directory defines a non-authoritative experimental scaffold for testing bounded local-agent context access, task routing, structured outputs, and context telemetry.

The scaffold does not grant production execution authority and does not modify Sovereign Operating Layer capability semantics.

## Governing Principle

Context access is an admitted capability, not ambient memory.

A ContextGrant authorizes only the explicitly enumerated context resources and does not imply filesystem mutation, network access, command execution, capability exercise, admission, or promotion.

Authority boundary:

`ContextGrant authorizes context only; it does not imply tool or effect authority.`

Role registry Level 2 read classes describe eligibility only. Actual Level 2 access requires an explicit valid ContextGrant.

Probabilistic roles may request additional context but may not issue, enlarge, or delegate authority.

JSON Schema validates artifact shape only. Deterministic policy validation is responsible for role, resource, digest, budget, and grant compatibility.

ContextGrant v1 intentionally defines no expiration, revocation, or dynamic-freshness semantics.

## Current Artifacts

- `ROLE_REGISTRY-v1.json`
- `CONTEXT_GRANT-v1.schema.json`
- `TASK_ENVELOPE-v1.schema.json`
- `AGENT_OUTPUT-v1.schema.json`
- `CONTEXT_TELEMETRY-v1.schema.json`

## Planned Dry-Run Fixture

- `fixtures/dry-run-sol-test-worker/`

The dry-run fixture is not part of the current six-file scaffold and must pass its own admission and validation gate before being added.

## Explicitly Out of Scope

- Live-agent production authority
- Automatic repository mutation
- Commit, push, merge, or branch-management authority for model workers
- Ambient filesystem access
- Ambient network access
- Issue #174 semantics
- New Sovereign OS gates
- Capability expansion by inference
- Treating successful model output as admission or promotion

## Validation Posture

This scaffold is designed to reduce state contamination, bound context consumption, and prevent ambient authority escalation.

Those properties remain hypotheses subject to empirical validation.
