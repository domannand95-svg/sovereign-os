# Legacy Extraction and Retirement Matrix

| Field | Value |
|---|---|
| Document ID | SOV-LEGACY-001 |
| Status | Active audit |
| Last updated | 2026-07-31 |
| Scope | Non-workspace prototype crates |

## Decision rules

- **Keep:** the concept and implementation fit the authoritative architecture.
- **Redesign:** preserve the behavioral contract, but do not copy the prototype
  implementation into production.
- **Archive:** retain concise historical or research documentation only.
- **Remove:** delete after all selected contracts and evidence have named homes.

No prototype is production authority merely because it compiles or has tests.
The canonical ledger, state machine, policy, registry, and engine remain the
source of truth.

## Crate-level disposition

| Prototype | Current role | Decision | Retirement condition |
|---|---|---|---|
| `active-memory` | JSON-lines event storage and in-memory view | Redesign | Projection/query requirements are owned by audit architecture and replay tests |
| `event-log` | Thin event API layered over active memory | Remove | Useful ordering and round-trip cases are represented against `sovereign-ledger` |
| `registry-service` | Mixed registry, orchestration, governance, persistence, and consensus experiment | Split and redesign | Every module below has a destination or explicit rejection |
| `governance-wrapper` | Node lifecycle, placement, scheduling, and metrics wrapper | Split and redesign | Scheduling and governance contracts are specified without duplicate authority |
| `network-service` | Raft and transport experiment | Archive and redesign later | Protocol/adversarial cases are captured for the distributed track |

## `active-memory`

| Capability | Decision | Destination |
|---|---|---|
| Append-only event history | Remove implementation | `sovereign-ledger` |
| In-memory query projection | Redesign | `sovereign-audit` projection |
| JSON-lines persistence | Remove | Canonical ledger format |
| Event identifiers and timestamps | Redesign | Provenance schema |
| Storage round-trip and malformed-record tests | Extract | Audit/provenance replay tests |

## `event-log`

| Capability | Decision | Destination |
|---|---|---|
| Event append API | Remove | `sovereign-ledger` append boundary |
| Event ordering | Keep contract | Ledger LSN invariants |
| History query | Redesign | Audit projection |
| Duplicate event model | Remove | Canonical ledger record/directive types |
| Round-trip test | Extract | Ledger/audit integration test |

## `registry-service`

| Module or capability | Decision | Destination |
|---|---|---|
| `registry` node/workload models | Redesign | Versioned directive and registry projection schemas |
| `snapshot` JSON format | Remove | Versioned `sovereign-ledger` snapshots |
| `persistence` engines | Remove | Canonical ledger and projections |
| `event_ledger` | Remove | `sovereign-ledger` |
| `event_replay` | Remove implementation | Engine restoration and audit projection |
| `allocation` | Keep contract | Future scheduler policy |
| `agent_task` lifecycle | Redesign | Capability-firewall artifact lifecycle |
| `agent_task_queue` | Redesign | Governed execution service |
| `agent_task_scheduler` | Redesign | Governed scheduler |
| `agent_task_executor` | Redesign | Capability-scoped sandbox executor |
| `agent_registry` | Redesign | Content-addressed registry relationships |
| `verification_engine` | Keep contract | Evaluation evidence and promotion gates |
| `governance_engine` | Redesign | Independently authorized promotion transitions |
| `runtime_orchestrator` | Redesign | Supported single-node service boundary |
| `consensus_engine` | Archive | Distributed consensus research |

Additional extraction requirements:

- Task and executor APIs must not imply ambient tool access.
- Task scheduling must consume an explicit, expiring resource grant.
- Verification results must preserve failed attempts and reviewer disagreement.
- Agent and model identity must include version and lineage.
- Prototype governance votes must not be copied as sufficient authority for
  tool grants, promotions, removals, or constitutional changes.

## `governance-wrapper`

| Capability | Decision | Destination |
|---|---|---|
| Node registration and lifecycle | Redesign | Typed directives and registry projection |
| Capacity metrics | Keep contract | Node status/observability schema |
| Resource placement | Keep deterministic cases | Future scheduler policy |
| Workload scheduling | Redesign | Governed scheduler |
| Workload completion/recovery | Keep behavioral cases | Lifecycle integration tests |
| Governance authority wrapper | Remove implementation | `sovereign-policy` plus independently authorized transitions |

The scheduler must never gain direct canonical persistence authority. It may
propose a transition; policy and the engine decide whether it becomes canonical.

Capacity and placement logic should become resource-policy inputs. They must not
become a general reputation score or allow an agent to allocate resources to
itself.

## `network-service`

| Capability | Decision | Destination |
|---|---|---|
| Peer discovery | Redesign later | `sovereign-discovery` |
| Heartbeats and peer liveness | Keep contract | Distributed transport specification |
| Request-vote messages | Archive | Consensus ADR evidence |
| Append-entries messages | Archive | Consensus ADR evidence |
| Log conflict handling | Keep adversarial cases | Deterministic network simulation |
| Commit-index rules | Keep contract | Consensus safety specification |
| Snapshot installation | Redesign | Version-aware replication protocol |
| Durable network log | Remove duplicate authority | Canonical ledger/replication adapter |
| State machine application | Remove duplicate implementation | `sovereign-core-asm` |
| Client command pipeline | Redesign | Authenticated external boundary |
| Message bounds, timeouts, and backpressure | Keep requirement | Distributed transport security |

No networking implementation may be promoted until peer identity,
authentication, replay protection, message limits, downgrade behavior, and
partition tests are defined.

## New architecture with no safe legacy implementation

The following requirements arise from the governed-agentic-research mission and
must be designed explicitly rather than inferred from prototype code:

| Requirement | Destination |
|---|---|
| Scoped and expiring trust assessments | Constitutional trust schema |
| Tool-operation and target grants | Capability authority |
| Data classification and disclosure grants | Capability authority |
| Compute, storage, network, token, money, and time budgets | Resource authority |
| Tool delegation restrictions | Capability authority |
| Correction, remediation, demotion, suspension, revocation, and removal | Trust enforcement |
| Independent incident investigation and appeal | Governance architecture |
| Research claims, citations, uncertainty, and failed attempts | Evidence schema |
| Reviewer independence and correlated-failure tracking | Adversarial review |
| Consent, privacy, licensing, and permitted-use records | Training-data governance |
| Dataset and model lineage | Governed learning |
| Poisoning, contamination, memorisation, and leakage controls | Governed learning |
| Hidden evaluation and red-team separation | Model promotion gate |

Prototype code must not be retained merely to fill these gaps quickly.

## Repository-level cleanup candidates

| Item | Proposed decision | Prerequisite |
|---|---|---|
| Root `src/main.rs` printing `Hello, world!` | Remove | Confirm the root is a virtual workspace only |
| Obsolete phase handovers | Archive or remove | Preserve any unique decisions in current ADRs/docs |
| Duplicate event-log architecture claims | Update | Canonical ledger terminology pass |
| Prototype-specific snapshot claims | Archive | Versioned ledger snapshot docs remain authoritative |
| Local `target/` directories | Never transfer or commit | Already covered by `.gitignore` |

## Extraction evidence required before deletion

Each prototype removal pull request must include:

1. a list of deleted modules;
2. the destination of every retained concept;
3. tests or specifications for retained safety behavior;
4. a search proving no authoritative crate imports the prototype;
5. a clean active-workspace verification; and
6. explicit owner approval.
