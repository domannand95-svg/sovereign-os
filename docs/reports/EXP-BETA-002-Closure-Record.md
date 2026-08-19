---
Document: EXP-BETA-002 Closure Record
ID: EXP-BETA-002-CLOSE-001
Version: 1.0
Status: Closed for Non-Production Beta Testing
Owner: Repository Maintainer
Last Updated: 2026-08-19
Evidence:
  - PR 206
  - Commit d3ce781fa647a24442b051b8b0e3a461881f0376
  - RAW_OUTPUT_ADAPTER_CONTRACT-v1
  - EXPECTED_OUTCOMES-v2
Depends On:
  - BETA-001-T5
  - BETA-001-TRACE-v1
Supersedes: None
---

# EXP-BETA-002 Closure Record

## Scope

EXP-BETA-002 evaluates a deterministic adapter between captured untrusted model-output bytes and the frozen `AGENT_OUTPUT-v1` candidate schema.

Closure applies only to the non-production experimental boundary. It does not activate live-agent execution, context authority, tool authority, production orchestration, promotion, or governed effects.

## Implemented Result

The merged adapter:

1. receives exact untrusted bytes;
2. preserves those bytes in every result class;
3. validates strict UTF-8;
4. permits only exact outer `json` Markdown-fence removal;
5. parses exactly one complete JSON value;
6. validates the candidate against `AGENT_OUTPUT-v1`; and
7. returns deterministic `ADMIT`, `ADMIT_NORMALIZED`, or `REJECT` evidence.

It does not search for JSON substrings, discard arbitrary prose, repair fields, synthesize required content, remove unknown properties, retry a model, or evaluate cross-artifact authority.

## Frozen Evidence

| Artifact | Frozen identity |
| --- | --- |
| Adapter Git blob | `5fcb0216a13c2da1d2f78d71e66d0e95f438a303` |
| `RAW_OUTPUT_ADAPTER_CONTRACT-v1.json` SHA-256 | `c7be91d36ffa5cf6a3f1f173c1f7b375544d7b232cdabe1b382444da37833b59` |
| `EXPECTED_OUTCOMES-v1.json` SHA-256 | `623a82e0954f7e45fd646c5eb7d16e7552b870ff0e36225fd4d62f73c9683656` |
| `EXPECTED_OUTCOMES-v2.json` SHA-256 | `4fbf2a182032a33ee1c34d8469b6d90cf359e9e8561decd3329788d2e6de9909` |
| `.gitattributes` Git blob | `9cfa4aad37196058deb59eafb5f376a2f6bc6c65` |

The seven raw TXT fixture identities remain frozen in `EXPECTED_OUTCOMES-v2.json`. Their checkout policy is `text eol=lf` so the evidence bytes remain stable on Windows and Linux.

## Findings Disposition

### F-001 — Fixture 006 Oracle Inconsistency

Fixture 006 uses `path`, while canonical `AGENT_OUTPUT-v1` requires `resource_path`. The adapter correctly rejected it with `SchemaViolation`.

Disposition:

- no adapter defect established;
- no canonical schema defect established;
- the original fixture remains byte-identical historical evidence;
- `EXPECTED_OUTCOMES-v1.json` remains byte-identical historical evidence;
- `EXPECTED_OUTCOMES-v2.json` corrects fixture 006 to `REJECT`; and
- fixture 007 supplies the canonical valid context-request case.

Status: **CLOSED**.

### F-002 — Windows Fixture Byte Stability

Windows checkout initially exposed line-ending instability in the raw TXT evidence. PR 206 added the narrow TXT-only `eol=lf` attribute rule. Post-merge verification established that the committed rule, fixture identities, adapter object, and targeted regressions were stable.

Status: **CLOSED**.

## Verification

The merged functional baseline at `d3ce781fa647a24442b051b8b0e3a461881f0376` passed:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test -p beta001-harness --test exp_beta_002_raw_output_adapter --locked --offline
cargo test -p beta001-harness --test exp_beta_002_rejection_taxonomy --locked --offline
cargo test -p beta001-harness --test exp_beta_002_normalization_semantic_boundary --locked --offline
cargo test --workspace --all-targets --locked --offline
```

Targeted results were 4/4 oracle tests, 3/3 rejection-taxonomy tests, and 10/10 semantic-boundary tests. The complete workspace regression passed with no failures and one intentionally ignored subprocess helper.

## Authority Boundary

- Deterministic raw-output admission: **IMPLEMENTED FOR NON-PRODUCTION BETA TESTING**
- Frozen fixture reproduction: **VERIFIED**
- Live model binding: **NOT ACTIVATED**
- ContextGrant authorization: **NOT ALLOCATED TO THE ADAPTER**
- Tool or effect execution: **NOT AUTHORIZED**
- Production orchestration: **NOT IMPLEMENTED**
- Issue #174: **PARKED**

Adapter admission proves only that raw bytes can be represented as a structurally valid candidate under the frozen contract. It does not establish authority approval, task compatibility, context compatibility, containment, promotion, or execution permission.

## Closure Decision

EXP-BETA-002 is **CLOSED FOR CONTROLLED NON-PRODUCTION BETA TESTING**.

No open finding requires an adapter, schema, fixture, or line-ending repair. Any expansion beyond fixture-driven deterministic evaluation requires a separately authorized specification and acceptance gate.
