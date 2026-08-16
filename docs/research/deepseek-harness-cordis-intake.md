---
Document: DeepSeek Harness and Cordis Comparative Intake
ID: RES-HARNESS-001
Version: 1.0
Status: Research
Owner: Repository Maintainer
Last Updated: 2026-08-16
Production Effect: None
---

# DeepSeek Harness and Cordis Comparative Intake

## Research question

Does an append-only, reconstructable agent harness provide a useful
implementation substrate for SOL's stronger requirement that every
consequential authority transition be explicit, reconstructable, and
independently auditable?

## Evidence inspected

- DeepSeek Harness source at commit
  `47f943859bef60e4160492346772ded9b24f765a`.
- Harness architecture, lifecycle, capability-seam, tool-pipeline, session
  persistence, sandbox-policy, subagent, checkpoint, and invariant material.
- Cordis source and its reversible plugin-effect model.
- Official DeepSeek API pricing documentation. Prices are time- and
  model-specific; no cache multiplier is treated as a permanent constant.

Primary sources:

- <https://github.com/deepseek-ai/DeepSeek-Harness>
- <https://github.com/cordiverse/cordis>
- <https://api-docs.deepseek.com/quick_start/pricing>

## Findings

### ADAPT — event-derived state and reconstruction discipline

The append-only session log is the source for model history, resume, fork,
transcript, telemetry, and replay. Model-visible inputs must be logged, and
runtime invariants check relations between durable facts and derived views.
The JSONL provider preserves contiguous sequence numbers, rejects corrupt
committed history, retains exact events through packing, and makes interrupted
effects explicit as unknown rather than inventing success or failure.

SOL should adapt the discipline: durable facts first, projections second,
invariants over both, and explicit uncertainty after an interrupted
consequential operation. Harness reconstruction is useful substrate, but it is
not itself authorization.

### ADAPT — stable references, compaction, and cache-aware envelopes

Durable attachments use content-addressed opaque identifiers; logs retain the
reference rather than unstable host paths or bearer URLs. Checkpoints and
projection caches accelerate reconstruction without replacing the log.
Package documentation also treats model tokens and KV-cache effects as design
properties. Stable tool catalogs and append-oriented requests can preserve
provider prefix-cache reuse.

SOL should adapt durable identifiers, event-derived compaction, and explicit
cache-impact review. Cache economics remain an optimization constraint. A
reported 120-fold input-price difference is valid only for the particular
cache-hit/miss price pair and date that produced it, not as a universal ratio.

### COMPARE — plugin composability is not governed capability

Cordis answers **what can be composed**: plugins register services, events,
tools, policies, storage, sandboxes, and reversible effects in a shared
context. SOL must separately answer **what is permitted to exercise
consequence** for a subject, object, operation, policy, time, and budget.

Reversible registration improves lifecycle hygiene but does not undo an
external effect already exercised. A plugin that contributes a tool,
filesystem provider, subprocess provider, approval listener, policy row, or
agent preset can alter the reachable consequence surface. Therefore plugin
installation, enablement, configuration overlay, provider substitution, and
per-session composition are potential authority-escalation paths.

### REJECT — authority by registration or composition

SOL must not infer permission from successful plugin loading, service
availability, tool-schema visibility, provider compatibility, or possession
of a reversible disposer. Any production plugin plane would need an explicit
authority gate before a consequential provider can be selected or invoked,
plus durable records of the governing policy, resolved identity, capability,
decision, and result.

### COMPARE — isolation and scheduling

The harness distinguishes host and per-agent/session realms, sandbox policy,
approval services, in-process and out-of-process subagents, jobs, sessions,
storage, and checkpoints. These are useful separations. They do not establish
that a child, scheduled job, restored session, or substituted provider retains
only authorized capabilities. SOL must reconstruct and re-evaluate authority
at each such boundary.

## Classification summary

| Classification | Intake |
|---|---|
| ADAPT | Append-only facts, deterministic projections, runtime invariants, durable opaque references, explicit unknown outcomes, cache-aware stable envelopes |
| COMPARE | Plugin/service seams, reversible effects, subagent realms, scheduling, session and storage abstractions |
| REJECT | Treating composition, registration, tool visibility, or reversibility as permission |
| DEFER | Any Cordis or DeepSeek Harness integration until SOL authority-transition records and conformance tests exist |

## Disposition

The central question is answered **yes, conditionally**. An append-only,
reconstructable harness is useful substrate for auditability, but SOL's
authority boundary must sit above and independently constrain composition and
effect execution. No production dependency or roadmap change is proposed.
