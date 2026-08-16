---
Document: Exo Recursive Self-Improvement Comparative Intake
ID: RES-RSI-001
Version: 1.0
Status: Research
Owner: Repository Maintainer
Last Updated: 2026-08-16
Production Effect: None
---

# Exo Recursive Self-Improvement Comparative Intake

## Scope and evidence

This intake examines recursive self-modification, protected history, rollback,
and lineage without proposing that SOL become a self-modifying harness.

Exo source was inspected at commit
`5bc77ce7c7a2921794083d58c926cf721c14bf8a`, including its RSI,
self-control, lifecycle, time-travel, tool, sandbox, secret, scheduler, and
executor/exoharness design records.

Primary source: <https://github.com/exoharness/exo>

## Findings

### COMPARE — protected canonical history below mutable policy

Exo separates a durable exoharness from a swappable executor. The exoharness
owns identity, append-only conversation events, artifacts, secrets, and
sandbox lifecycle; the executor owns prompts, model calls, tool dispatch,
memory compaction, and approvals. Sandbox rewind does not erase the canonical
event history. Forks and clones retain lineage.

This strongly resembles SOL's need to keep audit state outside the component
being evaluated or changed. The useful principle is not unrestricted
self-modification; it is that mutable intelligence cannot rewrite the record
used to judge its behavior.

### ADAPT — change records, lineage, verification, and rollback

SOL should adapt explicit proposal records for changes, immutable parentage,
pre-adoption verification evidence, activation decisions, observed results,
and rollback records. Code history, runtime events, and sandbox snapshots
serve different purposes and should remain linked rather than conflated.

### REJECT — executor approval as sufficient authority

Exo describes approvals as executor policy, while the standard configuration
protects the exoharness by policy rather than technical impossibility. That is
too weak for SOL's authority boundary. An agent's ability to edit its policy,
tool registry, scheduler, adapter, prompt, or executor must never allow that
same mutable layer to authorize activation.

### GAP IN COMPARATIVE SYSTEM — trusted tool registration

Exo's own tool documentation states that TypeScript tool modules run as
trusted harness-process code and that there is not yet a capability sandbox
around the tool module. Installation is therefore a direct trust and
consequence transition. Pinning source, validating schemas, and recording
tool events improve integrity and observability but do not constitute governed
authorization.

### DEFER — recursive clone management

Clone/fork lineage, stopping conditions, capability attenuation, secret
inheritance, budget inheritance, and independent evaluation require explicit
governance contracts before experimentation in SOL. No autonomous
self-rewrite, rebuild, restart, adapter installation, or scheduled activation
belongs in the current roadmap slice.

## Classification summary

| Classification | Intake |
|---|---|
| ADAPT | Immutable change lineage, protected event history, separately linked snapshots, verification and rollback records |
| COMPARE | Durable exoharness versus mutable executor; fork, rewind, scheduler, adapter, and secret boundaries |
| REJECT | Self-authorization by a mutable executor; trusted tool loading as implicit permission; rollback as a substitute for prevention |
| DEFER | Recursive self-modification, clone fleets, inherited secrets/capabilities, and automated activation |

## SOL boundary

For any future self-change: intelligence may propose a change and epistemic
processes may evaluate it, but only an independent authority decision may
permit activation. A successful build, benchmark, test, or rollback plan does
not implicitly grant that permission.
