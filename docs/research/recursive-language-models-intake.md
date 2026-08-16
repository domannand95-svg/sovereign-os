---
Document: Recursive Language Models Comparative Intake
ID: RES-RLM-001
Version: 1.0
Status: Research
Owner: Repository Maintainer
Last Updated: 2026-08-16
Production Effect: None
---

# Recursive Language Models Comparative Intake

## Scope and evidence

RLM treats large context as data in a live programming environment. The model
receives instructions and a variable name, then writes code to inspect,
decompose, and recursively query over the underlying value.

Source was inspected at commit
`caf0bffa1acec17c062559433b4cd4ed92eee3d6`, including the runtime,
environment, recursion, persistence, compaction, logging, custom-tool, and
training documentation and implementation.

Primary sources:

- <https://github.com/alexzhang13/rlm>
- <https://arxiv.org/abs/2512.24601>

## Findings

### ADAPT — programmable, reference-oriented context access

Moving a large artifact out of the repeated prompt and exposing it through a
durable name can reduce context pressure and let the model perform targeted
inspection. Recursive subqueries receive bounded tasks in fresh namespaces,
while trajectories can record iterations, code, outputs, and subcalls.

For SOL and BKI, the useful adaptation is a governed archive query surface:
models receive stable artifact identifiers and bounded read/query operations,
while the archive retains provenance and exact source bytes. This complements
hybrid attention and cache-stable prompting; it does not replace either.

### COMPARE — persistence, compaction, and recursion

Persistent mode retains versioned context and history variables. Optional
compaction summarizes model history. Child RLMs have separate namespaces and
receive remaining time/token budgets, with recursion falling back to a plain
model call at the depth limit. These are useful mechanics for resource
accounting and subtask isolation, but the context variable, generated code,
subquery prompts, summaries, and outputs must all be reconstructable if used
in a governed decision.

### REJECT — local execution as a security boundary

The default `LocalREPL` executes model-generated Python with `exec()` in the
host interpreter. Removing selected builtins is explicitly described as a
soft sandbox, not a security boundary. SOL must reject this path for untrusted
content or any operation with external consequence. Container and remote
environments improve isolation but still require explicit capability and
network/filesystem policy.

### REJECT — custom-tool availability as authority

RLM custom callables are placed directly in the execution namespace; children
inherit them unless a separate child tool set is supplied. Registration answers
what code is callable, not what the subject is permitted to do. Consequential
tools require independent identity, scope, budget, policy, and authorization
checks at invocation time. Prompt injection in the offloaded document remains
an input-integrity risk even when the document is not initially pasted into
the model window.

### DEFER — training and production integration

The training harness connects the local REPL policy to `prime-rl`. Training a
better recursive policy does not solve authority, provenance, sandboxing, or
evaluation independence. Benchmark claims and production integration remain
deferred pending source-level benchmark forensics and a SOL-governed archive
query contract.

## Classification summary

| Classification | Intake |
|---|---|
| ADAPT | Stable artifact references, bounded archive queries, logged recursive decomposition, inherited resource ceilings |
| COMPARE | Persistent namespaces, trajectory logging, compaction, child isolation, depth limits |
| REJECT | Same-process soft sandbox for untrusted work; inherited callable tools as implicit permission; summaries as authoritative evidence |
| DEFER | RLM training, benchmark conclusions, and production dependency adoption |

## SOL boundary

RLM is an epistemic and context-management technique. Its outputs may support
evaluation, but neither successful retrieval nor recursive reasoning can admit
a record, expand policy, or permit an external effect.
