# Sovereign OS

> Provider-neutral infrastructure for governed agentic research, reproducible
> evidence, bounded authority, and progressively earned trust.

Sovereign OS is an experimental Rust platform for coordinating AI agents under
explicit governance. Its objective is not to declare agents trustworthy. It is
to make their authority, evidence, tool use, resource consumption, review
history, and promotion or demotion decisions inspectable and reproducible.

The long-term platform is intended for institutions that want to use capable
external models while retaining local control of policy, data boundaries,
research evidence, agent authority, and institution-developed models.

> [!IMPORTANT]
> The governed research record layer is now partially implemented: the A04
> common envelope plus canonical Objective and Claim payloads are merged. Tool
> authority, governed admission, adversarial review, enforcement, and learning
> remain planned architecture rather than production features.

## Mission

Sovereign OS is designed around four pillars:

1. **Governed agentic research** — claims move through declared objectives,
   evidence requirements, independent challenge, and recorded decisions.
2. **Capability, tool, and resource authority** — agents receive only explicitly
   granted operations, targets, data, network access, and budgets.
3. **Evidence-based trust and adversarial scrutiny** — trust is scoped, earned
   through observable conduct, independently reviewed, and revocable.
4. **Governed local model development** — authorised and verified evidence may
   support institution-controlled models without making training equivalent to
   trust.

The complete proposed authority model is documented in
[Governed Agentic Research Architecture](docs/architecture/governed-agentic-research.md).

## Core principles

- Proposers cannot approve themselves.
- Agents cannot grant themselves tools, resources, trust, or promotion.
- Confidence and model reputation are not substitutes for evidence.
- Important claims preserve sources, methods, uncertainty, failed attempts,
  reviewer findings, and reproducibility results.
- Tool, data, network, and resource access is default-deny, narrow, expiring,
  monitored, and revocable.
- Ordinary mistakes support correction and remediation.
- Persistent cheating, evidence manipulation, or serious boundary violations
  support restriction, suspension, revocation, and removal.
- Local models and commercial models pass through the same governed trust
  process.
- Human constitutional authority remains available at high-impact boundaries.
- External models may be nondeterministic; the platform must record enough
  context to describe honestly what was replayed or reproduced.

## Implemented baseline

The authoritative workspace currently provides:

- append-only ledger records with checksums and strict sequence ordering;
- deterministic replay and state reconstruction;
- crash-atomic, explicitly versioned snapshots;
- snapshot integrity, state-root validation, and legacy-format recovery;
- safe fallback from invalid or unsupported snapshots to ledger replay;
- content-addressed registry nodes and deterministic ledger projection;
- deterministic, fail-closed directive admission;
- single-node boot, directive submission, and restart reconstruction;
- fail-closed handling of ambiguous persistence outcomes; and
- corruption and interrupted-publication tests;
- the canonical A04 governed-evidence envelope;
- bounded, deterministic Objective and Claim payload encodings with fixed
  vectors and kind-confusion protection; and
- a pinned, read-only `bki.validation.v1` compatibility boundary with the
  Knowledge Infrastructure Bootstrap Kit.

This baseline supplies canonical state, persistence, recovery, and policy
boundaries. Governed evidence encoding is in progress. Record admission, agent
trust, tool grants, adversarial review, enforcement, governed learning, and
institutional interfaces remain staged work.

## Current architecture

```text
Directive
    |
    v
Deterministic policy
    |
    v
Append-only canonical ledger
    |
    +------------------+
    |                  |
    v                  v
Authoritative state    Registry projection
    |                  |
    +---------+--------+
              |
              v
      Single-node engine
```

Snapshots accelerate restoration. They are validated caches; the ledger remains
canonical.

## Target governed research flow

```text
Research objective
    -> capability-scoped agent work
    -> recorded sources, methods, tools, budgets, and uncertainty
    -> specialized independent and adversarial review
    -> verified | disputed | rejected
    -> independently authorized promotion, remediation, or revocation
    -> reproducible evidence and audit history
```

Verified evidence may later enter a consented, licensed, versioned training
corpus. A resulting local model re-enters as an untrusted candidate and must
earn bounded authority through evaluation.

## Workspace boundary

The root `Cargo.toml` defines the authoritative seven-crate workspace:

```text
crates/
|-- sovereign-core-asm/   deterministic state and snapshot representation
|-- sovereign-ledger/     append, replay, restore, integrity, and snapshots
|-- sovereign-registry/   content-addressed derived registry graph
|-- sovereign-policy/     deterministic directive admission
|-- sovereign-engine/     single-node boot and command orchestration
|-- sovereign-audit/      governed-evidence envelope, Objective, and Claim
|-- sovereign-discovery/  scaffold; not implemented
`-- service prototypes/   excluded; classified for extraction or retirement
```

The older `active-memory`, `event-log`, `registry-service`,
`governance-wrapper`, and `network-service` crates are not active workspace
members. They are preserved temporarily while useful contracts and adversarial
cases are extracted. They must not be treated as production authority.

See [Project Status](PROJECT_STATUS.md) and the
[Legacy Extraction Matrix](docs/roadmap/LEGACY-EXTRACTION-MATRIX.md).

## Verification

The repository pins its Rust toolchain. Run:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

GitHub Actions runs the active-workspace checks on Linux and Windows. Security
CI also enforces decoder fuzz smoke testing, at least 90% workspace line
coverage, Rust dependency policy, filesystem scanning, and CycloneDX SBOM
generation. Boundary-affecting changes additionally run the read-only BKI
compatibility suite on both platforms.

## Delivery plan

Work advances through small, independently reviewable pull requests. The
project owner approves each merge; approving a plan does not pre-authorize
later merges.

- [Execution Plan](docs/roadmap/EXECUTION-PLAN.md)
- [Delivery Backlog](docs/roadmap/DELIVERY-BACKLOG.md)
- [Legacy Extraction Matrix](docs/roadmap/LEGACY-EXTRACTION-MATRIX.md)
- [Development and Review Workflow](docs/governance/DEVELOPMENT-WORKFLOW.md)

The immediate direction is:

1. implement the remaining A04 payloads in dependency order, beginning with
   Source;
2. add governed admission and cross-record lifecycle validation;
3. implement the minimal read-only Capability Firewall path;
4. define the Untrusted Model Harness threat model and governed memory flow;
5. run joint BKI-Sovereign OS compatibility and adversarial beta testing;
6. harden privacy, recovery, key management, licensing, and release operations;
   and
7. defer distributed consensus until single-node authority is dependable.

## Institutional design boundary

Sovereign OS aims to provide open, exportable technical evidence and locally
controlled governance. It does not by itself certify legal compliance, research
truth, model safety, or regulatory approval.

## Contributing

Keep changes focused, distinguish implemented behavior from proposals, and
include the verification performed. See
[CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under the [Apache License 2.0](LICENSE).

